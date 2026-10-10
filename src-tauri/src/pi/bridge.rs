use super::{job::ProcessJob, runtime, tasks::{TaskContext, TaskRecord}};
use crate::storage::{mods::{self, StorageState}, models::{DevelopmentStatus, SessionReference}, paths};
use serde::Serialize;
use serde_json::{json, Value};
use std::{fs, io::{BufRead, BufReader, Write}, process::{Child, ChildStdin}, sync::{Arc, Mutex}};
use tauri::{ipc::Channel, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope { mod_id: String, session_id: String, task_id: Option<String>, generation: u64, kind: String, record: Value }

struct Process {
    child: Child,
    stdin: ChildStdin,
    _job: Option<ProcessJob>,
    channel: Channel<Envelope>,
    mod_id: String,
    session_id: String,
    task: Option<TaskRecord>,
    active: bool,
    compacting: bool,
    prompt_id: Option<String>,
    tool_inputs: std::collections::BTreeMap<String, Value>,
    app: tauri::AppHandle,
    pending_title: Option<String>,
    waiting_leases: Vec<String>,
    steering: Vec<Value>,
    steering_queue: Vec<String>,
    consumed_inputs: std::collections::VecDeque<String>,
    confirming_input: Option<String>,
    stopping: bool,
}
#[derive(Default)]
struct Inner { generation: u64, process: Option<Process> }
#[derive(Default, Clone)]
pub struct PiState(Arc<super::manager::Registry<Inner>>);
#[derive(Clone)]
struct SlotState(Arc<Mutex<Inner>>);
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity { mod_id: String, session_id: String, generation: u64 }

fn persist_task(app: &tauri::AppHandle, task: &mut TaskRecord) -> Result<(), String> {
    if !matches!(task.status.as_str(), "running" | "preparing") && task.finished_at.is_none() {
        task.finished_at = Some(mods::now()?);
    }
    crate::storage::repository::save_task(app, task)
}

fn emit(process: &Process, generation: u64, kind: &str, record: Value) {
    let message = match (kind, record["type"].as_str().unwrap_or("")) {
        ("diagnostic", _) => record["message"].as_str().map(str::to_owned),
        ("exit", _) => Some("运行时已关闭".into()),
        ("task", "task_started") => Some("开始新一轮任务，沿用项目源码与会话历史".into()),
        ("record", "agent_start") => Some("AI 开始执行".into()),
        ("record", "agent_settled") => Some(if record["aborted"] == true { "本轮已停止" } else { "本轮已结束" }.into()),
        ("record", "compaction_start") => Some("开始压缩上下文".into()),
        ("record", "compaction_end") => Some("上下文压缩结束".into()),
        ("record", "tool_execution_start") => Some(format!("开始工具 {} · {}", record["toolName"].as_str().unwrap_or("未知"), record["args"])),
        ("record", "tool_execution_end") => Some(format!("工具 {} · {} · 命令日志 {}", record["toolName"].as_str().unwrap_or("未知"), if record["isError"] == true || record["result"]["details"]["status"] == "failed" { "失败，详情见任务工具日志" } else { "完成" }, record["result"]["details"]["logPath"].as_str().unwrap_or("见本轮 tool-results"))),
        ("artifact", "artifact_registered") => Some(format!("已登记交付版本 {}", record["versionId"].as_str().unwrap_or("未知"))),
        ("record", "response") if record["command"] == "get_state" => Some(format!("会话 {} · 历史 {} 条消息 · {}", record["data"]["sessionId"].as_str().unwrap_or("未知"), record["data"]["messageCount"], record["data"]["sessionFile"].as_str().unwrap_or("未生成历史文件"))),
        _ => None,
    };
    if let Some(message) = message {
        let entry = super::logs::RuntimeLogEntry { timestamp: mods::now().unwrap_or_default(), generation, task_id: process.task.as_ref().map(|t| t.context.task_id.clone()), message: message.chars().take(4000).collect() };
        let saved = crate::storage::repository::log(&process.app, &process.mod_id, &process.session_id, &entry);
        if let Err(error) = saved {
            let _ = process.channel.send(Envelope { mod_id: process.mod_id.clone(), session_id: process.session_id.clone(), task_id: process.task.as_ref().map(|t| t.context.task_id.clone()), generation, kind: "diagnostic".into(), record: json!({"type": "log_error", "message": format!("运行日志保存失败：{error}")}) });
        }
    }
    let _ = process.channel.send(Envelope { mod_id: process.mod_id.clone(), session_id: process.session_id.clone(), task_id: process.task.as_ref().map(|t| t.context.task_id.clone()), generation, kind: kind.into(), record });
}

/// 命令 leader 退出不代表后代已结束；等待时释放槽锁，停止仍可进入。
fn wait_command_descendants(slot: &SlotState, generation: u64) -> Result<(),String> {
    loop {
        {
            let inner=slot.0.lock().map_err(|_|"会话运行锁异常")?;
            if inner.generation!=generation { return Err("任务已替换".into()); }
            let process=inner.process.as_ref().ok_or("任务已停止")?;
            if process._job.as_ref().ok_or("进程 Job 缺失")?.active_processes()?<=1 { return Ok(()); }
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

fn notify_source_changed(app: &tauri::AppHandle, mod_id: &str, task_id: &str) {
    if let Ok(slots)=app.state::<PiState>().0.all() {
        for slot in slots {
            if let Ok(inner)=slot.lock() {
                if let Some(process)=&inner.process {
                    if process.mod_id==mod_id { emit(process,inner.generation,"task",json!({"type":"source_changed","originTaskId":task_id,"message":"共享源码已变化，修改前请重新读取相关文件"})); }
                }
            }
        }
    }
}

fn emit_entry_confirmation(channel: &Channel<Envelope>, mod_id: String, session_id: String, task_id: Option<String>, generation: u64, input: Value) {
    let _ = channel.send(Envelope { mod_id,session_id,task_id,generation,kind:"task".into(),record:json!({"type":"steering_confirmed","input":input}) });
}

fn continue_worker_cleanup(process: &mut Process, request: &Value) {
    if request["action"] != "acquire" {
        if let Some(id) = request["leaseId"].as_str() { process.waiting_leases.retain(|v|v!=id); }
    }
}

fn redact(value: &mut Value, key: &str) {
    if key.is_empty() { return; }
    match value {
        Value::String(text) => { *text = text.replace(key, "[redacted]"); }
        Value::Array(items) => { for item in items { redact(item, key); } }
        Value::Object(fields) => { for item in fields.values_mut() { redact(item, key); } }
        _ => {}
    }
}

fn record_receipt(app: &tauri::AppHandle, task: &mut TaskRecord, receipt: super::tasks::CommandReceipt, tool_id: &str) -> Result<(), String> {
    paths::validate_id(&receipt.command_id)?;
    app.state::<super::workspace::Workspaces>().validate_command(&task.context,&receipt)?;
    if receipt.task_id != task.context.task_id || receipt.tool_call_id != tool_id { return Err("命令记录不属于当前工具调用".into()); }
    let root = paths::project_dir(app, &task.context.mod_id)?.join("logs").join(&task.context.task_id);
    let expected = crate::artifacts::registry::checked_path(&root, &format!("commands/{}.log", receipt.command_id))?;
    if std::path::Path::new(&receipt.log_path).canonicalize().map_err(|e| e.to_string())? != expected.canonicalize().map_err(|e| e.to_string())? { return Err("命令日志路径不属于当前任务".into()); }
    if !task.commands.iter().any(|c| c.command_id == receipt.command_id) { task.commands.push(receipt); }
    persist_task(app, task)
}

/// 派生差异异步刷新，取得租约后再核对代次，旧任务不会覆盖新连接。
fn refresh_changes(app: &tauri::AppHandle, process: &Process, generation: u64, settled: bool) -> Result<(),String> {
    let Some(task) = process.task.clone() else { return Ok(()); };
    let slot = app.state::<PiState>().0.slot(&process.mod_id,&process.session_id)?;
    let app = app.clone();
    let channel = process.channel.clone();
    let title = process.pending_title.clone();
    // 租约可能等待其他会话写入，等待期间不能占用本会话的停止和引导入口。
    std::thread::spawn(move || {
        let context = &task.context;
        let workspaces = app.state::<super::workspace::Workspaces>();
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
                    crate::storage::repository::update(&app,&context.mod_id,|project| {
                        if task.status == "finished" {
                            if let Some(title) = &title {
                                if let Some(session) = project.sessions.iter_mut().find(|s|s.session_id==context.session_id && s.name.is_empty()) { session.name=title.clone(); }
                            }
                        }
                        project.development_status = if crate::storage::repository::has_active_tasks(&app,&context.mod_id)? { DevelopmentStatus::Running }
                            else if matches!(task.status.as_str(),"failed"|"cancelled"|"interrupted") { DevelopmentStatus::Failed }
                            else if task.registered_version_id.is_some() { DevelopmentStatus::Ready }
                            else if crate::changes::list_task_changes(app.clone(),context.mod_id.clone(),context.task_id.clone())?.is_empty() { DevelopmentStatus::Draft }
                            else { DevelopmentStatus::Generated };
                        project.updated_at=mods::now()?; Ok(())
                    })?;
                }
                let _ = channel.send(Envelope {mod_id:context.mod_id.clone(),session_id:context.session_id.clone(),task_id:Some(context.task_id.clone()),generation,kind:"task".into(),record:json!({"type":"changes_updated","taskId":context.task_id})});
                Ok(())
            })();
            drop(inner);
            let released = workspaces.release(&app,context,&json!({"leaseId":id,"toolCallId":"native-refresh","success":refreshed.is_ok()}));
            return refreshed.and(released.map(|_|()));
        } })();
        if let Err(error) = result {
            let _ = channel.send(Envelope {mod_id:context.mod_id.clone(),session_id:context.session_id.clone(),task_id:Some(context.task_id.clone()),generation,kind:"diagnostic".into(),record:json!({"type":"bridge_error","message":error})});
        }
    });
    Ok(())
}

