use super::{models::{DevelopmentStatus, ModProject}, paths};
use std::{fs, io::Write, path::Path, sync::Mutex};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// 管理操作与版本文件操作的短期互斥；不会锁住整轮模型任务。
#[derive(Default)]
pub struct StorageState(pub Mutex<()>);

pub fn now() -> Result<String, String> {
    OffsetDateTime::now_utc().format(&Rfc3339).map_err(|e| e.to_string())
}

/// 同目录临时文件 + 原子替换，Windows 也不先删除已有元数据。
pub fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path.parent().ok_or("元数据路径缺少目录")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut temp, value).map_err(|e| e.to_string())?;
    temp.write_all(b"\n").map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn read_project(app: &tauri::AppHandle, id: &str) -> Result<ModProject, String> {
    super::repository::read(app, id)
}

fn validate_name(name: &str) -> Result<String, String> {
    let value = name.trim();
    if value.is_empty() || value.chars().count() > 120 {
        return Err("项目名称应为 1–120 个字符".into());
    }
    Ok(value.to_string())
}

#[tauri::command]
pub fn create_mod(app: tauri::AppHandle, state: tauri::State<StorageState>, name: String, game_skill_name: String, requirement: String) -> Result<ModProject, String> {
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    let name = validate_name(&name)?;
    let selected = crate::skills::registry::resolve(&app, &game_skill_name)?;
    let game_id = selected.last().and_then(|v| v.definition.game_id.clone()).ok_or("能力缺少游戏标识")?;
    let id = uuid::Uuid::new_v4().to_string();
    let root = paths::mods_root(&app)?;
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    paths::reject_redirect(&root)?;
    let directory = paths::project_dir(&app, &id)?;
    fs::create_dir(&directory).map_err(|e| e.to_string())?;
    let project = ModProject {
        mod_id: id, name, revision: 0,
        game_id, game_skill_name,
        draft_requirement: requirement, updated_at: now()?,
        development_status: DevelopmentStatus::Draft, latest_version_id: None,
        last_deployment_directory: None,
        versions: vec![], installations: vec![], installed_targets: vec![], sessions: vec![],
    };
    let result = (|| {
        for child in ["source", "artifacts", "logs"] {
            fs::create_dir(directory.join(child)).map_err(|e| e.to_string())?;
        }
        super::repository::create(&app, &project)
    })();
    if let Err(error) = result {
        // 仅清理由本次调用创建的 UUID 目录，不触碰已有项目。
        let actual = directory.canonicalize().map_err(|e| e.to_string())?;
        if actual.starts_with(root.canonicalize().map_err(|e| e.to_string())?) { let _ = fs::remove_dir_all(actual); }
        return Err(error);
    }
    Ok(project)
}

#[tauri::command]
pub fn list_mods(app: tauri::AppHandle, state: tauri::State<StorageState>) -> Result<Vec<ModProject>, String> {
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    super::repository::list(&app)
}

#[tauri::command]
pub fn get_mod(app: tauri::AppHandle, state: tauri::State<StorageState>, mod_id: String) -> Result<ModProject, String> {
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    let mut project = read_project(&app, &mod_id)?;
    crate::skills::registry::refresh_sessions(&app, &mut project)?;
    if let Ok(views) = crate::skills::registry::resolve(&app, &project.game_skill_name) {
        if let Some(id) = views.last().and_then(|v| v.definition.game_id.clone()) { project.game_id = id; }
    }
    super::repository::update(&app,&mod_id,|current| {
        current.game_id=project.game_id.clone();
        for session in &mut current.sessions { if let Some(snapshot)=project.sessions.iter().find(|s|s.session_id==session.session_id) {session.read_only=snapshot.read_only;} }
        Ok(())
    })
}

#[tauri::command]
pub fn rename_mod(app: tauri::AppHandle, state: tauri::State<StorageState>, mod_id: String, name: String) -> Result<ModProject, String> {
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    let name=validate_name(&name)?;
    super::repository::update(&app,&mod_id,|project| {project.name=name;project.updated_at=now()?;Ok(())})
}

#[tauri::command]
pub fn save_mod_draft(app: tauri::AppHandle, state: tauri::State<StorageState>, mod_id: String, session_id: Option<String>, requirement: String, expected_requirement: Option<String>) -> Result<ModProject, String> {
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    let mut project = read_project(&app, &mod_id)?;
    if let Some(id) = session_id {
        if let Some(expected) = expected_requirement {
            // 发送期间可能切换并编辑草稿，只清理仍与已发送内容一致的原会话。
            let db = super::database::connect(&app)?;
            db.execute("UPDATE sessions SET draft=?3 WHERE mod_id=?1 AND session_id=?2 AND deleted=0 AND draft=?4",rusqlite::params![mod_id,id,requirement,expected]).map_err(|e|e.to_string())?;
        } else {
            super::repository::save_preferences(&app,&mod_id,&id,Some(&requirement),None)?;
        }
        return Ok(project);
    }
    if !project.sessions.is_empty() { return Err("保存草稿必须指定会话".into()); }
    project.draft_requirement = requirement;
    project.updated_at = now()?;
    crate::storage::repository::save(&app, &mut project)?;
    Ok(project)
}
