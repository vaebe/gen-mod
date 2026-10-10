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
    crate::storage::repository::logs(&app, &mod_id, &session_id)
}