fn stop(app: &tauri::AppHandle, inner: &mut Inner) -> Result<(), String> {
    stop_inner(app,inner,false)
}
/// 意外关闭 stdout 与用户停止分别记录，不能把进程崩溃显示为手动取消。
fn stop_inner(app: &tauri::AppHandle, inner: &mut Inner, unexpected: bool) -> Result<(), String> {
    if let Some(process) = inner.process.as_ref() {
        if let Some(job) = &process._job { job.terminate_and_wait()?; }
    }
    if let Some(mut process) = inner.process.take() {
        let _ = process.child.kill();
        let _ = process.child.wait();
        drop(process._job.take()); // 先回收命令后代，再保存最终源码差异。
        if let Some(task) = &process.task { app.state::<super::workspace::Workspaces>().cleanup(&process.mod_id,&task.context.task_id,&process.waiting_leases)?; }
        let saved = (|| -> Result<(), String> { if let Some(task) = process.task.as_mut() {
            if task.status == "running" || task.status == "preparing" {
                task.status = if unexpected {"interrupted"} else {"cancelled"}.into();
                persist_task(app, task)?;
                crate::storage::repository::update(app,&task.context.mod_id,|project| {
                    project.development_status = if crate::storage::repository::has_active_tasks(app,&task.context.mod_id)? {DevelopmentStatus::Running} else {DevelopmentStatus::Failed}; project.updated_at = mods::now()?; Ok(())
                })?;
            }
        } Ok(()) })();
        refresh_changes(app, &process, inner.generation, false)?;
        for input in &mut process.steering {
            if matches!(input["status"].as_str(),Some("submitted"|"queued")) { input["status"]="unknown".into(); crate::storage::repository::steering(app,input)?; }
        }
        emit(&process, inner.generation, "exit", json!({ "type": "exit", "reason": if unexpected {"unexpected"} else {"closed"} }));
        saved?;
    }
    Ok(())
}

fn save_session(app: &tauri::AppHandle, mod_id: &str, session_id: &str, file: &str) -> Result<(), String> {
    let state = app.state::<StorageState>();
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    mods::read_project(app, mod_id)?;
    let expected = paths::project_dir(app, mod_id)?.join("logs/sessions").canonicalize().map_err(|e| e.to_string())?;
    let input = std::path::Path::new(file);
    let actual = if input.exists() {
        paths::reject_redirect(input)?;
        input.canonicalize().map_err(|e| e.to_string())?
    } else {
        // pi 延迟创建首次会话文件；校验已有父目录后可保存未来文件关联。
        input.parent().ok_or("会话路径无效")?.canonicalize().map_err(|e| e.to_string())?
            .join(input.file_name().ok_or("会话文件名无效")?)
    };
    if !actual.starts_with(expected) { return Err("pi 会话文件位于项目会话目录之外".into()); }
    crate::storage::repository::update(app,mod_id,|project| {
        let session=project.sessions.iter_mut().find(|s|s.session_id==session_id).ok_or("会话不存在")?;
        session.session_file=actual.to_string_lossy().into_owned(); Ok(())
    }).map(|_|())

}

