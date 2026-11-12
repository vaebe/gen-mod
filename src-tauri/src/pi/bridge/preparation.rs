//! 任务准备应用流程：模型/历史/Skills、持久快照、bootstrap 与首次需求发送。
//! 作为 bridge 的私有子模块使用原有槽位，不向其他模块公开锁或运行时可写状态。
use super::*;

pub(super) struct CommandPlan {
    pub(super) command: std::process::Command,
    pub(super) configuration: tempfile::TempDir,
    pub(super) selected_model: crate::config::ModelDefinition,
    pub(super) secrets: Vec<String>,
    pub(super) source: std::path::PathBuf,
}

pub(super) fn command(app: &tauri::AppHandle, mod_id: &str, session_id: &str, generation: u64, task: &mut Option<TaskRecord>) -> Result<CommandPlan, String> {
    let directory = paths::project_dir(app, mod_id)?;
    let source = directory.join("source");
    paths::reject_redirect(&source)?;
    let sessions = directory.join("logs/sessions");
    fs::create_dir_all(&sessions).map_err(|e| e.to_string())?;
    paths::reject_redirect(&sessions)?;
    let config = crate::config::read_session(app, mod_id, session_id)?;
    let selected_model = config.model_definition()?;
    let configuration = config.prepare()?;
    let mut command = runtime::command(app, &source, &sessions, configuration.path())?;
    let project = mods::read_project(app, mod_id)?;
    // 已记录的历史文件优先于按 ID 发现，避免发现失败时静默新建同名会话。
    let history = project.sessions.iter().find(|s| s.session_id == session_id).ok_or("会话不属于当前项目")?;
    if !history.session_file.is_empty() && std::path::Path::new(&history.session_file).exists() {
        let file = std::path::Path::new(&history.session_file);
        paths::reject_redirect(file)?;
        let actual = file.canonicalize().map_err(|e| e.to_string())?;
        if actual.parent() != Some(sessions.canonicalize().map_err(|e| e.to_string())?.as_path()) { return Err("会话历史文件位于项目之外".into()); }
        let mut header = String::new();
        BufReader::new(fs::File::open(&actual).map_err(|e| e.to_string())?).read_line(&mut header).map_err(|e| e.to_string())?;
        let header: Value = serde_json::from_str(&header).map_err(|_| "会话历史头损坏，不能开始新任务")?;
        if header["id"] != session_id { return Err("会话历史 ID 不匹配".into()); }
        command.arg("--session").arg(actual);
    } else {
        // 两种会话选择参数互斥；没有历史文件时才按 ID 创建或查找。
        command.args(["--session-id", session_id]);
    }
    for view in crate::skills::registry::resolve_for_game(app, &project.game_id)? {
        command.arg("--skill").arg(std::path::Path::new(&view.definition.path).join("SKILL.md"));
    }
    command.args(["--provider", &config.provider, "--model", &config.model])
        .env("GEN_MOD_PROVIDER_API_KEY", &config.key)
        .env("GEN_MOD_REDACTION_KEYS", serde_json::to_string(&config.secrets).map_err(|_| "无法准备凭据脱敏")?);
    if let Some(value) = task.as_mut() {
        value.context.generation = generation;
        command.env("GEN_MOD_TASK_CONTEXT", serde_json::to_string(&value.context).map_err(|e| e.to_string())?);
        command.arg("--extension").arg(runtime::paths(&runtime::resources(app)?)?.extension)
            .args(["--tools", "read,edit,write,grep,find,ls,run_command,prepare_delivery,read_project_context,read_project_memory_sources,update_project_memory,read_feedback_sources,propose_verification,search_game_knowledge,save_long_term_memory,read_game_inspection"]);
    } else {
        command.env_remove("GEN_MOD_TASK_CONTEXT").args(["--no-tools"]);
    }
    Ok(CommandPlan { command, configuration, selected_model, secrets: config.secrets, source })
}

