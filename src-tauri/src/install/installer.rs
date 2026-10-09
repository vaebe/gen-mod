use super::models::{InstallPreview, PreviewEntry};
use crate::{artifacts::registry::{checked_path, manifest, version_root}, changes::{file_hash, relative_path}, storage::{mods, paths, models::{InstallationRecord, InstallationStatus, RollbackStatus, InstalledTarget, TargetState}}};
use std::{collections::BTreeMap, fs, path::{Path, PathBuf}, sync::Mutex};

/// 预览令牌只存在原生内存；执行时重新核对，不能凭前端伪造清单安装。
#[derive(Default)]
pub struct InstallState(pub Mutex<BTreeMap<String, InstallPreview>>);

fn target_root(app: &tauri::AppHandle, target: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(target);
    if !path.is_absolute() || !path.is_dir() { return Err("请选择存在的目标目录".into()); }
    for ancestor in path.ancestors().filter(|p| !p.as_os_str().is_empty()) { paths::reject_redirect(ancestor)?; }
    let actual = path.canonicalize().map_err(|e| e.to_string())?;
    let mods = paths::mods_root(app)?.canonicalize().map_err(|e| e.to_string())?;
    if actual.starts_with(&mods) || mods.starts_with(&actual) { return Err("安装目标不能覆盖应用项目目录".into()); }
    Ok(actual)
}
fn target_file(root: &Path, name: &str) -> Result<PathBuf, String> {
    let relative = relative_path(name)?;
    let mut path = root.to_path_buf();
    let components: Vec<_> = relative.components().collect();
    for (index, component) in components.iter().enumerate() {
        path.push(component);
        if fs::symlink_metadata(&path).is_ok() {
            paths::reject_redirect(&path)?;
            if index + 1 < components.len() && !path.is_dir() { return Err("安装目标父路径不是目录".into()); }
            if index + 1 == components.len() && !path.is_file() { return Err("安装目标已有同名目录".into()); }
        }
    }
    Ok(path)
}
fn calculate(app: &tauri::AppHandle, mod_id: &str, version_id: &str, target: &str) -> Result<InstallPreview, String> {
    let root = version_root(app, mod_id, version_id)?;
    let target = target_root(app, target)?;
    let project = mods::read_project(app, mod_id)?;
    let version = project.versions.iter().find(|v| v.version_id == version_id).ok_or("版本不存在")?;
    let mut entries = vec![];
    for entry in manifest(&root)?.entries {
        let source = checked_path(&root.join("files"), &entry.source)?;
        let expected = version.files.iter().find(|f| f.path == entry.source).ok_or("安装清单引用版本外文件")?;
        if file_hash(&source)? != expected.sha256 { return Err("版本文件校验失败，不能安装".into()); }
        let destination = target_file(&target, &entry.target)?;
        let overwrite = destination.exists();
        entries.push(PreviewEntry { source: source.to_string_lossy().into_owned(), target: destination.to_string_lossy().into_owned(), overwrite, before_hash: if overwrite { Some(file_hash(&destination)?) } else { None } });
    }
    Ok(InstallPreview { preview_id: uuid::Uuid::new_v4().to_string(), mod_id: mod_id.into(), version_id: version_id.into(), target_directory: target.to_string_lossy().into_owned(), entries })
}
#[tauri::command]
pub fn preview_install(app: tauri::AppHandle, storage: tauri::State<mods::StorageState>, state: tauri::State<InstallState>, mod_id: String, version_id: String, target_directory: String) -> Result<InstallPreview, String> {
    let _lock = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    let preview = calculate(&app, &mod_id, &version_id, &target_directory)?;
    // 记录最近通过校验的选择，即使用户之后取消安装也可再次复用。
    let mut project = mods::read_project(&app, &mod_id)?;
    project.last_deployment_directory = Some(preview.target_directory.clone());
    project.updated_at = mods::now()?;
    mods::write_json(&paths::project_dir(&app, &mod_id)?.join("mod.json"), &project)?;
    let mut previews = state.0.lock().map_err(|_| "安装预览锁异常")?;
    previews.clear(); // 单用户确认流程，旧预览失效。
    previews.insert(preview.preview_id.clone(), preview.clone());
    Ok(preview)
}