/// 把原生交付校验结果回传给工具，版本登记成功才允许模型收到成功结果。
fn register_prepared_delivery(app: &tauri::AppHandle, process: &mut Process, generation: u64, tool_id: &str, details: &Value) -> Result<String, String> {
    let input = process.tool_inputs.get(tool_id).ok_or("交付没有对应工具开始事件")?.clone();
    let task = process.task.as_mut().ok_or("没有活动交付任务")?;
    for raw in details["commands"].as_array().ok_or("交付缺少真实打包命令记录")? {
        record_receipt(app, task, serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?, tool_id)?;
    }
    let version = crate::artifacts::registry::register(app, task, serde_json::from_value(details.clone()).map_err(|e| e.to_string())?, &input)?;
    persist_task(app, task)?;
    let artifact = json!({ "type": "artifact_registered", "taskId": task.context.task_id, "versionId": version.version_id });
    emit(process, generation, "artifact", artifact);
    Ok(version.version_id)
}

fn handle_record(app: &tauri::AppHandle, process: &mut Process, generation: u64, record: &Value) -> Result<(), String> {
    // 复用 RPC 扩展请求通道完成应用内部登记，不弹出用户输入框。
    if record["type"] == "extension_ui_request" && record["method"] == "input" && record["title"] == "gen-mod:register-delivery" {
        let request_id = record["id"].as_str().ok_or("交付登记请求缺少 ID")?;
        let registered = (|| -> Result<String, String> {
            let request: Value = serde_json::from_str(record["placeholder"].as_str().ok_or("交付登记请求缺少数据")?).map_err(|e| e.to_string())?;
            register_prepared_delivery(app, process, generation, request["toolCallId"].as_str().ok_or("交付缺少工具 ID")?, &request["delivery"])
        })();
        let result = match registered {
            Ok(version_id) => json!({ "versionId": version_id }),
            Err(error) => {
                emit(process, generation, "diagnostic", json!({ "type": "delivery_error", "message": format!("交付登记未成功：{error}") }));
                json!({ "error": error })
            }
        };
        writeln!(process.stdin, "{}", json!({ "type": "extension_ui_response", "id": request_id, "value": result.to_string() })).map_err(|e| e.to_string())?;
        process.stdin.flush().map_err(|e| e.to_string())?;
        return Ok(());
    }
    // 先交付协议记录，派生文件检查失败也不丢失工具/消息终态。
    let mut annotated = record.clone();
    update_steering(app,process,&mut annotated)?;
    emit(process, generation, "record", annotated);
    let record_type = record["type"].as_str().unwrap_or("");
    if record_type == "response" && record["command"] == "get_session_stats" && record["success"] == true {
        if record["data"]["contextUsage"].is_object() { crate::storage::repository::save_stats(app,&process.mod_id,&process.session_id,generation,&record["data"]["contextUsage"])?; }
    }
    if record_type == "response" && record["command"] == "get_state" && record["success"] == true {
        if let Some(level) = record["data"]["thinkingLevel"].as_str() { crate::storage::repository::save_preferences(app,&process.mod_id,&process.session_id,None,Some(level))?; }
    }
    if record_type == "compaction_end" {
        crate::storage::repository::save_stats(app,&process.mod_id,&process.session_id,generation,&json!({"tokens":null,"percent":null,"contextWindow":null}))?;
    }
    if record_type=="message_end" && record["message"]["role"]=="user" {
        writeln!(process.stdin,"{}",json!({"type":"get_entries","id":format!("native-entries-{}",uuid::Uuid::new_v4())})).map_err(|e|e.to_string())?;
        process.stdin.flush().map_err(|e|e.to_string())?;
    }
    if record_type=="response" && record["command"]=="get_entries" && record["success"]==true {
        if let Some(entries) = record["data"]["entries"].as_array() {
            for input in &mut process.steering {
                if input["status"] != "confirmed" || input["entryId"].is_string() { continue; }
                if let Some(entry) = entries.iter().find(|entry|entry["type"]=="message" && entry["message"]["role"]=="user" && entry["message"]["timestamp"]==input["messageTimestamp"] && input_fingerprint(&entry["message"],None).is_ok_and(|hash|input["fingerprint"]==hash)) {
                    input["entryId"]=entry["id"].clone(); crate::storage::repository::steering(app,input)?;
                    emit_entry_confirmation(&process.channel,process.mod_id.clone(),process.session_id.clone(),process.task.as_ref().map(|t|t.context.task_id.clone()),generation,input.clone());
                }
            }
        }
    }
    if record_type == "compaction_start" { process.compacting = true; }
    if record_type == "compaction_end" || (record_type == "response" && record["command"] == "compact") { process.compacting = false; }
    if record_type == "tool_execution_start" {
        if let Some(id) = record["toolCallId"].as_str() { process.tool_inputs.insert(id.into(), record["args"].clone()); }
    }
    if record_type == "message_end" && record["message"]["stopReason"] == "error" {
        if let Some(task) = process.task.as_mut() { task.status = "failed".into(); persist_task(app, task)?; }
    }
    if record_type == "agent_start" { process.active = true; }
    if record_type == "agent_settled" {
        process.active = false;
        process.stopping = false;
        if let Some(task) = process.task.as_mut() {
            if task.status != "failed" {
                task.status = if record["aborted"].as_bool() == Some(true) { "cancelled" } else { "finished" }.into();
            }
            persist_task(app, task)?;
        }
        refresh_changes(app, process, generation, true)?;
    }
    if record_type == "tool_execution_end" {
        if let Some(task) = process.task.as_mut() {
            let tool_id = record["toolCallId"].as_str().ok_or("工具结束事件缺少 ID")?;
            let logs = paths::project_dir(app, &task.context.mod_id)?.join("logs").join(&task.context.task_id);
            let results = logs.join("tool-results");
            fs::create_dir_all(&results).map_err(|e| e.to_string())?;
            paths::reject_redirect(&results)?;
            use sha2::{Digest, Sha256};
            let filename = format!("{:x}.json", Sha256::digest(tool_id.as_bytes()));
            mods::write_json(&results.join(filename), record)?;
            if record["isError"].as_bool() != Some(true) {
                let input = process.tool_inputs.get(tool_id).ok_or("工具结果没有对应开始事件")?;
                if record["toolName"] == "run_command" {
                    let receipt: super::tasks::CommandReceipt = serde_json::from_value(record["result"]["details"].clone()).map_err(|e| e.to_string())?;
                    if receipt.program != input["program"].as_str().unwrap_or("") || serde_json::to_value(&receipt.args).map_err(|e| e.to_string())? != input["args"] { return Err("命令记录与实际调用参数不一致".into()); }
                    record_receipt(app, task, receipt, tool_id)?;
                }
                if record["toolName"] == "prepare_delivery" {
                    // 兼容旧运行时；新工具已经在返回结果前取得原生登记回执。
                    if task.registered_version_id.is_none() {
                        register_prepared_delivery(app, process, generation, tool_id, &record["result"]["details"])?;
                    }
                }
            }
        }
        refresh_changes(app, process, generation, false)?;
    }
    if record_type == "response" && record["command"] == "get_state" {
        if let Some(file) = record["data"]["sessionFile"].as_str() {
            save_session(app, &process.mod_id, &process.session_id, file)?;
        }
    }
    if record_type == "response" && process.prompt_id.as_deref() == record["id"].as_str() {
        if record["success"].as_bool() != Some(true) {
            process.active = false;
            if let Some(task) = process.task.as_mut() { task.status = "failed".into(); persist_task(app, task)?; }
        } else if record["data"]["disposition"] == "handled" {
            // handled 不证明扩展没有另行启动任务，不强行清除 active。
            if !process.active {
                if let Some(task) = process.task.as_mut() { task.status = "finished".into(); persist_task(app, task)?; }
            }
        }
    }
    Ok(())
}

