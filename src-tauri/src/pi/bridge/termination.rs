//! 任务终态与退出收尾。调用方持有会话槽锁并完成 generation/任务身份核对。
//! 进程与锁仍是 bridge 私有状态；本模块不对其他能力公开过程状态。
use super::*;

/// 派生差异异步刷新，取得租约后再核对代次，旧任务不会覆盖新连接。
pub(super) fn refresh_changes(app: &tauri::AppHandle, process: &Process, generation: u64, settled: bool) -> Result<(),String> {
    let Some(task) = process.task.clone() else { return Ok(()); };
    let slot = app.state::<PiState>().0.slot(&process.mod_id,&process.session_id)?;
    let app = app.clone();
    let channel = process.channel.clone();
    let title = process.pending_title.clone();
    // 租约可能等待其他会话写入，等待期间不能占用本会话的停止和引导入口。
    std::thread::spawn(move || {
        let context = &task.context;
        let workspaces = app.state::<crate::pi::workspace::Workspaces>();
        let id = uuid::Uuid::new_v4().to_string();
        let result = (|| -> Result<(),String> { loop {
            workspaces.acquire(&app,context,&json!({"leaseId":id,"toolCallId":"native-refresh","operation":"snapshot"}))?;
            // 新任务准备也可能持槽锁等待租约，不能持有读租约反向等待槽锁。
            let inner = match slot.try_lock() {
                Ok(inner) => inner,
                Err(std::sync::TryLockError::WouldBlock) => {
                    workspaces.release(&app,context,&json!({"leaseId":id,"toolCallId":"native-refresh","success":true}))?;
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                Err(_) => {
                    let _ = workspaces.release(&app,context,&json!({"leaseId":id,"toolCallId":"native-refresh","success":false}));
                    return Err("会话运行锁异常".into());
                }
            };
            let refreshed = (|| -> Result<(),String> {
                if inner.generation != generation || inner.process.as_ref().is_some_and(|p|p.task.as_ref().is_none_or(|t|t.context.task_id!=context.task_id)) { return Ok(()); }
                crate::changes::refresh(&app,context)?;
                if settled {
                    let status = if matches!(task.status.as_str(),"failed"|"cancelled"|"interrupted") { DevelopmentStatus::Failed }
                        else if task.registered_version_id.is_some() { DevelopmentStatus::Ready }
                        else if crate::changes::list_task_changes(app.clone(),context.mod_id.clone(),context.task_id.clone())?.is_empty() { DevelopmentStatus::Draft }
                        else { DevelopmentStatus::Generated };
                    let title=if task.status=="finished" {title.clone().map(|title|(context.session_id.clone(),title))} else {None};
                    session_repository::edit(&app,&context.mod_id,SessionChange::Runtime {status,title,touch:true})?;
                }
                let _ = channel.send(Envelope {sequence:0,subscription:0,mod_id:context.mod_id.clone(),session_id:context.session_id.clone(),task_id:Some(context.task_id.clone()),generation,kind:"task".into(),record:json!({"type":"changes_updated","taskId":context.task_id})});
                Ok(())
            })();
            drop(inner);
            let released = workspaces.release(&app,context,&json!({"leaseId":id,"toolCallId":"native-refresh","success":refreshed.is_ok()}));
            return refreshed.and(released.map(|_|()));
        } })();
        if let Err(error) = result {
            let _ = channel.send(Envelope {sequence:0,subscription:0,mod_id:context.mod_id.clone(),session_id:context.session_id.clone(),task_id:Some(context.task_id.clone()),generation,kind:"diagnostic".into(),record:json!({"type":"bridge_error","message":error})});
        }
    });
    Ok(())
}

pub(super) fn stop(app: &tauri::AppHandle, inner: &mut Inner) -> Result<(), String> {
    stop_inner(app,inner,false)
}
/// 意外关闭 stdout 与用户停止分别记录，不能把进程崩溃显示为手动取消。
pub(super) fn stop_inner(app: &tauri::AppHandle, inner: &mut Inner, unexpected: bool) -> Result<(), String> {
    if let Some(process) = inner.process.as_ref() {
        for token in process.tool_cancellations.values(){token.store(true,std::sync::atomic::Ordering::SeqCst);}
        process.transport.terminate_and_wait()?;
    }
    if let Some(mut process) = inner.process.take() {
        process.transport.reap(); // 先回收命令后代，再保存最终源码差异。
        if let Some(task) = &process.task { app.state::<crate::pi::workspace::Workspaces>().cleanup(&process.mod_id,&task.context.task_id,&process.waiting_leases)?; }
        let saved = (|| -> Result<(), String> { if let Some(task) = process.task.as_mut() {
            if task.status == "running" || task.status == "preparing" {
                task.status = if unexpected {"interrupted"} else {"cancelled"}.into();
                persist_task(app, task)?;
                session_repository::edit(app,&task.context.mod_id,SessionChange::Runtime {status:DevelopmentStatus::Failed,title:None,touch:true})?;
            }
        } Ok(()) })();
        refresh_changes(app, &process, inner.generation, false)?;
        for input in &mut process.steering {
            if matches!(input["status"].as_str(),Some("submitted"|"queued")) { input["status"]="unknown".into(); crate::pi::session_repository::steering(app,input)?; }
        }
        emit(&process, inner.generation, "exit", json!({ "type": "exit", "reason": if unexpected {"unexpected"} else {"closed"} }));
        saved?;
    }
    Ok(())
}

/// 模型报错与 prompt 拒绝沿用失败终态；后续 settled 不覆盖失败。
pub(super) fn fail(app: &tauri::AppHandle, process: &mut Process) -> Result<(), String> {
    if let Some(task) = process.task.as_mut() { task.status = "failed".into(); persist_task(app, task)?; }
    Ok(())
}

pub(super) fn settle(app: &tauri::AppHandle, process: &mut Process, generation: u64, aborted: bool) -> Result<(), String> {
    process.active = false;
    process.stopping = false;
    if let Some(task) = process.task.as_mut() {
        if task.status != "failed" { task.status = if aborted { "cancelled" } else { "finished" }.into(); }
        persist_task(app, task)?;
    }
    refresh_changes(app, process, generation, true)
}

pub(super) fn handled(app: &tauri::AppHandle, process: &mut Process) -> Result<(), String> {
    // handled 不证明扩展没有另行启动任务，不强行清除 active。
    if !process.active {
        if let Some(task) = process.task.as_mut() { task.status = "finished".into(); persist_task(app, task)?; }
    }
    Ok(())
}

/// 准备流程持槽锁调用：先尝试回收，再按准备结果覆盖 stop 的取消终态。
/// 未启动进程的失败仍由 preparation 保存，不在此重复回收快照租约。
pub(super) fn preparation_failed(app: &tauri::AppHandle, inner: &mut Inner, task: &mut TaskRecord, cancelled: &std::sync::atomic::AtomicBool, error: &str) -> Result<(), String> {
    let cleanup = stop(app, inner);
    task.status = if cancelled.load(std::sync::atomic::Ordering::SeqCst) { "cancelled" } else { "failed" }.into();
    persist_task(app, task)?;
    if let Err(cleanup) = cleanup { return Err(format!("{error}；进程回收失败：{cleanup}")); }
    Ok(())
}
