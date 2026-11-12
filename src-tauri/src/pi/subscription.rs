use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope { pub(super) sequence: u64, pub(super) subscription: u64, pub(super) mod_id: String, pub(super) session_id: String, pub(super) task_id: Option<String>, pub(super) generation: u64, pub(super) kind: String, pub(super) record: Value }

use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::{Arc, Mutex}};
use tauri::ipc::Channel;

/// 所有异步发送者共享接收目标；重绑后旧 clone 也会转发到新订阅。
#[derive(Clone)]
pub(super) struct Subscription(Arc<Mutex<Delivery>>);
struct Delivery {
    channel: Channel<Envelope>,
    sequence: u64,
    reserved: u64,
    subscription: u64,
    messages: BTreeMap<(i64, String, String), Value>,
    tools: BTreeMap<String, Envelope>,
}
impl Subscription {
    pub fn new(channel: Channel<Envelope>) -> Self {
        Self(Arc::new(Mutex::new(Delivery { channel, sequence: 0, reserved: 0, subscription: 0, messages: BTreeMap::new(), tools: BTreeMap::new() })))
    }
    pub fn send(&self, mut event: Envelope) -> Result<(), String> {
        let mut state = self.0.lock().map_err(|_| "事件订阅锁异常")?;
        state.sequence += 1;
        event.sequence = state.sequence;
        event.subscription = state.subscription;
        let record = &event.record;
        if event.kind == "record" {
            if matches!(record["type"].as_str(), Some("message_start"|"message_update"|"message_end")) {
                if let (Some(timestamp), Some(role)) = (record["message"]["timestamp"].as_i64(), record["message"]["role"].as_str()) {
                    let mut message = record["message"].clone();
                    message["appInputId"] = record["inputId"].clone();
                    message["appTaskId"] = event.task_id.clone().into();
                    let tool_id = message["toolCallId"].as_str().unwrap_or("").to_string();
                    state.messages.insert((timestamp, role.into(), tool_id), message);
                }
            }
            if record["type"].as_str().is_some_and(|v| v.starts_with("tool_execution_")) {
                if let Some(id) = record["toolCallId"].as_str() {
                    let mut latest = event.clone();
                    if latest.record["args"].is_null() {
                        if let Some(previous) = state.tools.get(id) { latest.record["args"] = previous.record["args"].clone(); }
                    }
                    state.tools.insert(id.into(), latest);
                }
            }
        }
        state.channel.send(event).map_err(|e| e.to_string())
    }
    pub fn reserve(&self) -> Result<u64, String> {
        let mut state = self.0.lock().map_err(|_| "事件订阅锁异常")?;
        state.reserved += 1;
        Ok(state.reserved)
    }
    pub fn snapshot(&self, attach: Option<(u64, Channel<Envelope>)>) -> Result<Value, String> {
        let mut state = self.0.lock().map_err(|_| "事件订阅锁异常")?;
        let full = attach.is_some();
        if let Some((ticket, channel)) = attach {
            if ticket != state.reserved || ticket <= state.subscription { return Err("订阅恢复已过期，请重新核对状态".into()); }
            state.subscription = ticket;
            state.channel = channel;
        }
        Ok(json!({"sequence":state.sequence,"subscription":state.subscription,
            "messages":if full {state.messages.values().cloned().collect::<Vec<_>>()} else {vec![]},
            "tools":if full {state.tools.values().cloned().collect::<Vec<_>>()} else {vec![]}}))
    }
}
