pub mod models;
pub mod repository;
pub mod evidence;
pub mod bootstrap;
#[cfg(test)]
mod tests;
use crate::storage::{database, paths};
use models::*;
use sha2::{Digest, Sha256};
pub fn validate(value: &ProjectContext) -> Result<(), String> {
    if value.prerequisites.len() > 100
        || value.confirmed_requirements.len() > 100
        || value.reference_sets.len() > 16
    {
        return Err("项目环境条目过多".into());
    }
    if value
        .confirmed_requirements
        .iter()
        .any(|s| s.trim().is_empty() || s.len() > 4000)
        || value.prerequisites.iter().any(|p| {
            p.name.trim().is_empty()
                || p.name.len() > 200
                || p.version.as_ref().is_some_and(|v| v.len() > 200)
        })
    {
        return Err("环境条目为空或过长".into());
    }
    if let Some(v) = &value.game_version {
        if v.value.trim().is_empty()
            || v.value.len() > 200
            || !matches!(v.source.as_str(), "user" | "inspection")
        {
            return Err("游戏版本信息无效".into());
        }
    }
    Ok(())
}
pub fn fingerprint_references(value: &mut ProjectContext) -> Result<(), String> {
    let mut ids = std::collections::BTreeSet::new();
    let mut total = 0u64;
    for set in &mut value.reference_sets {
        if set.id.is_empty() || !ids.insert(set.id.clone()) || set.files.len() > 500 {
            return Err("参考文件组 ID 重复或文件过多".into());
        }
        let root = std::path::Path::new(&set.directory);
        paths::reject_redirect(root)?;
        if !root.is_dir() {
            return Err("参考目录不存在".into());
        }
        let mut files = std::collections::BTreeSet::new();
        for file in &mut set.files {
            if !files.insert(file.path.to_lowercase()) {
                return Err("参考文件重复".into());
            }
            let path = crate::artifacts::registry::checked_path(root, &file.path)?;
            let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
            total = total.checked_add(size).ok_or("参考文件过大")?;
            if size > 64 * 1024 * 1024 || total > 256 * 1024 * 1024 {
                return Err("参考文件指纹范围超过 64 MiB/文件或 256 MiB/总量".into());
            }
            file.sha256 = format!(
                "{:x}",
                Sha256::digest(std::fs::read(path).map_err(|e| e.to_string())?)
            );
        }
    }
    Ok(())
}
pub fn check_task_references(
    app: &tauri::AppHandle,
    task: &crate::pi::tasks::TaskContext,
) -> Result<(), String> {
    if task.context_snapshot_id.is_none() {
        return Ok(());
    }
    let original =
        repository::snapshot_for_task_on(&database::connect(app)?, &task.mod_id, &task.task_id)?
            .context;
    let mut current = original.clone();
    fingerprint_references(&mut current)?;
    if original.reference_sets != current.reference_sets {
        return Err("参考文件已变化，请保存最新项目环境并开始新任务".into());
    }
    Ok(())
}
#[tauri::command]
pub fn get_project_context(
    app: tauri::AppHandle,
    mod_id: String,
) -> Result<ProjectContext, String> {
    crate::storage::repository::read(&app, &mod_id)?;
    repository::read_on(&database::connect(&app)?, &mod_id)
}
