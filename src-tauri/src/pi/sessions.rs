use crate::storage::{mods, paths, models::SessionReference};
use std::process::{Command, Stdio};

/// 只修改应用会话标题，不改项目目录、历史或交付版本。
#[tauri::command]
pub fn rename_mod_session(app: tauri::AppHandle, mod_id: String, session_id: String, name: String) -> Result<crate::storage::models::ModProject, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(|c| c.is_control()) { return Err("会话名称需为 1–120 个字符且不含控制字符".into()); }
    (|| {
        use tauri::Manager;
        let state = app.state::<mods::StorageState>();
        let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
        crate::storage::repository::update(&app,&mod_id,|project| {
            let session=project.sessions.iter_mut().find(|s|s.session_id==session_id).ok_or("会话不属于当前项目")?;
            session.name=name.into(); project.updated_at=mods::now()?; Ok(())
        })
    })()
}

#[tauri::command]
pub fn list_mod_sessions(app: tauri::AppHandle, mod_id: String) -> Result<Vec<SessionReference>, String> {
    Ok(mods::read_project(&app, &mod_id)?.sessions)
}

/// readonly 会话使用锁定 SDK 的投影能力，只读取文件，不启动第二执行链。
#[tauri::command]
pub fn read_session_history(app: tauri::AppHandle, mod_id: String, session_id: String) -> Result<serde_json::Value, String> {
    let project = mods::read_project(&app, &mod_id)?;
    let session = project.sessions.iter().find(|s| s.session_id == session_id).ok_or("会话不属于当前项目")?;
    if session.session_file.is_empty() { return Ok(serde_json::json!({ "messages": [] })); }
    let directory = paths::project_dir(&app, &mod_id)?;
    let sessions = directory.join("logs/sessions").canonicalize().map_err(|e| e.to_string())?;
    let file = std::path::Path::new(&session.session_file);
    if !file.exists() { return Ok(serde_json::json!({ "messages": [] })); }
    paths::reject_redirect(file)?;
    let actual = file.canonicalize().map_err(|e| e.to_string())?;
    if !actual.starts_with(&sessions) { return Err("会话文件位于当前项目之外".into()); }
    let resources = super::runtime::resources(&app)?;
    let helper = resources.join("pi-portable-v1.5.0/helpers/read-session.mjs");
    if !helper.is_file() { return Err("历史读取资源尚未准备，请执行 pnpm prepare:runtime".into()); }
    let mut command = Command::new(resources.join("node-v26.12.1-win-x64/node.exe"));
    command.arg(helper).arg(actual).arg(sessions).arg(directory.join("source"))
        .env_clear().env("PI_OFFLINE", "1").env("PI_TELEMETRY", "0")
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
    let result = command.output().map_err(|e| e.to_string())?;
    if !result.status.success() { return Err("pi SDK 无法读取会话历史，请检查会话文件是否损坏".into()); }
    let mut history = serde_json::from_slice(&result.stdout).map_err(|_| "pi SDK 返回了无效的历史数据".to_string())?;
    super::submissions::annotate_history(&crate::storage::database::connect(&app)?, &mod_id, &session_id, &mut history)?;
    Ok(history)
}

#[tauri::command]
pub fn list_mod_tasks(app: tauri::AppHandle, mod_id: String) -> Result<Vec<super::tasks::TaskRecord>, String> {
    mods::read_project(&app, &mod_id)?;
    crate::storage::repository::tasks(&app, &mod_id)
}
