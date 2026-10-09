use crate::storage::{mods, paths};
use serde::{Deserialize, Serialize};
use std::{fs, io::{BufRead, BufReader, Seek, SeekFrom}};

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
    let root = paths::project_dir(&app, &mod_id)?.join("logs/sessions");
    paths::reject_redirect(&root)?;
    let path = root.join(format!("{session_id}.runtime.jsonl"));
    if !path.exists() { return Ok(vec![]); }
    paths::reject_redirect(&path)?;
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    let offset = size.saturating_sub(512 * 1024);
    file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    if offset > 0 { let mut partial = String::new(); reader.read_line(&mut partial).map_err(|e| e.to_string())?; }
    let mut result = std::collections::VecDeque::new();
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        // 写入尚未完成的一行稍后刷新即可，不能阻断已经完整的日志。
        if let Ok(entry) = serde_json::from_str::<RuntimeLogEntry>(&line) {
            result.push_back(entry);
            if result.len() > 200 { result.pop_front(); }
        }
    }
    Ok(result.into_iter().collect())
}
