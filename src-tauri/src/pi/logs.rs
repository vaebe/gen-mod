use crate::storage::{database, repository::{encode, decode}};
use rusqlite::{params, Connection};
use crate::storage::{mods, paths};
use serde::{Deserialize, Serialize};

/// 会话运行摘要跨轮次保存，不包含模型思考正文或凭据。
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogEntry {
    pub timestamp: String,
    pub generation: u64,
    pub task_id: Option<String>,
    pub message: String,
}

/// 读取当前会话日志尾部，限制读取量及返回条数。
#[tauri::command]
pub fn read_session_runtime_logs(app: tauri::AppHandle, mod_id: String, session_id: String) -> Result<Vec<RuntimeLogEntry>, String> {
    paths::validate_id(&session_id)?;
    let project = mods::read_project(&app, &mod_id)?;
    if !project.sessions.iter().any(|s| s.session_id == session_id) { return Err("会话不属于当前项目".into()); }
    crate::pi::logs::logs(&app, &mod_id, &session_id)
}

pub fn log_on(db: &Connection, mod_id: &str, session_id: &str, entry: &RuntimeLogEntry) -> Result<(), String> {
    if db.execute("INSERT INTO runtime_logs(mod_id,session_id,task_id,timestamp,payload) SELECT ?1,?2,?3,?4,?5 WHERE EXISTS(SELECT 1 FROM projects WHERE mod_id=?1 AND archived=0) AND NOT EXISTS(SELECT 1 FROM project_removals WHERE mod_id=?1) AND NOT EXISTS(SELECT 1 FROM file_operations WHERE mod_id=?1 AND kind='project_delete' AND status IN ('pending','requires_review'))",params![mod_id,session_id,entry.task_id,entry.timestamp,encode(entry)?]).map_err(|e| e.to_string())?!=1 {return Err("项目已不可写，未保存迟到日志".into());}
    db.execute("UPDATE sessions SET last_activity_at=?3 WHERE mod_id=?1 AND session_id=?2",params![mod_id,session_id,entry.timestamp]).map_err(|e|e.to_string())?;
    Ok(())
}
pub fn log(app: &tauri::AppHandle, mod_id: &str, session_id: &str, entry: &RuntimeLogEntry) -> Result<(), String> { log_on(&database::connect(app)?,mod_id,session_id,entry) }
pub fn logs(app: &tauri::AppHandle, mod_id: &str, session_id: &str) -> Result<Vec<RuntimeLogEntry>, String> {
    let db = database::connect(app)?;
    let mut query = db.prepare("SELECT payload FROM (SELECT log_id,payload FROM runtime_logs WHERE mod_id=?1 AND session_id=?2 ORDER BY log_id DESC LIMIT 200) ORDER BY log_id").map_err(|e| e.to_string())?;
    let rows = query.query_map(params![mod_id,session_id], |r| r.get::<_,String>(0)).map_err(|e| e.to_string())?.map(|r| decode(r.map_err(|e| e.to_string())?)).collect::<Result<Vec<_>,_>>();
    rows
}