/// 重新校验预览后直接覆盖；不创建备份，部分复制失败时将目标标为不确定。
#[tauri::command]
pub fn install_version(app: tauri::AppHandle, storage: tauri::State<mods::StorageState>, state: tauri::State<InstallState>, preview_id: String) -> Result<InstallationRecord, String> {
    let _lock = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    let preview = state.0.lock().map_err(|_| "安装预览锁异常")?.remove(&preview_id).ok_or("安装预览已失效，请重新预览")?;
    let current = calculate(&app, &preview.mod_id, &preview.version_id, &preview.target_directory)?;
    if serde_json::to_value(&current.entries).map_err(|e| e.to_string())? != serde_json::to_value(&preview.entries).map_err(|e| e.to_string())? { return Err("目标文件在预览后发生变化，请重新确认".into()); }
    let project_root = paths::project_dir(&app, &preview.mod_id)?;
    let mut record = InstallationRecord { installation_id: uuid::Uuid::new_v4().to_string(), version_id: preview.version_id.clone(), target_directory: preview.target_directory.clone(), attempted_at: mods::now()?, status: InstallationStatus::Failed, rollback: RollbackStatus::NotNeeded, backup_directory: None, error: None };
    let mut touched = false;
    let result = (|| -> Result<(), String> {
        for entry in &preview.entries {
            let path = Path::new(&entry.target);
            let parent = path.parent().ok_or("目标父目录无效")?;
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            // 每次复制重新核对路径，保留原有目录重定向保护。
            target_file(Path::new(&preview.target_directory), &path.strip_prefix(&preview.target_directory).map_err(|_| "目标目录越界")?.to_string_lossy().replace('\\', "/"))?;
            // 复制失败也可能截断目标；从第一次复制起不再相信之前的版本状态。
            touched = true;
            fs::copy(&entry.source, path).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        record.error = Some(error);
        if touched { record.rollback = RollbackStatus::NotAttempted; }
    } else { record.status = InstallationStatus::Succeeded; }
    let mut project = mods::read_project(&app, &preview.mod_id)?;
    if matches!(record.status, InstallationStatus::Succeeded) {
        project.installed_targets.retain(|target| target.target_directory != preview.target_directory);
        project.installed_targets.push(InstalledTarget { target_directory: preview.target_directory.clone(), version_id: Some(preview.version_id.clone()), state: TargetState::Known });
    } else if touched {
        project.installed_targets.retain(|target| target.target_directory != preview.target_directory);
        project.installed_targets.push(InstalledTarget { target_directory: preview.target_directory.clone(), version_id: None, state: TargetState::Uncertain });
    }
    project.installations.push(record.clone()); project.updated_at = mods::now()?;
    if let Err(error) = mods::write_json(&project_root.join("mod.json"), &project) {
        // 文件不回退；尝试保存失败记录，避免旧的已安装标记冒充本次成功。
        record.status = InstallationStatus::Failed;
        record.rollback = if touched { RollbackStatus::NotAttempted } else { RollbackStatus::NotNeeded };
        record.error = Some(format!("安装记录保存失败：{error}；本次未备份，请核对目标文件"));
        if touched {
            project.installed_targets.retain(|target| target.target_directory != preview.target_directory);
            project.installed_targets.push(InstalledTarget { target_directory: preview.target_directory.clone(), version_id: None, state: TargetState::Uncertain });
        }
        if let Some(last) = project.installations.last_mut() { *last = record.clone(); }
        mods::write_json(&project_root.join("mod.json"), &project).map_err(|error| format!("{}；失败记录也无法保存：{error}", record.error.as_deref().unwrap_or("安装失败")))?;
    }
    Ok(record)
}