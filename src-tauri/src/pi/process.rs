//! 进程与线协议适配；不持会话锁，不读取或修改业务持久状态。
use super::job::{ProcessJob, JobProcess};
use serde_json::{json, Value};
use std::{io::{BufRead, BufReader, Read, Write}, process::{Child, ChildStdin, ChildStdout, ChildStderr, Command}};

pub(super) struct NativeProcess {
    child: Child,
    stdin: ChildStdin,
    job: Option<ProcessJob>,
    _configuration: tempfile::TempDir,
}
impl NativeProcess {
    pub(super) fn spawn(mut command: Command, configuration: tempfile::TempDir) -> Result<(Self, ChildStdout, ChildStderr), String> {
        let mut child = command.spawn().map_err(|e| format!("无法启动内置 pi：{e}"))?;
        let job = match ProcessJob::assign(&child) {
            Ok(job) => job,
            Err(error) => { let _ = child.kill(); let _ = child.wait(); return Err(format!("无法管理 pi 进程树：{error}")); }
        };
        let stdin = child.stdin.take().ok_or("pi stdin 未建立")?;
        let stdout = child.stdout.take().ok_or("pi stdout 未建立")?;
        let stderr = child.stderr.take().ok_or("pi stderr 未建立")?;
        Ok((Self { child, stdin, job: Some(job), _configuration: configuration }, stdout, stderr))
    }
    pub(super) fn write_record(&mut self, record: &Value) -> std::io::Result<()> { writeln!(self.stdin, "{}", record) }
    pub(super) fn flush(&mut self) -> std::io::Result<()> { self.stdin.flush() }
    pub(super) fn process_ids(&self) -> Result<Vec<u32>, String> { self.job.as_ref().ok_or("进程 Job 缺失")?.process_ids() }
    pub(super) fn describe_processes(&self, ids: &[u32]) -> Result<Vec<JobProcess>, String> { Ok(self.job.as_ref().ok_or("进程 Job 缺失")?.describe_processes(ids)) }
    pub(super) fn terminate_descendants(&self, baseline: &[u32]) -> Result<Vec<u32>, String> { self.job.as_ref().ok_or("进程 Job 缺失")?.terminate_descendants(baseline) }
    pub(super) fn terminate_and_wait(&self) -> Result<(), String> {
        if let Some(job) = &self.job { job.terminate_and_wait()?; }
        Ok(())
    }
    pub(super) fn reap(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        drop(self.job.take());
    }
}

pub(super) enum ProtocolRecord {
    Record { record: Value, original_user_message: Option<Value> },
    Invalid,
    Incomplete,
}

pub(super) fn records(reader: impl Read, secrets: Vec<String>) -> impl Iterator<Item = ProtocolRecord> {
    let mut reader = BufReader::new(reader);
    let mut bytes = vec![];
    std::iter::from_fn(move || {
        // 只按 LF 分帧，U+2028/U+2029 是 JSON 字符串内容。
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes) { Ok(0) | Err(_) => return None, _ => {} }
        if bytes.last() != Some(&b'\n') { return Some(ProtocolRecord::Incomplete); }
        Some(match serde_json::from_slice::<Value>(&bytes) {
            Ok(mut record) if record.get("type").and_then(Value::as_str).is_some() => {
                // 指纹使用原始用户消息；对外事件与日志只接收脱敏内容。
                let original_user_message = (record["type"] == "message_end" && record["message"]["role"] == "user").then(|| record["message"].clone());
                redact(&mut record, &secrets);
                ProtocolRecord::Record { record, original_user_message }
            }
            _ => ProtocolRecord::Invalid,
        })
    })
}

#[derive(serde::Deserialize)]
#[serde(tag="status",rename_all="snake_case",rename_all_fields="camelCase",deny_unknown_fields)]
pub(super) enum MemoryBootstrapAck {
    Ready{mod_id:String,session_id:String,task_id:String,generation:u64,snapshot_id:String,sha256:String},
    Error{mod_id:String,session_id:String,task_id:String,generation:u64,error:String},
}
fn redact(value: &mut Value, keys: &[String]) {
    match value {
        Value::String(text) => { for key in keys { *text = text.replace(key, "[redacted]"); } }
        Value::Array(items) => { for item in items { redact(item, keys); } }
        Value::Object(fields) => { for item in fields.values_mut() { redact(item, keys); } }
        _ => {}
    }
}

/// 握手先投递专用通道；诊断回调可以取得会话锁，但不能阻塞准备回执。
pub(super) fn read_diagnostics(
    stderr: impl Read + Send + 'static,
    secrets: Vec<String>,
    expects_bootstrap: bool,
    bootstrap_tx: std::sync::mpsc::Sender<Result<MemoryBootstrapAck, String>>,
    mut emit: impl FnMut(Vec<Value>) -> bool + Send + 'static,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut bootstrap_waiting = expects_bootstrap;
        let mut initialization_diagnostics = Vec::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Some(raw) = line.strip_prefix("GEN_MOD_MEMORY_BOOTSTRAP ") {
                let ack = if raw.len() > 4096 { Err("项目记忆回执超过上限".into()) } else { serde_json::from_str::<MemoryBootstrapAck>(raw).map_err(|_| "项目记忆回执格式无效".into()) };
                let _ = bootstrap_tx.send(ack);
                bootstrap_waiting = false;
                continue;
            }
            let message = secrets.iter().fold(line, |text, key| text.replace(key, "[redacted]"));
            if bootstrap_waiting {
                if initialization_diagnostics.len() < 20 { initialization_diagnostics.push(message.chars().take(2000).collect::<String>()); }
                continue;
            }
            let mut records: Vec<Value> = initialization_diagnostics.drain(..)
                .map(|message| json!({"type":"stderr","message":message})).collect();
            let diagnostic = message.strip_prefix("GEN_MOD_DIAGNOSTIC ").and_then(|raw| serde_json::from_str::<Value>(raw).ok()).filter(|v| v["type"] == "command_diagnostic");
            let record = if let Some(mut record) = diagnostic {
                record["message"] = format!("运行时诊断 · {}", record).into();
                record
            } else { json!({"type":"stderr","message":message}) };
            records.push(record);
            if !emit(records) { break; }
        }
        let _ = bootstrap_tx.send(Err("运行时在项目记忆加载前退出".into()));
    })
}