fn spawn(app: &tauri::AppHandle, state: &SlotState, inner: &mut Inner, mod_id: &str, session_id: &str, channel: Channel<Envelope>, mut task: Option<TaskRecord>) -> Result<u64, String> {
    stop(app, inner)?;
    let connection = crate::storage::database::connect(app)?;
    let saved_generation: i64 = connection.query_row("SELECT generation FROM sessions WHERE mod_id=?1 AND session_id=?2",rusqlite::params![mod_id,session_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    inner.generation = inner.generation.max(saved_generation as u64);
    inner.generation += 1;
    connection.execute("UPDATE sessions SET generation=?3 WHERE mod_id=?1 AND session_id=?2",rusqlite::params![mod_id,session_id,inner.generation as i64]).map_err(|e|e.to_string())?;
    let generation = inner.generation;
    let directory = paths::project_dir(app, mod_id)?;
    let source = directory.join("source");
    paths::reject_redirect(&source)?;
    let sessions = directory.join("logs/sessions");
    fs::create_dir_all(&sessions).map_err(|e| e.to_string())?;
    paths::reject_redirect(&sessions)?;
    let config = crate::config::read(app)?;
    let config_dir = crate::config::prepare(app, &config)?;
    let mut command = runtime::command(app, &source, &sessions, &config_dir)?;
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
    for view in crate::skills::registry::resolve(app, &project.game_skill_name)? {
        command.arg("--skill").arg(std::path::Path::new(&view.definition.path).join("SKILL.md"));
    }
    command.args(["--provider", "stepfun-plan", "--model", &config.model])
        .env("STEPFUN_API_KEY", &config.key).env("STEPFUN_BASE_URL", &config.base_url).env("STEPFUN_MODEL", &config.model);
    if let Some(value) = task.as_mut() {
        value.context.generation = generation;
        command.env("GEN_MOD_TASK_CONTEXT", serde_json::to_string(&value.context).map_err(|e| e.to_string())?);
        command.arg("--extension").arg(runtime::resources(app)?.join("pi-portable-v1.1.0/extensions/mod-tools.ts"))
            .args(["--tools", "read,edit,write,grep,find,ls,run_command,prepare_delivery"]);
    } else {
        command.env_remove("GEN_MOD_TASK_CONTEXT").args(["--no-tools"]);
    }
    let mut child = command.spawn().map_err(|e| format!("无法启动内置 pi：{e}"))?;
    let job = match ProcessJob::assign(&child) {
        Ok(job) => job,
        Err(error) => { let _ = child.kill(); let _ = child.wait(); return Err(format!("无法管理 pi 进程树：{error}")); }
    };
    let stdin = child.stdin.take().ok_or("pi stdin 未建立")?;
    let stdout = child.stdout.take().ok_or("pi stdout 未建立")?;
    let stderr = child.stderr.take().ok_or("pi stderr 未建立")?;
    inner.process = Some(Process { child, stdin, _job: Some(job), channel, mod_id: mod_id.into(), session_id: session_id.into(), task, active: false, compacting: false, prompt_id: None, tool_inputs: std::collections::BTreeMap::new(), app: app.clone(), pending_title: None, waiting_leases: vec![], steering: vec![], steering_queue: vec![], consumed_inputs: Default::default(), confirming_input: None, stopping: false });
    if let Some(process) = &inner.process {
        emit(process, generation, "diagnostic", json!({ "type": "runtime_started", "message": format!("启动 pi · 会话 {session_id} · 工作目录 {}", source.display()) }));
        if let Some(task) = &process.task { emit(process, generation, "task", json!({ "type": "task_started", "context": task.context })); }
    }

    let app_stdout = app.clone();
    let shared_stdout = state.clone();
    let stdout_secret = config.key.clone();
    std::thread::spawn(move || {
        // read_until 只按 LF 分帧，U+2028/U+2029 是 JSON 字符串内容。
        let mut reader = BufReader::new(stdout);
        let mut bytes = vec![];
        loop {
            bytes.clear();
            let read = reader.read_until(b'\n', &mut bytes);
            if matches!(read, Ok(0)) || read.is_err() { break; }
            let mut inner = match shared_stdout.0.lock() { Ok(value) => value, Err(_) => break };
            if inner.generation != generation { break; }
            let Some(process) = inner.process.as_mut() else { break; };
            if bytes.last() != Some(&b'\n') {
                emit(process, generation, "diagnostic", json!({ "type": "protocol_error", "message": "pi 退出前留下不完整 JSONL 记录" }));
                break;
            }
            match serde_json::from_slice::<Value>(&bytes) {
                Ok(mut record) if record.get("type").and_then(Value::as_str).is_some() => {
                    redact(&mut record, &stdout_secret);
                    if record["type"] == "extension_ui_request" && record["title"] == "gen-mod:workspace" {
                        let request = serde_json::from_str::<Value>(record["placeholder"].as_str().unwrap_or(""));
                        if let Ok(request) = request {
                            let reply_id = record["id"].as_str().unwrap_or("").to_owned();
                            if let Some(context) = process.task.as_ref().map(|t|t.context.clone()) {
                                if request["taskId"] != context.task_id || request["generation"] != generation || request["modId"] != context.mod_id || request["sessionId"] != context.session_id {
                                    let _ = writeln!(process.stdin,"{}",json!({"type":"extension_ui_response","id":reply_id,"value":json!({"error":"工作区请求身份不匹配"}).to_string()}));
                                    let _ = process.stdin.flush();
                                    continue;
                                }
                                let action = request["action"].as_str().unwrap_or("");
                                if action == "acquire" {
                                    if !process.tool_inputs.contains_key(request["toolCallId"].as_str().unwrap_or("")) {
                                        let _ = writeln!(process.stdin,"{}",json!({"type":"extension_ui_response","id":reply_id,"value":json!({"error":"工作区请求没有实际工具调用"}).to_string()}));
                                        let _ = process.stdin.flush();
                                        continue;
                                    }
                                    if let Some(id) = request["leaseId"].as_str() { process.waiting_leases.push(id.into()); }
                                    emit(process,generation,"task",json!({"type":"workspace_waiting","toolCallId":request["toolCallId"]}));
                                }
                                let worker_app = app_stdout.clone();
                                let worker_slot = shared_stdout.clone();
                                std::thread::spawn(move || {
                                    let workspaces = worker_app.state::<super::workspace::Workspaces>();
                                    if request["action"] == "terminate" {
                                        if let Ok(mut inner) = worker_slot.0.lock() {
                                            if inner.generation==generation { let _ = stop(&worker_app,&mut inner); }
                                        }
                                        return;
                                    }
                                    let result = match request["action"].as_str() {
                                        Some("acquire") => workspaces.acquire(&worker_app,&context,&request),
                                        Some("release") => {
                                            let completed = if matches!(request["operation"].as_str(),Some("run_command"|"prepare_delivery")) { wait_command_descendants(&worker_slot,generation) } else { Ok(()) };
                                            completed.and_then(|_|workspaces.release(&worker_app,&context,&request))
                                        },
                                        Some("cancel") => workspaces.cancel(&context.mod_id,request["leaseId"].as_str().unwrap_or("")).map(|_|json!({})),
                                        _ => Err("未知工作区请求".into())
                                    };
                                    if let Ok(mut inner) = worker_slot.0.lock() {
                                        if inner.generation == generation {
                                            if let Some(process) = inner.process.as_mut() {
                                                let value = result.unwrap_or_else(|error|json!({"error":error}));
                                                let _ = writeln!(process.stdin,"{}",json!({"type":"extension_ui_response","id":reply_id,"value":value.to_string()}));
                                                let _ = process.stdin.flush();
                                                emit(process,generation,"task",json!({"type":"workspace_ready"}));
                                                continue_worker_cleanup(process,&request);
                                                drop(inner);
                                                if request["action"]=="release" && matches!(request["operation"].as_str(),Some("write"|"edit"|"run_command")) { notify_source_changed(&worker_app,&context.mod_id,&context.task_id); }
                                                return;
                                            }
                                        }
                                    }
                                    if request["action"] == "acquire" { let _ = workspaces.cancel(&context.mod_id,request["leaseId"].as_str().unwrap_or("")); }
                                });
                                continue;
                            }
                        }
                    }
                    if let Err(error) = handle_record(&app_stdout, process, generation, &record) {
                        emit(process, generation, "diagnostic", json!({ "type": "bridge_error", "message": error }));
                    }
                }
                _ => emit(process, generation, "diagnostic", json!({ "type": "protocol_error", "message": "pi 输出了无效 JSONL 记录" })),
            }
        }
        if let Ok(mut inner) = shared_stdout.0.lock() {
            if inner.generation == generation {
                let _ = stop_inner(&app_stdout, &mut inner,true);
            }
        }
    });
    let shared_stderr = state.clone();
    // 避免凭据进入诊断：只转发已移除当前 Key 的 stderr 行，不输出环境。
    let secret = config.key.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let message = if secret.is_empty() { line } else { line.replace(&secret, "[redacted]") };
            let inner = match shared_stderr.0.lock() { Ok(value) => value, Err(_) => break };
            if inner.generation != generation { break; }
            if let Some(process) = &inner.process { emit(process, generation, "diagnostic", json!({ "type": "stderr", "message": message })); }
        }
    });
    Ok(generation)
}

