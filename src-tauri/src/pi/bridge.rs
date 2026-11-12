use super::session_repository::{self, SessionChange};
mod preparation;
mod termination;
use termination::{refresh_changes, stop, stop_inner};
use preparation::start_mod_task_blocking;

use super::{process::{NativeProcess, ProtocolRecord, MemoryBootstrapAck}, runtime, tasks::{TaskContext, TaskRecord}};
use crate::storage::{mods::{self, StorageState}, models::{DevelopmentStatus, SessionReference}, paths};
use serde::Serialize;
use serde_json::{json, Value};
use std::{fs, io::{BufRead, BufReader, Write}, sync::{Arc, Mutex}};
use tauri::{ipc::Channel, Manager};
use super::subscription::Subscription;

pub use super::subscription::Envelope;

struct Process {
    transport: NativeProcess,
    channel: Subscription,
    mod_id: String,
    session_id: String,
    task: Option<TaskRecord>,
    active: bool,
    compacting: bool,
    prompt_id: Option<String>,
    tool_inputs: std::collections::BTreeMap<String, Value>,
    tool_names: std::collections::BTreeMap<String, String>,
    app: tauri::AppHandle,
    pending_title: Option<String>,
    waiting_leases: Vec<String>,
    command_baselines: std::collections::BTreeMap<String, Vec<u32>>,
    steering: Vec<Value>,
    steering_queue: Vec<String>,
    consumed_inputs: std::collections::VecDeque<String>,
    confirming_input: Option<String>,
    stopping: bool,
    model: crate::config::ModelDefinition,
    known_secrets: Vec<String>,
    tool_cancellations: std::collections::BTreeMap<String, Arc<std::sync::atomic::AtomicBool>>,
    bootstrap: Option<std::sync::mpsc::Receiver<Result<MemoryBootstrapAck,String>>>,
    preparation_guard: Option<PreparationGuard>,
    preparation_cancel: Arc<std::sync::atomic::AtomicBool>,
}
#[derive(Default)]
struct Inner { generation: u64, process: Option<Process> }
type Preparations=Arc<Mutex<std::collections::BTreeMap<(String,String),Preparation>>>;
struct Preparation {generation:u64,cancelled:Arc<std::sync::atomic::AtomicBool>,channel:Subscription}
#[derive(Default, Clone)]
pub struct PiState(Arc<super::manager::Registry<Inner>>,Preparations);
struct PreparationGuard {registry:Preparations,key:(String,String)}
impl Drop for PreparationGuard {fn drop(&mut self){if let Ok(mut values)=self.registry.lock(){values.remove(&self.key);}}}
#[derive(Clone)]
struct SlotState(Arc<Mutex<Inner>>);
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity { mod_id: String, session_id: String, generation: u64 }

/// 调用方先持生命周期预留；这里不持 StorageState，关闭时不反向等待存储锁。
pub(crate) fn close_idle_project(app: &tauri::AppHandle,id: &str) -> Result<(),String> {
    let slots=app.state::<PiState>().0.for_project(id)?;
    for slot in slots {
        let mut inner=slot.try_lock().map_err(|_| "项目会话正忙，请稍后重试")?;
        if inner.process.as_ref().is_some_and(|p|p.active || p.compacting || p.stopping || p.task.as_ref().is_some_and(|t|matches!(t.status.as_str(),"preparing"|"running"|"stopping"))) {return Err("请先结束项目中的活动任务或压缩操作".into());}
        stop(app,&mut inner)?;
    }
    Ok(())
}
pub(crate) fn forget_project(app: &tauri::AppHandle,id: &str) -> Result<(),String> {
    app.state::<PiState>().0.remove_project(id)
}

fn persist_task(app: &tauri::AppHandle, task: &mut TaskRecord) -> Result<(), String> {
    if !matches!(task.status.as_str(), "running" | "preparing") && task.finished_at.is_none() {
        task.finished_at = Some(mods::now()?);
    }
    crate::pi::tasks::save_task(app, task)
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
        ("record", "tool_execution_end") => Some(format!("工具 {} · {} · toolCallId {} · 退出码 {} · 命令日志 {}{}", record["toolName"].as_str().unwrap_or("未知"), if record["isError"] == true || record["result"]["details"]["status"] == "failed" { "失败" } else { "完成" }, record["toolCallId"].as_str().unwrap_or("未知"), record["result"]["details"]["exitCode"], record["result"]["details"]["logPath"].as_str().unwrap_or("见本轮 tool-results"), if record["isError"] == true { format!(" · 错误 {}",record["result"]["content"]) } else { String::new() })),
        ("artifact", "artifact_registered") => Some(format!("已登记交付版本 {}", record["versionId"].as_str().unwrap_or("未知"))),
        ("record", "response") if record["command"] == "get_state" => Some(format!("会话 {} · 历史 {} 条消息 · {}", record["data"]["sessionId"].as_str().unwrap_or("未知"), record["data"]["messageCount"], record["data"]["sessionFile"].as_str().unwrap_or("未生成历史文件"))),
        _ => None,
    };
    if let Some(message) = message {
        // 保存可关联的完整事件；不记录模型思考、提示词或环境变量。
        if let Some(task) = &process.task {
            if matches!(kind,"diagnostic"|"exit") || matches!(record["type"].as_str(),Some("tool_execution_start"|"tool_execution_end"|"task_started")) {
                let saved = (|| -> Result<(),String> {
                    let directory = paths::project_dir(&process.app,&process.mod_id)?.join("logs").join(&task.context.task_id);
                    paths::reject_redirect(&directory)?;
                    let path = directory.join("runtime-events.jsonl");
                    if path.exists() { paths::reject_redirect(&path)?; }
                    let mut file = fs::OpenOptions::new().append(true).create(true).open(path).map_err(|e|e.to_string())?;
                    writeln!(file,"{}",json!({"timestamp":mods::now()?,"generation":generation,"taskId":task.context.task_id,"kind":kind,"record":record})).map_err(|e|e.to_string())
                })();
                if let Err(error) = saved {
                    let _ = process.channel.send(Envelope {sequence:0,subscription:0,mod_id:process.mod_id.clone(),session_id:process.session_id.clone(),task_id:Some(task.context.task_id.clone()),generation,kind:"diagnostic".into(),record:json!({"type":"log_error","message":format!("任务事件日志保存失败：{error}")})});
                }
            }
        }
        let entry = super::logs::RuntimeLogEntry { timestamp: mods::now().unwrap_or_default(), generation, task_id: process.task.as_ref().map(|t| t.context.task_id.clone()), message: message.chars().take(4000).collect() };
        let saved = crate::pi::logs::log(&process.app, &process.mod_id, &process.session_id, &entry);
        if let Err(error) = saved {
            let _ = process.channel.send(Envelope { sequence: 0, subscription: 0, mod_id: process.mod_id.clone(), session_id: process.session_id.clone(), task_id: process.task.as_ref().map(|t| t.context.task_id.clone()), generation, kind: "diagnostic".into(), record: json!({"type": "log_error", "message": format!("运行日志保存失败：{error}")}) });
        }
    }
    let _ = process.channel.send(Envelope { sequence: 0, subscription: 0, mod_id: process.mod_id.clone(), session_id: process.session_id.clone(), task_id: process.task.as_ref().map(|t| t.context.task_id.clone()), generation, kind: kind.into(), record });
}

