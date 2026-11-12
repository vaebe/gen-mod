use crate::storage::{database, repository::{encode, decode}};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskContext {
    pub task_id: String,
    pub mod_id: String,
    pub session_id: String,
    pub generation: u64,
    pub source_directory: String,
    pub staging_id: String,
    pub staging_directory: String,
    pub created_at: String,
    pub context_snapshot_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_bootstrap_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_bootstrap_sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandReceipt {
    pub source_revision: u64,
    pub task_id: String,
    pub command_id: String,
    pub tool_call_id: String,
    pub exit_code: Option<i32>,
    pub status: String,
    pub log_path: String,
    pub program: String,
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub started_at: String,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub output_tail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub startup_error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    pub context: TaskContext,
    pub status: String,
    #[serde(default)]
    pub finished_at: Option<String>,
    pub commands: Vec<CommandReceipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_version_id: Option<String>,
}

pub fn task_on(db: &Connection, task: &TaskRecord) -> Result<(), String> {
    db.execute("INSERT INTO tasks(task_id,mod_id,session_id,status,created_at,finished_at,payload) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(task_id) DO UPDATE SET status=excluded.status,finished_at=excluded.finished_at,payload=excluded.payload", params![task.context.task_id,task.context.mod_id,task.context.session_id,task.status,task.context.created_at,task.finished_at,encode(task)?]).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn save_task(app: &tauri::AppHandle, task: &TaskRecord) -> Result<(), String> { task_on(&database::connect(app)?,task) }
pub fn tasks(app: &tauri::AppHandle, mod_id: &str) -> Result<Vec<TaskRecord>, String> {
    let db = database::connect(app)?;
    let mut query = db.prepare("SELECT payload FROM tasks WHERE mod_id=?1 ORDER BY created_at DESC").map_err(|e| e.to_string())?;
    let rows = query.query_map([mod_id], |r| r.get::<_,String>(0)).map_err(|e| e.to_string())?.map(|r| decode(r.map_err(|e| e.to_string())?)).collect::<Result<Vec<_>,_>>();
    rows
}
pub fn task(app: &tauri::AppHandle, mod_id: &str, task_id: &str) -> Result<TaskRecord, String> {
    let payload = database::connect(app)?.query_row("SELECT payload FROM tasks WHERE task_id=?1 AND mod_id=?2",params![task_id,mod_id],|r| r.get(0)).map_err(|e| e.to_string())?;
    decode(payload)
}
pub fn has_active_tasks(app: &tauri::AppHandle, mod_id: &str) -> Result<bool,String> {
    database::connect(app)?.query_row("SELECT EXISTS(SELECT 1 FROM tasks WHERE mod_id=?1 AND status IN ('preparing','running','stopping'))",[mod_id],|r|r.get(0)).map_err(|e|e.to_string())
}