fn bootstrap_ready(receiver:&std::sync::mpsc::Receiver<Result<MemoryBootstrapAck,String>>,task:&TaskContext,timeout:std::time::Duration,cancelled:&std::sync::atomic::AtomicBool)->Result<(),String>{
    if cancelled.load(std::sync::atomic::Ordering::SeqCst){return Err("项目记忆加载已取消".into());}
    let deadline=std::time::Instant::now()+timeout;
    let ack=loop {
        if cancelled.load(std::sync::atomic::Ordering::SeqCst){return Err("项目记忆加载已取消".into());}
        let remaining=deadline.saturating_duration_since(std::time::Instant::now());
        match receiver.recv_timeout(remaining.min(std::time::Duration::from_millis(50))) {
            Ok(value)=>break value?,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) if std::time::Instant::now()<deadline=>continue,
            Err(_)=>return Err("项目记忆加载未确认，未发送需求".into()),
        }
    };
    if cancelled.load(std::sync::atomic::Ordering::SeqCst){return Err("项目记忆加载已取消".into());}
    match ack {
        MemoryBootstrapAck::Ready{mod_id,session_id,task_id,generation,snapshot_id,sha256} if mod_id==task.mod_id && session_id==task.session_id && task_id==task.task_id && generation==task.generation && task.context_snapshot_id==snapshot_id && task.memory_bootstrap_sha256.as_deref()==Some(sha256.as_str())=>Ok(()),
        MemoryBootstrapAck::Error{mod_id,session_id,task_id,generation,error} if mod_id==task.mod_id && session_id==task.session_id && task_id==task.task_id && generation==task.generation=>Err(format!("项目记忆加载失败：{error}")),
        _=>Err("项目记忆加载回执身份不一致，未发送需求".into()),
    }
}