fn session(app: &tauri::AppHandle, mod_id: &str, requested: Option<&str>) -> Result<String, String> {
    let project = crate::storage::repository::update(app,mod_id,|project| crate::skills::registry::refresh_sessions(app,project))?;
    if let Some(id) = requested.filter(|id| !id.is_empty()) {
        paths::validate_id(id)?;
        let value = project.sessions.iter().find(|s| s.session_id == id).ok_or("会话不属于所选项目")?;
        if value.read_only { return Err("此会话的能力上下文已改变，只能查看历史".into()) }
        return Ok(id.into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let skills = crate::skills::registry::snapshots(&crate::skills::registry::resolve(app, &project.game_skill_name)?);
    crate::storage::repository::update(app,mod_id,|project| {
        project.sessions.push(SessionReference { session_id: id.clone(), session_file: String::new(), skills, read_only: false, name: String::new() }); Ok(())
    })?;
    Ok(id)
}

/// 只创建应用会话关联；查看历史不需要启动 pi 或配置 API Key。
#[tauri::command]
pub fn create_mod_session(app: tauri::AppHandle, storage: tauri::State<StorageState>, mod_id: String) -> Result<String,String> {
    let _lock=storage.0.lock().map_err(|_|"项目存储锁异常")?;
    session(&app,&mod_id,None)
}
#[tauri::command]
pub fn connect_pi(app: tauri::AppHandle, state: tauri::State<PiState>, storage: tauri::State<StorageState>, mod_id: String, session_id: Option<String>, on_event: Channel<Envelope>) -> Result<Identity, String> {
    let id = {
        let _storage = storage.0.lock().map_err(|_| "项目存储锁异常")?;
        session(&app, &mod_id, session_id.as_deref())?
    };
    let slot = SlotState(state.0.slot(&mod_id,&id)?);
    let mut inner = slot.0.lock().map_err(|_| "会话运行锁异常")?;
    if inner.process.as_ref().is_some_and(|p| p.active || p.compacting || p.task.as_ref().is_some_and(|t| t.status == "running")) { return Err("目标会话正在运行".into()); }
    let metadata = crate::storage::repository::read_session_metadata(app.clone(),mod_id.clone(),id.clone())?;
    let generation = spawn(&app, &slot, &mut inner, &mod_id, &id, on_event, None)?;
    if let Some(process) = inner.process.as_mut() {
        writeln!(process.stdin, "{}", json!({ "type": "set_thinking_level", "level": metadata["thinkingLevel"], "id": format!("native-thinking-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
        writeln!(process.stdin, "{}", json!({ "type": "set_auto_compaction", "enabled": true, "id": format!("native-auto-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
        writeln!(process.stdin, "{}", json!({ "type": "get_state", "id": format!("native-state-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
        process.stdin.flush().map_err(|e| e.to_string())?;
    }
    Ok(Identity { mod_id, session_id: id, generation })
}

#[tauri::command]
pub fn send_pi_command(app: tauri::AppHandle, state: tauri::State<PiState>, mod_id: String, session_id: String, generation: u64, command: Value) -> Result<(), String> {
    let slot = state.0.slot(&mod_id,&session_id)?;
    let mut inner = slot.lock().map_err(|_| "运行时锁异常")?;
    if inner.generation != generation { return Err("连接已被替换".into()) }
    let process = inner.process.as_mut().ok_or("pi 尚未连接")?;
    let kind = command["type"].as_str().ok_or("RPC 缺少类型")?;
    if !matches!(kind, "get_state" | "get_messages" | "get_commands" | "get_entries" | "get_tree" | "get_session_stats" | "get_available_thinking_levels" | "set_thinking_level" | "compact" | "abort" | "extension_ui_response") { return Err("此 RPC 命令不能由前端直接发送".into()) }
    if matches!(kind, "compact" | "set_thinking_level") && (process.active || process.compacting || process.task.as_ref().is_some_and(|t| t.status == "running")) { return Err("请等待会话空闲后操作".into()); }
    if kind == "set_thinking_level" && !matches!(command["level"].as_str(), Some("off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max")) { return Err("无效的思考强度".into()); }
    if command["id"].as_str().is_none() { return Err("RPC 缺少请求 ID".into()) }
    if kind=="abort" {
        emit(process,generation,"record",json!({"type":"response","command":"abort","id":command["id"],"success":true}));
        if let Err(error)=stop(&app,&mut inner) {
            if let Some(process)=&inner.process { emit(process,generation,"diagnostic",json!({"type":"stop_error","message":error})); }
            return Err(error);
        }
        return Ok(());
    }
    writeln!(process.stdin, "{}", command).map_err(|e| e.to_string())?;
    process.stdin.flush().map_err(|e| e.to_string())?;
    if kind == "compact" { process.compacting = true; }

    Ok(())
}

#[tauri::command]
pub fn disconnect_pi(app: tauri::AppHandle, state: tauri::State<PiState>, mod_id: String, session_id: String, generation: u64) -> Result<(), String> {
    let slot = state.0.slot(&mod_id,&session_id)?;
    let mut inner = slot.lock().map_err(|_| "运行时锁异常")?;
    if inner.generation != generation { return Err("连接已被替换".into()); }
    stop(&app, &mut inner)
}

#[tauri::command]
pub async fn start_mod_task(app: tauri::AppHandle, mod_id: String, session_id: String, requirement: String, thinking_level: String, images: Option<Vec<Value>>, request_id: String, on_event: Channel<Envelope>) -> Result<TaskContext, String> {
    // 文件快照与进程回收可能耗时，保持原有锁和校验顺序，在阻塞线程执行。
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<PiState>();
        let storage = app.state::<StorageState>();
        start_mod_task_blocking(app.clone(), state, storage, mod_id, session_id, requirement, thinking_level, images, request_id, on_event)
    }).await.map_err(|error| format!("任务准备线程异常：{error}"))?
}

fn validate_images(images: &[Value]) -> Result<(),String> {
    // 在创建任务之前限制附件体积及格式，避免无效图片占用任务槽。
    if images.len() > 6 || images.iter().map(|image| image["data"].as_str().map_or(0, str::len)).sum::<usize>() > 28 * 1024 * 1024 { return Err("图片附件合计不能超过 20 MB，最多 6 张".into()); }
    for image in images {
        let data = image["data"].as_str().ok_or("图片缺少 Base64 内容")?;
        if image["type"] != "image" || !matches!(image["mimeType"].as_str(), Some("image/png" | "image/jpeg" | "image/webp" | "image/gif")) || data.is_empty() || data.len() > 14 * 1024 * 1024 || data.len() % 4 != 0 || !data.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')) { return Err("图片格式或大小无效，单张上限 10 MB".into()); }
    }
    Ok(())
}

/// 只允许向身份及任务完全匹配的活动 Agent 发送引导，不创建任务或进程。
#[tauri::command]
pub fn steer_mod_task(app: tauri::AppHandle, state: tauri::State<PiState>, mod_id: String, session_id: String, generation: u64, task_id: String, input_id: String, message: String, images: Option<Vec<Value>>) -> Result<(),String> {
    paths::validate_id(&input_id)?;
    let images = images.unwrap_or_default(); validate_images(&images)?;
    if message.trim().is_empty() { return Err("引导不能为空".into()); }
    let slot = state.0.slot(&mod_id,&session_id)?;
    let mut inner = slot.lock().map_err(|_| "会话运行锁异常")?;
    if inner.generation != generation { return Err("会话代次已变化".into()); }
    let process = inner.process.as_mut().ok_or("任务已结束，请保留输入并发送新需求")?;
    let task = process.task.as_ref().ok_or("没有活动任务")?;
    if task.context.task_id != task_id || task.status != "running" || !process.active || process.compacting || process.stopping { return Err("任务无法接收引导，请保留输入后重试或发送新需求".into()); }
    let repository = crate::storage::repository::list_steering_inputs(app.clone(),mod_id.clone(),session_id.clone())?;
    if repository.iter().any(|v|v["inputId"]==input_id) { return Err("引导 ID 已提交，不自动重发".into()); }
    let fingerprint = input_fingerprint(&json!({"content":[{"type":"text","text":message}]}),Some(&images))?;
    let mut input = json!({"inputId":input_id,"modId":mod_id,"sessionId":session_id,"taskId":task_id,"generation":generation,"submittedAt":mods::now()?,"message":message,"fingerprint":fingerprint,"status":"submitted"});
    crate::storage::repository::steering(&app,&input)?;
    let sent = writeln!(process.stdin,"{}",json!({"type":"steer","id":input_id,"message":message,"images":images})).and_then(|_|process.stdin.flush());
    if let Err(error) = sent {
        input["status"]="unknown".into(); crate::storage::repository::steering(&app,&input)?;
        return Err(format!("引导投递未确认，不自动重发：{error}"));
    }
    process.steering.push(input);
    Ok(())
}
pub(crate) fn input_fingerprint(message: &Value, images: Option<&Vec<Value>>) -> Result<String,String> {
    use sha2::{Digest,Sha256};
    let content = message["content"].as_array().cloned().unwrap_or_default();
    let text = if let Some(text) = message["content"].as_str() {text.to_owned()} else { content.iter().filter_map(|v|v["text"].as_str()).collect::<Vec<_>>().join("") };
    let pictures: Vec<Value> = images.unwrap_or(&content).iter().filter(|v|v["type"]=="image").map(|v|json!([v["mimeType"],v["data"]])).collect();
    Ok(format!("{:x}",Sha256::digest(serde_json::to_vec(&json!([text,pictures])).map_err(|e|e.to_string())?)))
}
fn update_steering(app: &tauri::AppHandle, process: &mut Process, record: &mut Value) -> Result<(),String> {
    let kind = record["type"].as_str().unwrap_or("").to_owned();
    if kind == "response" && record["command"] == "steer" {
        if let Some(input) = process.steering.iter_mut().find(|v|v["inputId"]==record["id"]) {
            if input["status"] != "confirmed" {
                input["status"] = if record["success"]!=true { "rejected" } else if record["data"]["disposition"]=="handled" { "handled" } else { "queued" }.into();
                crate::storage::repository::steering(app,input)?;
            }
        }
    }
    if kind == "queue_update" {
        let next: Vec<String> = record["steering"].as_array().map(|v|v.iter().filter_map(|v|v.as_str().map(str::to_owned)).collect()).unwrap_or_default();
        let mut remaining = next.clone();
        for old in &process.steering_queue {
            if let Some(index) = remaining.iter().position(|v|v==old) { remaining.remove(index); }
            else if let Some(input) = process.steering.iter().find(|v|v["message"]==*old && matches!(v["status"].as_str(),Some("queued"|"submitted")) && !process.consumed_inputs.iter().any(|id|v["inputId"]==*id)) {
                if let Some(id) = input["inputId"].as_str() { process.consumed_inputs.push_back(id.into()); }
            }
        }
        process.steering_queue = next;
    }
    if kind == "message_start" && record["message"]["role"]=="user" {
        let fingerprint = input_fingerprint(&record["message"],None)?;
        let matched = process.consumed_inputs.front().cloned().filter(|id|process.steering.iter().any(|v|v["inputId"]==*id && v["fingerprint"]==fingerprint));
        if let Some(id) = matched {
            process.consumed_inputs.pop_front(); process.confirming_input=Some(id);
        } else if let Some(id) = process.consumed_inputs.pop_front() {
            if let Some(input) = process.steering.iter_mut().find(|v|v["inputId"]==id) { input["status"]="unknown".into(); crate::storage::repository::steering(app,input)?; }
        }
    }
    if matches!(kind.as_str(),"message_start"|"message_update"|"message_end") && record["message"]["role"]=="user" {
        if let Some(id) = &process.confirming_input {
            if let Some(input) = process.steering.iter_mut().find(|v|v["inputId"]==*id) {
                record["inputId"]=id.clone().into(); record["taskId"]=input["taskId"].clone();
                if kind=="message_end" {
                    input["status"]="confirmed".into(); input["messageTimestamp"]=record["message"]["timestamp"].clone();
                    crate::storage::repository::steering(app,input)?; process.confirming_input=None;
                }
            }
        }
    }
    if kind=="agent_settled" {
        for input in &mut process.steering {
            if matches!(input["status"].as_str(),Some("submitted"|"queued")) { input["status"]="unknown".into(); crate::storage::repository::steering(app,input)?; }
        }
    }
    Ok(())
}

/// 执行任务准备；仅由后台提交入口调用，不直接注册为 IPC 命令。
fn start_mod_task_blocking(app: tauri::AppHandle, state: tauri::State<PiState>, storage: tauri::State<StorageState>, mod_id: String, session_id: String, requirement: String, thinking_level: String, images: Option<Vec<Value>>, request_id: String, on_event: Channel<Envelope>) -> Result<TaskContext, String> {
    let images = images.unwrap_or_default();
    validate_images(&images)?;
    let slot = SlotState(state.0.slot(&mod_id,&session_id)?);
    let mut inner = slot.0.lock().map_err(|_| "运行时锁异常")?;
    if inner.process.as_ref().is_some_and(|p| p.active || p.compacting || p.task.as_ref().is_some_and(|t| t.status == "running")) { return Err("已有任务正在运行，请先取消或等待结束".into()) }
    let _storage = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    mods::read_project(&app, &mod_id)?;
    // 页面已经保存原始草稿；prompt 含附件展开文本，不能把它写回输入框草稿。
    crate::storage::repository::save_preferences(&app,&mod_id,&session_id,None,Some(&thinking_level))?;
    if requirement.trim().is_empty() { return Err("需求不能为空".into()) }
    if !matches!(thinking_level.as_str(), "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max") { return Err("无效的思考强度".into()); }
    if crate::config::read(&app)?.key.trim().is_empty() { return Err("需求已保存；请在 env 配置 STEPFUN_API_KEY 后发送".into()) }
    let id = session(&app, &mod_id, Some(&session_id))?;
    let project = mods::read_project(&app, &mod_id)?;
    let directory = paths::project_dir(&app, &mod_id)?;
    let task_id = uuid::Uuid::new_v4().to_string();
    let staging_id = uuid::Uuid::new_v4().to_string();
    let task_directory = directory.join("logs").join(&task_id);
    let staging = task_directory.join("staging").join(&staging_id);
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let context = TaskContext { task_id, mod_id: mod_id.clone(), session_id: id.clone(), generation: inner.generation + 1, source_directory: directory.join("source").to_string_lossy().into_owned(), staging_id, staging_directory: staging.to_string_lossy().into_owned(), created_at: mods::now()? };
    let mut task = TaskRecord { context: context.clone(), status: "preparing".into(), finished_at: None, commands: vec![], registered_version_id: None };
    persist_task(&app, &mut task)?;
    let exclusions: Vec<String> = crate::skills::registry::resolve(&app, &project.game_skill_name)?.iter().flat_map(|v| v.definition.exclude_paths.clone()).collect();
    drop(_storage);
    let lease_id = uuid::Uuid::new_v4().to_string();
    let workspace_request = json!({"leaseId":lease_id,"toolCallId":"native-snapshot","operation":"snapshot"});
    let workspaces = app.state::<super::workspace::Workspaces>();
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
    let started = (|| -> Result<TaskContext,String> {
    let process = inner.process.as_mut().ok_or("pi 进程未建立")?;
    process.prompt_id = Some(request_id.clone());
    process.pending_title = project.sessions.iter().find(|s| s.session_id == id && s.name.is_empty())
        .map(|_| requirement.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(40).collect());
    if let Some(task) = process.task.as_mut() { task.status = "running".into(); persist_task(&app, task)?; }
    // task.status 已占用任务槽；active 只表示实际 Agent 事件状态。
    process.active = false;
    writeln!(process.stdin, "{}", json!({ "type": "set_auto_compaction", "enabled": true, "id": format!("native-auto-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
    writeln!(process.stdin, "{}", json!({ "type": "set_thinking_level", "level": thinking_level, "id": format!("native-thinking-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
    writeln!(process.stdin, "{}", json!({ "type": "get_state", "id": format!("native-state-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
    writeln!(process.stdin,"{}",json!({"type":"set_steering_mode","mode":"one-at-a-time","id":format!("native-steering-{}",uuid::Uuid::new_v4())})).map_err(|e|e.to_string())?;
    writeln!(process.stdin, "{}", json!({ "type": "prompt", "id": request_id, "message": requirement, "images": images })).map_err(|e| e.to_string())?;
    process.stdin.flush().map_err(|e| e.to_string())?;
    crate::storage::repository::update(&app,&mod_id,|project| { project.development_status = DevelopmentStatus::Running; Ok(()) })?;
    Ok(process.task.as_ref().ok_or("任务记录缺失")?.context.clone())
    })();
    if let Err(error)=&started {
        let cleanup=stop(&app,&mut inner);
        task.status="failed".into();
        persist_task(&app,&mut task)?;
        if let Err(cleanup)=cleanup { return Err(format!("{error}；进程回收失败：{cleanup}")); }
    }
    started
}

pub fn shutdown(app: &tauri::AppHandle) {
    if let Ok(slots) = app.state::<PiState>().0.all() {
        for slot in slots {
            if let Ok(mut inner) = slot.lock() {
                let _ = stop(app, &mut inner);
            }
        }
    }
}

/// 删除会话及其唯一历史文件；任务日志、源码和版本仍保留用于交付追溯。
#[tauri::command]
pub fn delete_mod_session(app: tauri::AppHandle, state: tauri::State<PiState>, storage: tauri::State<StorageState>, mod_id: String, session_id: String) -> Result<crate::storage::models::ModProject, String> {
    let slot = state.0.slot(&mod_id,&session_id)?;
    let mut inner = slot.lock().map_err(|_| "运行时锁异常")?;
    if inner.process.as_ref().is_some_and(|p| p.active || p.compacting || p.task.as_ref().is_some_and(|t| t.status == "running")) { return Err("请等待会话空闲后删除".into()); }
    if inner.process.as_ref().is_some_and(|p|p.mod_id==mod_id && p.session_id==session_id) { stop(&app,&mut inner)?; }
    let _lock = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    paths::validate_id(&session_id)?;
    let mut project = mods::read_project(&app, &mod_id)?;
    let session = project.sessions.iter().find(|s| s.session_id == session_id).ok_or("会话不属于当前项目")?;
    let directory = paths::project_dir(&app, &mod_id)?;
    let history = if !session.session_file.is_empty() && std::path::Path::new(&session.session_file).exists() {
        let root = directory.join("logs/sessions");
        paths::reject_redirect(&directory.join("logs"))?;
        paths::reject_redirect(&root)?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let file = std::path::PathBuf::from(&session.session_file);
        paths::reject_redirect(&file)?;
        let file = file.canonicalize().map_err(|e| e.to_string())?;
        if file.parent() != Some(root.as_path()) || !file.is_file() { return Err("会话历史文件不在项目会话目录中".into()); }
        Some((file, root.join(format!("deleted-{}.tmp", uuid::Uuid::new_v4()))))
    } else { None };
    project.sessions.retain(|s| s.session_id != session_id);
    project.updated_at = mods::now()?;
    let operation_id=uuid::Uuid::new_v4().to_string();
    let recovery=history.as_ref().map(|(file,temporary)|json!({"sessionId":session_id,"original":file.file_name().unwrap().to_string_lossy(),"tombstone":temporary.file_name().unwrap().to_string_lossy()}));
    if let Some(payload)=&recovery { crate::storage::repository::file_operation(&app,&operation_id,&mod_id,"session_delete","pending",payload)?; }
    if let Some((file, temporary)) = &history { fs::rename(file, temporary).map_err(|e| e.to_string())?; }
    if let Err(error) = crate::storage::repository::save(&app, &mut project) {
        if let Some((file, temporary)) = &history { fs::rename(temporary, file).map_err(|e| format!("{error}；历史恢复失败：{e}"))?; }
        if let Some(payload)=&recovery { crate::storage::repository::file_operation(&app,&operation_id,&mod_id,"session_delete","done",payload)?; }
        return Err(error);
    }
    if let Some((_, temporary)) = history { fs::remove_file(temporary).map_err(|e| format!("会话已移除，但历史临时文件清理失败：{e}"))?; }
    if let Some(payload)=&recovery { crate::storage::repository::file_operation(&app,&operation_id,&mod_id,"session_delete","done",payload)?; }
    Ok(project)
}

/// 能力编辑与任务启动使用同一锁顺序；活动任务期间不改变加载上下文。
pub fn while_idle<T>(app: &tauri::AppHandle, operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let state = app.state::<PiState>();
    let slots = state.0.all()?;
    let guards = slots.iter().map(|slot| slot.lock().map_err(|_| "会话运行锁异常")).collect::<Result<Vec<_>,_>>()?;
    if guards.iter().any(|inner| inner.process.as_ref().is_some_and(|p| p.active || p.compacting || p.task.as_ref().is_some_and(|t| t.status == "running"))) { return Err("请等待会话空闲后操作".into()); }
    operation()
}