/// 命令 leader 退出不代表后代已结束；等待时释放槽锁，停止仍可进入。
fn wait_command_descendants(slot: &SlotState, generation: u64, request: &Value) -> Result<(),String> {
    let started = std::time::Instant::now();
    let mut previous = Vec::new();
    let mut last_log = started;
    let mut cleanup_started = false;
    let lease_id = request["leaseId"].as_str().ok_or("命令缺少租约 ID")?;
    loop {
        {
            let inner=slot.0.lock().map_err(|_|"会话运行锁异常")?;
            if inner.generation!=generation { return Err("任务已替换".into()); }
            let process=inner.process.as_ref().ok_or("任务已停止")?;
            let baseline = process.command_baselines.get(lease_id).ok_or("命令缺少启动前 Job 进程基线，不能释放租约")?;
            let job = &process.transport;
            let active = job.process_ids()?;
            let remaining: Vec<u32> = active.iter().copied().filter(|pid| !baseline.contains(pid)).collect();
            if remaining.is_empty() || remaining != previous || last_log.elapsed() >= std::time::Duration::from_secs(1) {
                emit(process,generation,"diagnostic",json!({"type":"command_descendants","message":format!("命令后代等待 · leaseId {lease_id} · toolCallId {} · 基线 {baseline:?} · 活动 {active:?} · 待退出 {remaining:?} · {}ms",request["toolCallId"],started.elapsed().as_millis()),"leaseId":lease_id,"toolCallId":request["toolCallId"],"baselinePids":baseline,"activePids":active,"remainingPids":remaining,"remainingProcesses":job.describe_processes(&remaining)?,"elapsedMs":started.elapsed().as_millis()}));
                previous = remaining.clone(); last_log = std::time::Instant::now();
            }
            if remaining.is_empty() {
                if cleanup_started { emit(process,generation,"diagnostic",json!({"type":"command_cleanup_complete","message":format!("本次命令后代已全部退出，保留 pi 并释放租约 · leaseId {lease_id}"),"leaseId":lease_id,"toolCallId":request["toolCallId"]})); }
                return Ok(());
            }
            if started.elapsed() >= std::time::Duration::from_secs(15) { return Err(format!("本次命令后代回收未确认，剩余 PID {remaining:?}，耗时 {}ms；必须停止运行时才能清理租约",started.elapsed().as_millis())); }
            if started.elapsed() >= std::time::Duration::from_secs(10) {
                if !cleanup_started {
                    emit(process,generation,"diagnostic",json!({"type":"command_cleanup_started","message":format!("命令已退出，回收本次新增后代 {remaining:?}，保留 pi · leaseId {lease_id}"),"leaseId":lease_id,"toolCallId":request["toolCallId"],"processes":job.describe_processes(&remaining)?}));
                    cleanup_started = true;
                }
                let terminated = job.terminate_descendants(baseline)?;
                if !terminated.is_empty() { emit(process,generation,"diagnostic",json!({"type":"command_cleanup_requested","message":format!("已请求终止命令后代 {terminated:?}，继续确认退出 · leaseId {lease_id}"),"leaseId":lease_id,"toolCallId":request["toolCallId"],"terminatedPids":terminated})); }
            }
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

fn emit_entry_confirmation(channel: &Subscription, mod_id: String, session_id: String, task_id: Option<String>, generation: u64, input: Value) {
    let _ = channel.send(Envelope { sequence:0,subscription:0,mod_id,session_id,task_id,generation,kind:"task".into(),record:json!({"type":"steering_confirmed","input":input}) });
}

fn continue_worker_cleanup(process: &mut Process, request: &Value) {
    if request["action"] != "acquire" {
        if let Some(id) = request["leaseId"].as_str() { process.waiting_leases.retain(|v|v!=id); }
        if let Some(id) = request["leaseId"].as_str() { process.command_baselines.remove(id); }
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
    session_repository::edit(app,mod_id,SessionChange::History {session_id:session_id.into(),file:actual.to_string_lossy().into_owned()}).map(|_|())

}

/// 把原生交付校验结果回传给工具，版本登记成功才允许模型收到成功结果。
fn register_prepared_delivery(app: &tauri::AppHandle, process: &mut Process, generation: u64, tool_id: &str, details: &Value) -> Result<String, String> {
    let input = process.tool_inputs.get(tool_id).ok_or("交付没有对应工具开始事件")?.clone();
    let task = process.task.as_mut().ok_or("没有活动交付任务")?;
    for raw in details["commands"].as_array().ok_or("交付缺少真实打包命令记录")? {
        record_receipt(app, task, serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?, tool_id)?;
    }
    let version = crate::artifacts::registry::register(app, task, serde_json::from_value(details.clone()).map_err(|e| e.to_string())?, &input, &process.known_secrets, None)?;
    persist_task(app, task)?;
    let artifact = json!({ "type": "artifact_registered", "taskId": task.context.task_id, "versionId": version.version_id });
    emit(process, generation, "artifact", artifact);
    Ok(version.version_id)
}

fn handle_record(app: &tauri::AppHandle, process: &mut Process, generation: u64, record: &Value, original_user_message: Option<&Value>) -> Result<(), String> {
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
        process.transport.write_record(&json!({ "type": "extension_ui_response", "id": request_id, "value": result.to_string() })).map_err(|e| e.to_string())?;
        process.transport.flush().map_err(|e| e.to_string())?;
        return Ok(());
    }
    // 先交付协议记录，派生文件检查失败也不丢失工具/消息终态。
    let mut annotated = record.clone();
    update_steering(app,process,&mut annotated)?;
    if annotated["message"]["role"] == "user" && annotated["inputId"].is_null() {
        if let Some(id) = &process.prompt_id { annotated["inputId"] = id.clone().into(); }
    }
    if let Some(message) = original_user_message {
        if let Some(id) = &process.prompt_id {
            if annotated["inputId"] == *id { super::submissions::confirm_message(app, id, message)?; }
        }
    }
    emit(process, generation, "record", annotated);
    let record_type = record["type"].as_str().unwrap_or("");
    if record_type == "response" && record["command"] == "get_session_stats" && record["success"] == true {
        if record["data"]["contextUsage"].is_object() { session_repository::save_stats(app,&process.mod_id,&process.session_id,generation,&record["data"]["contextUsage"])?; }
    }
    if record_type == "compaction_end" {
        session_repository::save_stats(app,&process.mod_id,&process.session_id,generation,&json!({"tokens":null,"percent":null,"contextWindow":null}))?;
    }
    if record_type=="message_end" && record["message"]["role"]=="user" {
        process.transport.write_record(&json!({"type":"get_entries","id":format!("native-entries-{}",uuid::Uuid::new_v4())})).map_err(|e|e.to_string())?;
        process.transport.flush().map_err(|e|e.to_string())?;
    }
    if record_type=="response" && record["command"]=="get_entries" && record["success"]==true {
        if let Some(entries) = record["data"]["entries"].as_array() {
            for input in &mut process.steering {
                if input["status"] != "confirmed" || input["entryId"].is_string() { continue; }
                if let Some(entry) = entries.iter().find(|entry|entry["type"]=="message" && entry["message"]["role"]=="user" && entry["message"]["timestamp"]==input["messageTimestamp"] && input_fingerprint(&entry["message"],None).is_ok_and(|hash|input["fingerprint"]==hash)) {
                    input["entryId"]=entry["id"].clone(); crate::pi::session_repository::steering(app,input)?;
                    emit_entry_confirmation(&process.channel,process.mod_id.clone(),process.session_id.clone(),process.task.as_ref().map(|t|t.context.task_id.clone()),generation,input.clone());
                }
            }
        }
    }
    if record_type == "compaction_start" { process.compacting = true; }
    if record_type == "compaction_end" || (record_type == "response" && record["command"] == "compact") { process.compacting = false; }
    if record_type == "tool_execution_start" {
        if let Some(id) = record["toolCallId"].as_str() { process.tool_inputs.insert(id.into(), record["args"].clone()); process.tool_names.insert(id.into(),record["toolName"].as_str().unwrap_or("").into()); process.tool_cancellations.insert(id.into(),Arc::new(std::sync::atomic::AtomicBool::new(false))); }
    }
    if record_type == "message_end" && record["message"]["stopReason"] == "error" {
        termination::fail(app, process)?;
    }
    if record_type == "agent_start" { process.active = true; }
    if record_type == "agent_settled" {
        termination::settle(app, process, generation, record["aborted"].as_bool() == Some(true))?;
    }
    if record_type == "tool_execution_end" {
        if let Some(token)=record["toolCallId"].as_str().and_then(|id|process.tool_cancellations.get(id)){token.store(true,std::sync::atomic::Ordering::SeqCst);}
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
            if let Some(id) = &process.prompt_id { super::submissions::reject(app,id,record["error"].as_str().unwrap_or("需求提交失败"))?; }
            process.active = false;
            termination::fail(app, process)?;
        } else if record["data"]["disposition"] == "handled" {
            termination::handled(app, process)?;
        }
    }
    Ok(())
}

fn spawn(app: &tauri::AppHandle, state: &SlotState, inner: &mut Inner, mod_id: &str, session_id: &str, channel: Channel<Envelope>, mut task: Option<TaskRecord>) -> Result<u64, String> {
    stop(app, inner)?;
    let channel = Subscription::new(channel);
    let connection = crate::storage::database::connect(app)?;
    let saved_generation: i64 = connection.query_row("SELECT generation FROM sessions WHERE mod_id=?1 AND session_id=?2",rusqlite::params![mod_id,session_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    inner.generation = inner.generation.max(saved_generation as u64);
    inner.generation += 1;
    connection.execute("UPDATE sessions SET generation=?3 WHERE mod_id=?1 AND session_id=?2",rusqlite::params![mod_id,session_id,inner.generation as i64]).map_err(|e|e.to_string())?;
    let generation = inner.generation;
    let preparation::CommandPlan { command, configuration, selected_model, secrets, source } =
        preparation::command(app, mod_id, session_id, generation, &mut task)?;
    let (bootstrap_tx,bootstrap_rx)=std::sync::mpsc::channel();
    let expects_bootstrap=task.is_some();
    let task_bootstrap=task.as_ref().map(|_|bootstrap_rx);
    let preparation_cancel=Arc::new(std::sync::atomic::AtomicBool::new(false));
    let preparation_guard=if task.is_some(){
        let registry=app.state::<PiState>().1.clone();
        let key=(mod_id.to_string(),session_id.to_string());
        registry.lock().map_err(|_|"任务准备状态锁异常")?.insert(key.clone(),Preparation{generation,cancelled:preparation_cancel.clone(),channel:channel.clone()});
        Some(PreparationGuard{registry,key})
    }else{None};
    let (transport, stdout, stderr) = NativeProcess::spawn(command, configuration)?;
    inner.process = Some(Process { transport, channel, mod_id: mod_id.into(), session_id: session_id.into(), task, active: false, compacting: false, prompt_id: None, tool_inputs: std::collections::BTreeMap::new(), tool_names: Default::default(), app: app.clone(), pending_title: None, waiting_leases: vec![], command_baselines: Default::default(), steering: vec![], steering_queue: vec![], consumed_inputs: Default::default(), confirming_input: None, stopping: false, model: selected_model, known_secrets: secrets.clone(), tool_cancellations: Default::default(), bootstrap: task_bootstrap, preparation_guard, preparation_cancel });
    if let Some(process) = &inner.process {
        emit(process, generation, "diagnostic", json!({ "type": "runtime_started", "message": format!("启动 pi · 会话 {session_id} · 工作目录 {}", source.display()) }));
        if let Some(task) = &process.task { emit(process, generation, "task", json!({ "type": "task_started", "context": task.context })); }
    }

    let app_stdout = app.clone();
    let shared_stdout = state.clone();
    let stdout_secrets = secrets.clone();
    std::thread::spawn(move || {
        for event in super::process::records(stdout, stdout_secrets) {
            let mut inner = match shared_stdout.0.lock() { Ok(value) => value, Err(_) => break };
            if inner.generation != generation { break; }
            let Some(process) = inner.process.as_mut() else { break; };
            match event {
                ProtocolRecord::Record { record, original_user_message } => {
                    if record["type"]=="extension_ui_request" && record["method"]=="input" && (super::task_tools::expected_tool(record["title"].as_str().unwrap_or("")).is_some() || record["title"]=="gen-mod:register-delivery") {
                        let title=record["title"].as_str().unwrap_or("").to_string();
                        let request=serde_json::from_str::<Value>(record["placeholder"].as_str().unwrap_or(""));
                        let prepared=(||->Result<_,String>{
                            let request=request.map_err(|e|e.to_string())?;
                            let task=process.task.clone().ok_or("没有活动任务")?;
                            let tool=request["toolCallId"].as_str().ok_or("缺少工具调用 ID")?.to_string();
                            let expected=if title=="gen-mod:register-delivery" {"prepare_delivery"}else{super::task_tools::expected_tool(&title).ok_or("未知工具请求")?};
                            if process.tool_names.get(&tool).map(String::as_str)!=Some(expected){return Err("请求没有对应工具调用".into());}
                            if title!="gen-mod:register-delivery" && (request["modId"]!=task.context.mod_id || request["sessionId"]!=task.context.session_id || request["taskId"]!=task.context.task_id || request["generation"]!=generation){return Err("任务请求身份不匹配".into());}
                            let input=process.tool_inputs.get(&tool).ok_or("工具参数缺失")?.clone();
                            let cancelled=process.tool_cancellations.get(&tool).ok_or("工具执行状态缺失")?.clone();
                            Ok((request,task,tool,input,cancelled))
                        })();
                        let (request,mut task,tool,input,cancelled)=match prepared {Ok(v)=>v,Err(error)=>{
                            let _=process.transport.write_record(&json!({"type":"extension_ui_response","id":record["id"],"value":json!({"error":error}).to_string()}));let _=process.transport.flush();continue;
                        }};
                        let worker_app=app_stdout.clone();let worker_slot=shared_stdout.clone();let reply_id=record["id"].clone();let secrets=process.known_secrets.clone();
                        std::thread::spawn(move||{
                            let result=(||->Result<Value,String>{
                                if title=="gen-mod:register-delivery" {
                                    let details=&request["delivery"];
                                    {
                                        let mut inner=worker_slot.0.lock().map_err(|_|"会话运行锁异常")?;
                                        if inner.generation!=generation{return Err("任务已替换".into());}
                                        let process=inner.process.as_mut().ok_or("任务已停止")?;
                                        let live=process.task.as_mut().filter(|v|v.context.task_id==task.context.task_id&&v.status=="running").ok_or("任务已停止")?;
                                        for raw in details["commands"].as_array().ok_or("缺少打包命令记录")?{record_receipt(&worker_app,live,serde_json::from_value(raw.clone()).map_err(|e|e.to_string())?,&tool)?;}
                                        task=live.clone();
                                    }
                                    let version=crate::artifacts::registry::register(&worker_app,&mut task,serde_json::from_value(details.clone()).map_err(|e|e.to_string())?,&input,&secrets,Some(&cancelled))?;
                                    Ok(json!({"versionId":version.version_id}))
                                }else{super::task_tools::execute(&worker_app,&task.context,&title,&request,&input,&secrets,&cancelled)}
                            })();
                            if let Ok(mut inner)=worker_slot.0.lock(){
                                if inner.generation==generation {
                                    if let Some(process)=inner.process.as_mut().filter(|p|p.task.as_ref().is_some_and(|t|t.context.task_id==task.context.task_id)){
                                        let value=result.unwrap_or_else(|error|json!({"error":error,"requiresNewTask":title=="gen-mod:register-delivery"}));
                                        if title=="gen-mod:propose-verification" && value.get("error").is_none(){emit(process,generation,"task",json!({"type":"feedback_draft_updated"}));}
                                        if title=="gen-mod:update-project-memory" && value["projectMemoryUpdate"]["outcome"]=="saved" {emit(process,generation,"task",json!({"type":"project_memory_updated","revision":value["projectMemoryUpdate"]["revision"]}));}
                                        if let Some(id)=value["versionId"].as_str(){if let Some(live)=process.task.as_mut(){live.registered_version_id=task.registered_version_id.clone();}emit(process,generation,"artifact",json!({"type":"artifact_registered","versionId":id,"taskId":task.context.task_id}));}
                                        let _=process.transport.write_record(&json!({"type":"extension_ui_response","id":reply_id,"value":value.to_string()}));let _=process.transport.flush();
                                    }
                                }
                            }
                        });
                        continue;
                    }
                    if record["type"] == "extension_ui_request" && record["title"] == "gen-mod:workspace" {
                        let request = serde_json::from_str::<Value>(record["placeholder"].as_str().unwrap_or(""));
                        if let Ok(request) = request {
                            let reply_id = record["id"].as_str().unwrap_or("").to_owned();
                            if let Some(context) = process.task.as_ref().map(|t|t.context.clone()) {
                                if request["taskId"] != context.task_id || request["generation"] != generation || request["modId"] != context.mod_id || request["sessionId"] != context.session_id {
                                    let _ = process.transport.write_record(&json!({"type":"extension_ui_response","id":reply_id,"value":json!({"error":"工作区请求身份不匹配"}).to_string()}));
                                    let _ = process.transport.flush();
                                    continue;
                                }
                                let action = request["action"].as_str().unwrap_or("");
                                emit(process,generation,"diagnostic",json!({"type":"workspace_request","message":format!("工作区请求 · action {action} · operation {} · leaseId {} · toolCallId {}",request["operation"],request["leaseId"],request["toolCallId"]),"request":request}));
                                if action == "acquire" {
                                    if !process.tool_inputs.contains_key(request["toolCallId"].as_str().unwrap_or("")) {
                                        let _ = process.transport.write_record(&json!({"type":"extension_ui_response","id":reply_id,"value":json!({"error":"工作区请求没有实际工具调用"}).to_string()}));
                                        let _ = process.transport.flush();
                                        continue;
                                    }
                                    if let Some(id) = request["leaseId"].as_str() { process.waiting_leases.push(id.into()); }
                                    emit(process,generation,"task",json!({"type":"workspace_waiting","toolCallId":request["toolCallId"]}));
                                }
                                let worker_app = app_stdout.clone();
                                let worker_slot = shared_stdout.clone();
                                std::thread::spawn(move || {
                                    let started = std::time::Instant::now();
                                    let workspaces = worker_app.state::<super::workspace::Workspaces>();
                                    if request["action"] == "terminate" {
                                        if let Ok(mut inner) = worker_slot.0.lock() {
                                            if inner.generation==generation {
                                                if let Some(process) = &inner.process { emit(process,generation,"diagnostic",json!({"type":"command_cleanup","message":format!("命令后代回收未确认，停止运行时 · leaseId {} · toolCallId {}",request["leaseId"],request["toolCallId"])})); }
                                                if let Err(error) = stop(&worker_app,&mut inner) {
                                                    if let Some(process) = &inner.process { emit(process,generation,"diagnostic",json!({"type":"command_cleanup_error","message":format!("运行时回收失败，租约保留：{error}")})); }
                                                }
                                            }
                                        }
                                        return;
                                    }
                                    let reference_check=if request["action"]=="acquire" && matches!(request["operation"].as_str(),Some("run_command"|"prepare_delivery")) {crate::project_context::check_task_references(&worker_app,&context)}else{Ok(())};
                                    let result = if let Err(error)=reference_check {Err(error)}else{match request["action"].as_str() {
                                        Some("acquire") => workspaces.acquire(&worker_app,&context,&request).and_then(|value| {
                                            if !matches!(request["operation"].as_str(),Some("run_command"|"prepare_delivery")) { return Ok(value); }
                                            // 独占租约取得后、回传给扩展前取基线，此时命令尚未启动。
                                            let captured = (|| -> Result<Value,String> {
                                                let mut inner = worker_slot.0.lock().map_err(|_|"会话运行锁异常")?;
                                                if inner.generation != generation { return Err("任务已替换".into()); }
                                                let process = inner.process.as_mut().ok_or("任务已停止")?;
                                                let ids = process.transport.process_ids()?;
                                                let lease_id = request["leaseId"].as_str().ok_or("命令租约缺少 ID")?;
                                                emit(process,generation,"diagnostic",json!({"type":"command_baseline","message":format!("命令启动前 Job 基线 · leaseId {lease_id} · toolCallId {} · PID {ids:?}",request["toolCallId"]),"leaseId":lease_id,"toolCallId":request["toolCallId"],"processIds":ids}));
                                                process.command_baselines.insert(lease_id.into(),ids);
                                                Ok(value)
                                            })();
                                            if captured.is_err() { let _ = workspaces.cancel(&context.mod_id,request["leaseId"].as_str().unwrap_or("")); }
                                            captured
                                        }),
                                        Some("release") => {
                                            let completed = if matches!(request["operation"].as_str(),Some("run_command"|"prepare_delivery")) { wait_command_descendants(&worker_slot,generation,&request) } else { Ok(()) };
                                            if let Err(error) = completed {
                                                // 不能把等待超时当成命令完成；先确认 Job 已终止，再由 stop 清理租约。
                                                if let Ok(mut inner) = worker_slot.0.lock() {
                                                    if inner.generation == generation {
                                                        if let Some(process) = &inner.process { emit(process,generation,"diagnostic",json!({"type":"command_cleanup","message":error,"leaseId":request["leaseId"],"toolCallId":request["toolCallId"]})); }
                                                        if let Err(cleanup_error) = stop_inner(&worker_app,&mut inner,true) {
                                                            if let Some(process) = &inner.process { emit(process,generation,"diagnostic",json!({"type":"command_cleanup_error","message":format!("运行时回收失败，租约保留：{cleanup_error}")})); }
                                                        }
                                                    }
                                                }
                                                return;
                                            }
                                            workspaces.release(&worker_app,&context,&request)
                                        },
                                        Some("cancel") => workspaces.cancel(&context.mod_id,request["leaseId"].as_str().unwrap_or("")).map(|_|json!({})),
                                        _ => Err("未知工作区请求".into())
                                    }};
                                    if let Ok(mut inner) = worker_slot.0.lock() {
                                        if inner.generation == generation {
                                            if let Some(process) = inner.process.as_mut() {
                                                let value = result.unwrap_or_else(|error|json!({"error":error}));
                                                emit(process,generation,"diagnostic",json!({"type":"workspace_response","message":format!("工作区响应 · action {} · operation {} · leaseId {} · toolCallId {} · {}ms · {value}",request["action"],request["operation"],request["leaseId"],request["toolCallId"],started.elapsed().as_millis()),"request":request,"result":value,"elapsedMs":started.elapsed().as_millis()}));
                                                if let Err(error) = process.transport.write_record(&json!({"type":"extension_ui_response","id":reply_id,"value":value.to_string()})).and_then(|_|process.transport.flush()) {
                                                    emit(process,generation,"diagnostic",json!({"type":"workspace_response_error","message":format!("工作区响应发送失败：{error} · leaseId {} · toolCallId {}",request["leaseId"],request["toolCallId"])}));
                                                }
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
                    if let Err(error) = handle_record(&app_stdout, process, generation, &record, original_user_message.as_ref()) {
                        emit(process, generation, "diagnostic", json!({ "type": "bridge_error", "message": error }));
                    }
                }
                ProtocolRecord::Invalid => emit(process, generation, "diagnostic", json!({ "type": "protocol_error", "message": "pi 输出了无效 JSONL 记录" })),
                ProtocolRecord::Incomplete => {
                    emit(process, generation, "diagnostic", json!({ "type": "protocol_error", "message": "pi 退出前留下不完整 JSONL 记录" }));
                    break;
                }
            }
        }
        if let Ok(mut inner) = shared_stdout.0.lock() {
            if inner.generation == generation {
                let _ = stop_inner(&app_stdout, &mut inner,true);
            }
        }
    });
    let shared_stderr = state.clone();
    // 非当前 provider 的 Key 同样不能进入诊断或任务日志。
    super::process::read_diagnostics(stderr, secrets, expects_bootstrap, bootstrap_tx, move |records| {
        let inner = match shared_stderr.0.lock() { Ok(value) => value, Err(_) => return false };
        if inner.generation != generation { return false; }
        if let Some(process) = &inner.process { for record in records { emit(process, generation, "diagnostic", record); } }
        true
    });
    Ok(generation)
}

fn session(app: &tauri::AppHandle, mod_id: &str, requested: Option<&str>) -> Result<String, String> {
    session_selected(app,mod_id,requested,None)
}
fn session_selected(app: &tauri::AppHandle, mod_id: &str, requested: Option<&str>, new_id: Option<&str>) -> Result<String, String> {
    let project = session_repository::refresh_skills(app,mod_id)?;
    if let Some(id) = requested.filter(|id| !id.is_empty()) {
        paths::validate_id(id)?;
        let value = project.sessions.iter().find(|s| s.session_id == id).ok_or("会话不属于所选项目")?;
        if value.read_only { return Err("此会话的能力上下文已改变，只能查看历史".into()) }
        return Ok(id.into());
    }
    let id = new_id.map(str::to_owned).unwrap_or_else(||uuid::Uuid::new_v4().to_string());
    let skills = crate::skills::registry::snapshots(&crate::skills::registry::resolve_for_game(app, &project.game_id)?);
    let model = crate::config::session_model_state(app, None, "medium");
    let model=if !model.selection.provider_id.is_empty() && !model.selection.model_id.is_empty() {Some(model.selection)} else {None};
    session_repository::edit(app,mod_id,SessionChange::Create {
        session:SessionReference { session_id:id.clone(),session_file:String::new(),skills,read_only:false,name:String::new() }, model
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
    let requested=session_id.filter(|id|!id.is_empty());
    let id=requested.clone().unwrap_or_else(||uuid::Uuid::new_v4().to_string());
    let slot = SlotState(state.0.slot(&mod_id,&id)?);
    let mut inner = slot.0.lock().map_err(|_| "会话运行锁异常")?;
    let _reservation=app.state::<super::workspace::Workspaces>().try_reserve_native(&mod_id,super::workspace::NativeOperation::ProcessConnect)?;
    if inner.process.as_ref().is_some_and(|p| p.active || p.compacting || p.task.as_ref().is_some_and(|t| t.status == "running")) { return Err("目标会话正在运行".into()); }
    let _storage=storage.0.lock().map_err(|_| "项目存储锁异常")?;
    session_selected(&app,&mod_id,requested.as_deref(),Some(&id))?;
    let metadata = crate::pi::session_repository::read_session_metadata(app.clone(),mod_id.clone(),id.clone())?;
    if let Some(error) = metadata["modelState"]["error"].as_str().filter(|error| !error.is_empty()) { return Err(error.into()); }
    let selection = serde_json::from_value::<crate::config::ModelSelection>(metadata["modelState"]["selection"].clone()).map_err(|e| e.to_string())?;
    session_repository::save_model(&app, &mod_id, &id, Some(&selection))?;
    drop(_storage);
    let generation = spawn(&app, &slot, &mut inner, &mod_id, &id, on_event, None)?;
    if let Some(process) = inner.process.as_mut() {
        process.transport.write_record(&json!({ "type": "set_thinking_level", "level": metadata["modelState"]["thinkingLevel"], "id": format!("native-thinking-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
        process.transport.write_record(&json!({ "type": "get_available_thinking_levels", "id": format!("native-levels-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
        process.transport.write_record(&json!({ "type": "set_auto_compaction", "enabled": true, "id": format!("native-auto-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
        process.transport.write_record(&json!({ "type": "get_state", "id": format!("native-state-{}", uuid::Uuid::new_v4()) })).map_err(|e| e.to_string())?;
        process.transport.flush().map_err(|e| e.to_string())?;
    }
    Ok(Identity { mod_id, session_id: id, generation })
}

#[tauri::command]
pub fn save_session_thinking(app: tauri::AppHandle, state: tauri::State<PiState>, storage: tauri::State<StorageState>, mod_id: String, session_id: String, level: String) -> Result<(), String> {
    let slot = state.0.slot(&mod_id, &session_id)?;
    let inner = slot.lock().map_err(|_| "会话运行锁异常")?;
    if inner.process.as_ref().is_some_and(|p| p.active || p.compacting || p.stopping || p.task.as_ref().is_some_and(|t| t.status == "running")) { return Err("请等待会话空闲后操作".into()); }
    let _storage = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    session(&app, &mod_id, Some(&session_id))?;
    let (selection, preferred) = crate::pi::session_repository::model_preferences(&app, &mod_id, &session_id)?;
    let model = crate::config::session_model_state(&app, selection, &preferred);
    if !model.error.is_empty() { return Err(model.error); }
    if !model.thinking_levels.contains(&level) { return Err("模型不支持此思考强度".into()); }
    session_repository::save_preferences(&app, &mod_id, &session_id, None, Some(&level))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelChange {
    model_state: crate::config::SessionModelState,
    generation: Option<u64>,
}

/// 只替换空闲聊天的运行时配置；失败恢复模型记录，不重放任何输入。
#[tauri::command]
pub fn select_session_model(app: tauri::AppHandle, state: tauri::State<PiState>, storage: tauri::State<StorageState>, mod_id: String, session_id: String, selection: crate::config::ModelSelection, on_event: Channel<Envelope>) -> Result<ModelChange, String> {
    let slot = SlotState(state.0.slot(&mod_id, &session_id)?);
    let mut inner = slot.0.lock().map_err(|_| "会话运行锁异常")?;
    let _reservation=app.state::<super::workspace::Workspaces>().try_reserve_native(&mod_id,super::workspace::NativeOperation::ProcessConnect)?;
    if inner.process.as_ref().is_some_and(|p| p.active || p.compacting || p.stopping || p.task.as_ref().is_some_and(|t| t.status == "running")) { return Err("请等待会话空闲后操作".into()); }
    let _storage = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    session(&app, &mod_id, Some(&session_id))?;
    let (previous, preferred) = crate::pi::session_repository::model_preferences(&app, &mod_id, &session_id)?;
    let model_state = crate::config::session_model_state(&app, Some(selection.clone()), &preferred);
    if !model_state.error.is_empty() { return Err(model_state.error); }
    // 准备配置前完成凭据校验，密钥不会返回前端。
    crate::config::read_for(&app, &selection)?;
    let reconnect = inner.process.is_some();
    drop(_storage);
    if reconnect { stop(&app, &mut inner)?; }
    {
        let _storage=storage.0.lock().map_err(|_| "项目存储锁异常")?;
        session_repository::save_model(&app, &mod_id, &session_id, Some(&selection))?;
    }
    let changed = (|| -> Result<Option<u64>, String> {
        if !reconnect { return Ok(None); }
        let generation = spawn(&app, &slot, &mut inner, &mod_id, &session_id, on_event, None)?;
        let process = inner.process.as_mut().ok_or("运行时未建立")?;
        process.transport.write_record(&json!({"type":"set_thinking_level","level":model_state.thinking_level,"id":format!("native-thinking-{}",uuid::Uuid::new_v4())})).map_err(|e| e.to_string())?;
        process.transport.write_record(&json!({"type":"set_auto_compaction","enabled":true,"id":format!("native-auto-{}",uuid::Uuid::new_v4())})).map_err(|e| e.to_string())?;
        process.transport.write_record(&json!({"type":"get_available_thinking_levels","id":format!("native-levels-{}",uuid::Uuid::new_v4())})).map_err(|e| e.to_string())?;
        process.transport.write_record(&json!({"type":"get_state","id":format!("native-state-{}",uuid::Uuid::new_v4())})).map_err(|e| e.to_string())?;
        process.transport.flush().map_err(|e| e.to_string())?;
        Ok(Some(generation))
    })();
    match changed {
        Ok(generation) => Ok(ModelChange { model_state, generation }),
        Err(error) => {
            let cleanup = stop(&app, &mut inner);
            let _storage=storage.0.lock().map_err(|_| "项目存储锁异常")?;
            session_repository::save_model(&app, &mod_id, &session_id, previous.as_ref()).map_err(|rollback| format!("{error}；恢复模型记录失败：{rollback}"))?;
            cleanup.map_err(|cleanup| format!("{error}；回收运行时失败：{cleanup}"))?;
            Err(format!("模型切换失败，已恢复原选择；{error}"))
        }
    }
}

#[tauri::command]
pub fn send_pi_command(app: tauri::AppHandle, state: tauri::State<PiState>, mod_id: String, session_id: String, generation: u64, command: Value) -> Result<(), String> {
    // 任务握手持槽锁等待，取消必须能先送达，不能排到 prompt 发送之后。
    if command["type"]=="abort" {
        let preparing=state.1.lock().map_err(|_|"任务准备状态锁异常")?;
        if let Some(value)=preparing.get(&(mod_id.clone(),session_id.clone())) {
            if value.generation!=generation{return Err("连接已被替换".into());}
            if command["id"].as_str().is_none(){return Err("RPC 缺少请求 ID".into());}
            value.cancelled.store(true,std::sync::atomic::Ordering::SeqCst);
            let _=value.channel.send(Envelope{sequence:0,subscription:0,mod_id,session_id,task_id:None,generation,kind:"record".into(),record:json!({"type":"response","command":"abort","id":command["id"],"success":true})});
            return Ok(());
        }
    }
    let slot = state.0.slot(&mod_id,&session_id)?;
    let mut inner = if command["type"]=="abort" {slot.try_lock().map_err(|_|"任务准备中的取消尚未受理，请稍后重试")?}else{slot.lock().map_err(|_| "运行时锁异常")?};
    if inner.generation != generation { return Err("连接已被替换".into()) }
    let process = inner.process.as_mut().ok_or("pi 尚未连接")?;
    crate::project_lifecycle::repository::ensure_writable(&app,&mod_id)?;
    let kind = command["type"].as_str().ok_or("RPC 缺少类型")?;
    if !matches!(kind, "get_state" | "get_messages" | "get_commands" | "get_entries" | "get_tree" | "get_session_stats" | "get_available_thinking_levels" | "set_thinking_level" | "compact" | "abort" | "extension_ui_response") { return Err("此 RPC 命令不能由前端直接发送".into()) }
    if matches!(kind, "compact" | "set_thinking_level") && (process.active || process.compacting || process.task.as_ref().is_some_and(|t| t.status == "running")) { return Err("请等待会话空闲后操作".into()); }
    if kind == "set_thinking_level" && !process.model.thinking_levels().iter().any(|level| command["level"].as_str() == Some(level.as_str())) { return Err("模型不支持此思考强度".into()); }
    if command["id"].as_str().is_none() { return Err("RPC 缺少请求 ID".into()) }
    if kind=="abort" {
        emit(process,generation,"record",json!({"type":"response","command":"abort","id":command["id"],"success":true}));
        if let Err(error)=stop(&app,&mut inner) {
            if let Some(process)=&inner.process { emit(process,generation,"diagnostic",json!({"type":"stop_error","message":error})); }
            return Err(error);
        }
        return Ok(());
    }
    process.transport.write_record(&command).map_err(|e| e.to_string())?;
    process.transport.flush().map_err(|e| e.to_string())?;
    if kind == "compact" { process.compacting = true; }

    Ok(())
}

#[tauri::command]
pub fn disconnect_pi(app: tauri::AppHandle, state: tauri::State<PiState>, mod_id: String, session_id: String, generation: u64) -> Result<(), String> {
    let slot = state.0.slot(&mod_id,&session_id)?;
    let mut inner = slot.lock().map_err(|_| "运行时锁异常")?;
    // 模型切换启动失败后原生代次可能已增加，但已回收的槽不再需要停止。
    if inner.process.is_none() { return Ok(()); }
    if inner.generation != generation { return Err("连接已被替换".into()); }
    stop(&app, &mut inner)
}

#[tauri::command]
pub async fn start_mod_task(app: tauri::AppHandle, mod_id: String, session_id: String, requirement: String, thinking_level: String, images: Option<Vec<Value>>, request_id: String, on_event: Channel<Envelope>) -> Result<TaskContext, String> {
    super::submissions::register(&app,&mod_id,&session_id,&request_id)?;
    // 文件快照与进程回收可能耗时，保持原有锁和校验顺序，在阻塞线程执行。
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<PiState>();
        let storage = app.state::<StorageState>();
        let result = start_mod_task_blocking(app.clone(), state, storage, mod_id, session_id, requirement, thinking_level, images, request_id.clone(), on_event);
        if let Err(error) = &result { super::submissions::reject(&app,&request_id,error)?; }
        result
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
    let mut may_have_sent = false;
    let result = (|| -> Result<(), String> {
        crate::project_lifecycle::repository::ensure_writable(&app,&mod_id)?;
        paths::validate_id(&input_id)?;
        let images = images.unwrap_or_default(); validate_images(&images)?;
        if message.trim().is_empty() { return Err("引导不能为空".into()); }
        let slot = state.0.slot(&mod_id,&session_id)?;
        let mut inner = slot.lock().map_err(|_| "会话运行锁异常")?;
        let repository = crate::pi::session_repository::list_steering_inputs(app.clone(),mod_id.clone(),session_id.clone())?;
        if repository.iter().any(|v|v["inputId"]==input_id) {
            may_have_sent = true;
            return Err("引导 ID 已提交，不自动重发".into());
        }
        if inner.generation != generation { return Err("会话代次已变化".into()); }
        let process = inner.process.as_mut().ok_or("任务已结束，请保留输入并发送新需求")?;
        let task = process.task.as_ref().ok_or("没有活动任务")?;
        if task.context.task_id != task_id || task.status != "running" || !process.active || process.compacting || process.stopping { return Err("任务无法接收引导，请保留输入后重试或发送新需求".into()); }
        if !images.is_empty() && !process.model.input.iter().any(|input| input == "image") { return Err("当前模型不支持图片，请移除图片附件或切换模型".into()); }
        let fingerprint = input_fingerprint(&json!({"content":[{"type":"text","text":message}]}),Some(&images))?;
        let mut input = json!({"inputId":input_id,"modId":mod_id,"sessionId":session_id,"taskId":task_id,"generation":generation,"submittedAt":mods::now()?,"message":message,"fingerprint":fingerprint,"status":"submitted"});
        crate::pi::session_repository::steering(&app,&input)?;
        // 从尝试写入开始，失败也不能证明消息未到达 pi。
        may_have_sent = true;
        let sent = process.transport.write_record(&json!({"type":"steer","id":input_id,"message":message,"images":images})).and_then(|_|process.transport.flush());
        if let Err(error) = sent {
            input["status"]="unknown".into(); crate::pi::session_repository::steering(&app,&input)?;
            return Err(format!("引导投递未确认，不自动重发：{error}"));
        }
        process.steering.push(input);
        Ok(())
    })();
    result.map_err(|error| if may_have_sent { error } else { format!("STEERING_NOT_SENT:{error}") })
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
                crate::pi::session_repository::steering(app,input)?;
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
            if let Some(input) = process.steering.iter_mut().find(|v|v["inputId"]==id) { input["status"]="unknown".into(); crate::pi::session_repository::steering(app,input)?; }
        }
    }
    if matches!(kind.as_str(),"message_start"|"message_update"|"message_end") && record["message"]["role"]=="user" {
        if let Some(id) = &process.confirming_input {
            if let Some(input) = process.steering.iter_mut().find(|v|v["inputId"]==*id) {
                record["inputId"]=id.clone().into(); record["taskId"]=input["taskId"].clone();
                if kind=="message_end" {
                    input["status"]="confirmed".into(); input["messageTimestamp"]=record["message"]["timestamp"].clone();
                    crate::pi::session_repository::steering(app,input)?; process.confirming_input=None;
                }
            }
        }
    }
    if kind=="agent_settled" {
        for input in &mut process.steering {
            if matches!(input["status"].as_str(),Some("submitted"|"queued")) { input["status"]="unknown".into(); crate::pi::session_repository::steering(app,input)?; }
        }
    }
    Ok(())
}



/// 执行任务准备；仅由后台提交入口调用，不直接注册为 IPC 命令。
#[tauri::command]
pub async fn reserve_pi_subscription(app: tauri::AppHandle, mod_id: String, session_id: String) -> Result<Value,String> {
    tauri::async_runtime::spawn_blocking(move || {
        let slot=app.state::<PiState>().0.slot(&mod_id,&session_id)?;
        let inner=slot.lock().map_err(|_|"运行时锁异常")?;
        Ok(json!({"generation":inner.generation,"ticket":inner.process.as_ref().map(|p|p.channel.reserve()).transpose()?.unwrap_or(0)}))
    }).await.map_err(|e|e.to_string())?
}

/// 只读取或附着已有进程；快照和订阅切换共享槽锁，不创建或替换执行链。
#[tauri::command]
pub async fn read_pi_runtime(app: tauri::AppHandle, mod_id: String, session_id: String, attach_generation: Option<u64>, ticket: Option<u64>, on_event: Channel<Envelope>) -> Result<Value,String> {
    tauri::async_runtime::spawn_blocking(move || {
        let slot=app.state::<PiState>().0.slot(&mod_id,&session_id)?;
        let inner=slot.lock().map_err(|_|"运行时锁异常")?;
        if attach_generation.is_some_and(|v|v!=inner.generation) {return Err("恢复期间任务已切换，请重新连接".into());}
        let project=mods::read_project(&app,&mod_id)?;
        if !project.sessions.iter().any(|v|v.session_id==session_id) {return Err("会话不属于项目".into());}
        let full=attach_generation.is_some();
        // 持槽锁读取历史，随后捕获内存中的累计消息；期间的后续输出在快照后投递。
        let history=if full {super::sessions::read_session_history(app.clone(),mod_id.clone(),session_id.clone())?} else {json!({"messages":[]})};
        let Some(process)=inner.process.as_ref() else {
            let task=crate::pi::tasks::tasks(&app,&mod_id)?.into_iter().find(|task|task.context.session_id==session_id);
            return Ok(json!({"modId":mod_id,"sessionId":session_id,"generation":inner.generation,"connected":false,"active":false,"compacting":false,"stopping":false,"running":false,"sequence":0,"subscription":0,"task":task,"messages":history["messages"],"tools":[]}));
        };
        let attach=if full {Some((ticket.ok_or("缺少订阅票据")?,on_event))} else {None};
        let mut result=process.channel.snapshot(attach)?;
        if full {
            let mut messages=std::collections::BTreeMap::new();
            for message in history["messages"].as_array().into_iter().flatten().chain(result["messages"].as_array().into_iter().flatten()) {
                if let (Some(timestamp),Some(role))=(message["timestamp"].as_i64(),message["role"].as_str()) {messages.insert((timestamp,role.to_string(),message["toolCallId"].as_str().unwrap_or("").to_string()),message.clone());}
            }
            result["messages"]=json!(messages.into_values().collect::<Vec<_>>());
        }
        result["modId"]=mod_id.into(); result["sessionId"]=session_id.into(); result["generation"]=inner.generation.into();
        result["connected"]=true.into(); result["active"]=process.active.into(); result["compacting"]=process.compacting.into(); result["stopping"]=process.stopping.into();
        result["running"]=(process.active||process.task.as_ref().is_some_and(|t|matches!(t.status.as_str(),"preparing"|"running"|"stopping"))).into();
        result["task"]=serde_json::to_value(&process.task).map_err(|e|e.to_string())?;
        Ok(result)
    }).await.map_err(|e|e.to_string())?
}

/// 执行任务准备；仅由后台提交入口调用，不直接注册为 IPC 命令。

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
    crate::project_lifecycle::repository::ensure_writable(&app,&mod_id)?;
    paths::validate_id(&session_id)?;
    let project = mods::read_project(&app, &mod_id)?;
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
    let operation_id=uuid::Uuid::new_v4().to_string();
    let recovery=history.as_ref().map(|(file,temporary)|json!({"sessionId":session_id,"original":file.file_name().unwrap().to_string_lossy(),"tombstone":temporary.file_name().unwrap().to_string_lossy()}));
    if let Some(payload)=&recovery { crate::application::file_operations::file_operation(&app,&operation_id,&mod_id,"session_delete","pending",payload)?; }
    if let Some((file, temporary)) = &history { fs::rename(file, temporary).map_err(|e| e.to_string())?; }
    let removed=crate::storage::database::connect(&app).and_then(|mut db|session_repository::edit_on(&mut db,&mod_id,project.revision,SessionChange::Delete {session_id:session_id.clone()}));
    if let Err(error) = &removed {
        if let Some((file, temporary)) = &history { fs::rename(temporary, file).map_err(|e| format!("{error}；历史恢复失败：{e}"))?; }
        if let Some(payload)=&recovery { crate::application::file_operations::file_operation(&app,&operation_id,&mod_id,"session_delete","done",payload)?; }
        return Err(error.clone());
    }
    if let Some((_, temporary)) = history { fs::remove_file(temporary).map_err(|e| format!("会话已移除，但历史临时文件清理失败：{e}"))?; }
    if let Some(payload)=&recovery { crate::application::file_operations::file_operation(&app,&operation_id,&mod_id,"session_delete","done",payload)?; }
    removed
}

/// 能力编辑与任务启动使用同一锁顺序；活动任务期间不改变加载上下文。
pub fn while_idle<T>(app: &tauri::AppHandle, operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let state = app.state::<PiState>();
    let slots = state.0.all()?;
    let guards = slots.iter().map(|slot| slot.lock().map_err(|_| "会话运行锁异常")).collect::<Result<Vec<_>,_>>()?;
    if guards.iter().any(|inner| inner.process.as_ref().is_some_and(|p| p.active || p.compacting || p.task.as_ref().is_some_and(|t| t.status == "running"))) { return Err("请等待会话空闲后操作".into()); }
    operation()
}