pub(super) fn start_mod_task_blocking(app: tauri::AppHandle, state: tauri::State<PiState>, storage: tauri::State<StorageState>, mod_id: String, session_id: String, requirement: String, thinking_level: String, images: Option<Vec<Value>>, request_id: String, on_event: Channel<Envelope>) -> Result<TaskContext, String> {
    let images = images.unwrap_or_default();
    validate_images(&images)?;
    // 保持锁顺序：会话槽位 → 原生工作区预留 → 存储锁；快照租约前释放后两者。
    let slot = SlotState(state.0.slot(&mod_id,&session_id)?);
    let mut inner = slot.0.lock().map_err(|_| "运行时锁异常")?;
    if inner.process.as_ref().is_some_and(|p| p.active || p.compacting || p.task.as_ref().is_some_and(|t| t.status == "running")) { return Err("已有任务正在运行，请先取消或等待结束".into()) }
    let reservation = app.state::<crate::pi::workspace::Workspaces>().try_reserve_native(&mod_id, crate::pi::workspace::NativeOperation::TaskPrepare)?;
    let _storage = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    mods::read_project(&app, &mod_id)?;
    if requirement.trim().is_empty() { return Err("需求不能为空".into()) }
    if !matches!(thinking_level.as_str(), "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max") { return Err("无效的思考强度".into()); }
    let id = session(&app, &mod_id, Some(&session_id))?;
    let config = crate::config::read_session(&app, &mod_id, &id).map_err(|error| format!("需求已保存；{error}"))?;
    let model = config.model_definition()?;
    if !model.thinking_levels().contains(&thinking_level) { return Err("模型不支持此思考强度，请重新选择".into()); }
    if !images.is_empty() && !model.input.iter().any(|input| input == "image") { return Err("当前模型不支持图片，请移除图片附件或切换模型".into()); }
    session_repository::save_model(&app, &mod_id, &id, Some(&crate::config::ModelSelection { provider_id: config.provider, model_id: config.model }))?;
    let project = mods::read_project(&app, &mod_id)?;
    let directory = paths::project_dir(&app, &mod_id)?;
    let task_id = uuid::Uuid::new_v4().to_string();
    let staging_id = uuid::Uuid::new_v4().to_string();
    let task_directory = directory.join("logs").join(&task_id);
    let task_skills = crate::skills::registry::resolve_for_game(&app, &project.game_id)?;
    let staging = task_directory.join("staging").join(&staging_id);
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    crate::skills::registry::pin_task_on(&task_directory,&task_skills)?;
    let mut context = TaskContext { task_id, mod_id: mod_id.clone(), session_id: id.clone(), generation: inner.generation + 1, source_directory: directory.join("source").to_string_lossy().into_owned(), staging_id, staging_directory: staging.to_string_lossy().into_owned(), created_at: mods::now()?, context_snapshot_id: uuid::Uuid::new_v4().to_string(), memory_bootstrap_path: None, memory_bootstrap_sha256: None };
    // preparing、不可变记忆快照和提交绑定必须同事务提交。
    let (snapshot,mut task)={
        let mut db=crate::storage::database::connect(&app)?;
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        crate::project_lifecycle::repository::ensure_writable_on(&tx,&mod_id)?;
        let snapshot=crate::project_context::repository::snapshot_on(&tx,&mod_id,&context.task_id,&context.context_snapshot_id,&context.created_at)?;
        let task=TaskRecord{context:context.clone(),status:"preparing".into(),finished_at:None,commands:vec![],registered_version_id:None};
        crate::pi::tasks::task_on(&tx,&task)?;
        crate::pi::submissions::bind_on(&tx,&request_id,&task.context.task_id)?;
        tx.commit().map_err(|e|e.to_string())?;
        (snapshot,task)
    };
    let prepared=(||->Result<(),String>{
        let path=crate::project_context::bootstrap::write_bootstrap(&app,&context,&snapshot)?;
        context.memory_bootstrap_sha256=Some(crate::changes::file_hash(&path)?);
        context.memory_bootstrap_path=Some(path.to_string_lossy().into_owned());
        task.context=context.clone();
        persist_task(&app,&mut task)
    })();
    if let Err(error)=prepared {task.status="failed".into();persist_task(&app,&mut task)?;return Err(error);}
    let exclusions: Vec<String> = task_skills.iter().flat_map(|v| v.definition.exclude_paths.clone()).collect();
    drop(_storage);
    drop(reservation);
    let lease_id = uuid::Uuid::new_v4().to_string();
    let workspace_request = json!({"leaseId":lease_id,"toolCallId":"native-snapshot","operation":"snapshot"});
    let workspaces = app.state::<crate::pi::workspace::Workspaces>();
    let snapshot_result = workspaces.acquire(&app,&context,&workspace_request).and_then(|_| {
        let result = crate::changes::begin(&app,&context,&exclusions);
        let released = workspaces.release(&app,&context,&json!({"leaseId":lease_id,"toolCallId":"native-snapshot","success":result.is_ok()}));
        result.and(released.map(|_|()))
    });
    if let Err(error) = snapshot_result {
        task.status = "failed".into();
        persist_task(&app, &mut task)?;
        return Err(error);
    }
    if let Err(error) = spawn(&app, &slot, &mut inner, &mod_id, &id, on_event, Some(task.clone())) {
        task.status = "failed".into();
        persist_task(&app, &mut task)?;
        return Err(error);
    }
    task.context.generation=inner.generation;
    let preparation_cancel=inner.process.as_ref().ok_or("pi 进程未建立")?.preparation_cancel.clone();
    // spawn 仅接通进程；bootstrap 和取消门禁完成之前不交付玩家需求。
    let started = (|| -> Result<TaskContext,String> {
    let process = inner.process.as_mut().ok_or("pi 进程未建立")?;
    let receiver=process.bootstrap.take().ok_or("缺少项目记忆加载通道")?;
    let cancelled=preparation_cancel.clone();
    let _preparing=process.preparation_guard.take().ok_or("缺少任务准备取消注册")?;
    bootstrap_ready(&receiver,&task.context,std::time::Duration::from_secs(10),&cancelled)?;
    process.prompt_id = Some(request_id.clone());
    process.pending_title = project.sessions.iter().find(|s| s.session_id == id && s.name.is_empty())
        .map(|_| requirement.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(40).collect());
    if let Some(task) = process.task.as_mut() { task.status = "running".into(); persist_task(&app, task)?; }
    // task.status 已占用任务槽；active 只表示实际 Agent 事件状态。
    process.active = false;
    process.transport.write_record(&json!({ "type": "set_auto_compaction", "enabled": true, "id": format!("native-auto-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
    process.transport.write_record(&json!({ "type": "set_thinking_level", "level": thinking_level, "id": format!("native-thinking-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
    process.transport.write_record(&json!({ "type": "get_available_thinking_levels", "id": format!("native-levels-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
    process.transport.write_record(&json!({ "type": "get_state", "id": format!("native-state-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
    process.transport.write_record(&json!({"type":"set_steering_mode","mode":"one-at-a-time","id":format!("native-steering-{}",uuid::Uuid::new_v4())})).map_err(|e|e.to_string())?;
    {
        let _preparation_lock=state.1.lock().map_err(|_|"任务准备状态锁异常")?;
        if cancelled.load(std::sync::atomic::Ordering::SeqCst){return Err("项目记忆加载已取消，未发送需求".into());}
        process.transport.write_record(&json!({ "type": "prompt", "id": request_id, "message": requirement, "images": images })).map_err(|e| e.to_string())?;
        process.transport.flush().map_err(|e| e.to_string())?;
    }
    session_repository::edit(&app,&mod_id,SessionChange::Runtime {status:DevelopmentStatus::Running,title:None,touch:false})?;
    Ok(process.task.as_ref().ok_or("任务记录缺失")?.context.clone())
    })();
    // 准备流程决定失败出口，终止协调统一负责已启动进程的回收与终态补偿。
    if let Err(error)=&started {
        termination::preparation_failed(&app,&mut inner,&mut task,&preparation_cancel,error)?;
    }
    started
}

#[cfg(test)]
mod tests {
    use super::*;
    fn task()->TaskContext{serde_json::from_value(json!({"modId":"p","sessionId":"s","taskId":"t","generation":2,"contextSnapshotId":"snap","memoryBootstrapSha256":"hash","memoryBootstrapPath":"file","sourceDirectory":"source","stagingId":"stage","stagingDirectory":"stage","createdAt":"now"})).unwrap()}
    fn ack()->MemoryBootstrapAck{MemoryBootstrapAck::Ready{mod_id:"p".into(),session_id:"s".into(),task_id:"t".into(),generation:2,snapshot_id:"snap".into(),sha256:"hash".into()}}
    #[test]
    fn memory_bootstrap_rejects_error_wrong_identity_timeout_exit_and_late_ready(){
        let task=task();
        for raw in [json!({"status":"ready","modId":"p","sessionId":"s","taskId":"other","generation":2,"snapshotId":"snap","sha256":"hash"}),json!({"status":"ready","modId":"p","sessionId":"s","taskId":"t","generation":2,"snapshotId":"snap","sha256":"bad"}),json!({"status":"error","modId":"p","sessionId":"s","taskId":"t","generation":2,"error":"无法读取本任务项目记忆"})] {
            let (tx,rx)=std::sync::mpsc::channel();tx.send(Ok(serde_json::from_value(raw).unwrap())).unwrap();
            assert!(bootstrap_ready(&rx,&task,std::time::Duration::ZERO,&std::sync::atomic::AtomicBool::new(false)).is_err());
        }
        let (tx,rx)=std::sync::mpsc::channel();
        assert!(bootstrap_ready(&rx,&task,std::time::Duration::ZERO,&std::sync::atomic::AtomicBool::new(false)).is_err());
        tx.send(Ok(ack())).unwrap();assert!(bootstrap_ready(&rx,&task,std::time::Duration::ZERO,&std::sync::atomic::AtomicBool::new(true)).is_err());
        drop(tx);assert!(bootstrap_ready(&rx,&task,std::time::Duration::ZERO,&std::sync::atomic::AtomicBool::new(true)).is_err());
        let (tx,rx)=std::sync::mpsc::channel::<Result<MemoryBootstrapAck,String>>();drop(tx);
        assert!(bootstrap_ready(&rx,&task,std::time::Duration::ZERO,&std::sync::atomic::AtomicBool::new(false)).is_err());
    }
    #[test]
    fn bootstrap_channel_can_deliver_while_target_slot_is_locked(){
        let slot=Arc::new(Mutex::new(()));let held=slot.lock().unwrap();
        let (tx,rx)=std::sync::mpsc::channel();
        let output = concat!("initializing\n", "GEN_MOD_MEMORY_BOOTSTRAP {\"status\":\"ready\",\"modId\":\"p\",\"sessionId\":\"s\",\"taskId\":\"t\",\"generation\":2,\"snapshotId\":\"snap\",\"sha256\":\"hash\"}\n", "ready\n");
        let worker_slot=slot.clone();
        let worker=crate::pi::process::read_diagnostics(std::io::Cursor::new(output.as_bytes()),vec![],true,tx,move |_| {
            let _held=worker_slot.lock().unwrap();true
        });
        let result=bootstrap_ready(&rx,&task(),std::time::Duration::from_secs(1),&std::sync::atomic::AtomicBool::new(false));
        drop(held);
        worker.join().unwrap();
        result.unwrap();
    }
    #[test]
    fn cancellation_during_bootstrap_wait_rejects_a_late_ready(){
        let cancelled=Arc::new(std::sync::atomic::AtomicBool::new(false));let cancel=cancelled.clone();
        let (tx,rx)=std::sync::mpsc::channel();
        let worker=std::thread::spawn(move||{cancel.store(true,std::sync::atomic::Ordering::SeqCst);tx.send(Ok(ack())).unwrap();});
        let result=bootstrap_ready(&rx,&task(),std::time::Duration::from_secs(1),&cancelled);
        worker.join().unwrap();assert!(result.is_err());
    }
}
