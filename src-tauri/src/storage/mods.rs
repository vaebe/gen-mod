use super::{models::{DevelopmentStatus, ModProject}, paths};
use std::{fs, io::Write, path::Path, sync::Mutex};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// 首期单活动任务；共用写锁防止草稿与后续产物登记丢失彼此更新。
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
    let directory = paths::project_dir(app, id)?;
    paths::reject_redirect(&paths::mods_root(app)?)?;
    paths::reject_redirect(&directory)?;
    let path = directory.join("mod.json");
    paths::reject_redirect(&path)?;
    let value: ModProject = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("项目元数据损坏（{id}）：{e}"))?;
    if value.mod_id != id {
        return Err("项目目录与元数据 ID 不一致".into());
    }
    Ok(value)
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
        mod_id: id, name,
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
        write_json(&directory.join("mod.json"), &project)
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
    let root = paths::mods_root(&app)?;
    if !root.exists() { return Ok(vec![]); }
    paths::reject_redirect(&root)?;
    let mut projects = vec![];
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let id = entry.file_name().to_string_lossy().into_owned();
        paths::validate_id(&id)?;
        projects.push(read_project(&app, &id)?);
    }
    projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(projects)
}

#[tauri::command]
pub fn get_mod(app: tauri::AppHandle, state: tauri::State<StorageState>, mod_id: String) -> Result<ModProject, String> {
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    let mut project = read_project(&app, &mod_id)?;
    crate::skills::registry::refresh_sessions(&app, &mut project)?;
    if let Ok(views) = crate::skills::registry::resolve(&app, &project.game_skill_name) {
        if let Some(id) = views.last().and_then(|v| v.definition.game_id.clone()) { project.game_id = id; }
    }
    write_json(&paths::project_dir(&app, &mod_id)?.join("mod.json"), &project)?;
    Ok(project)
}

#[tauri::command]
pub fn rename_mod(app: tauri::AppHandle, state: tauri::State<StorageState>, mod_id: String, name: String) -> Result<ModProject, String> {
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    let mut project = read_project(&app, &mod_id)?;
    project.name = validate_name(&name)?;
    project.updated_at = now()?;
    write_json(&paths::project_dir(&app, &mod_id)?.join("mod.json"), &project)?;
    Ok(project)
}

#[tauri::command]
pub fn save_mod_draft(app: tauri::AppHandle, state: tauri::State<StorageState>, mod_id: String, requirement: String) -> Result<ModProject, String> {
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    let mut project = read_project(&app, &mod_id)?;
    project.draft_requirement = requirement;
    project.updated_at = now()?;
    write_json(&paths::project_dir(&app, &mod_id)?.join("mod.json"), &project)?;
    Ok(project)
}
