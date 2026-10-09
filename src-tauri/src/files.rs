use crate::{changes::relative_path, storage::paths};
use tauri_plugin_opener::OpenerExt;

/// 验证所有已存在路径组件，避免符号链接把文件定位带出源码目录。
pub fn source_file(app: &tauri::AppHandle, mod_id: &str, relative: &str) -> Result<std::path::PathBuf, String> {
    let relative = relative_path(relative)?;
    let project = paths::project_dir(app, mod_id)?;
    paths::reject_redirect(&paths::mods_root(app)?)?;
    paths::reject_redirect(&project)?;
    let root = project.join("source");
    paths::reject_redirect(&root)?;
    let mut path = root.clone();
    for part in relative.components() { path.push(part); paths::reject_redirect(&path)?; }
    let actual = path.canonicalize().map_err(|e| e.to_string())?;
    if !actual.starts_with(root.canonicalize().map_err(|e| e.to_string())?) { return Err("文件位于源码目录之外".into()); }
    Ok(actual)
}

#[tauri::command]
pub fn reveal_source_file(app: tauri::AppHandle, mod_id: String, relative_path: String) -> Result<(), String> {
    let path = source_file(&app, &mod_id, &relative_path)?;
    app.opener().reveal_item_in_dir(path.to_string_lossy().into_owned()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_message_link(app: tauri::AppHandle, mod_id: String, link: String) -> Result<(), String> {
    if let Ok(url) = tauri::Url::parse(&link) {
        if !matches!(url.scheme(), "https" | "http") || !url.username().is_empty() || url.password().is_some() { return Err("仅允许 HTTP(S) 文档链接和项目相对文件路径".into()); }
        app.opener().open_url(url.to_string(), None::<&str>).map_err(|e| e.to_string())
    } else {
        let path = source_file(&app, &mod_id, &link)?;
        app.opener().reveal_item_in_dir(path.to_string_lossy().into_owned()).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn pick_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog().file().set_title("选择目录").blocking_pick_folder()
        .map(|file| file.into_path().map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string())).transpose()
}

#[tauri::command]
pub fn open_mod_directory(app: tauri::AppHandle, mod_id: String, version_id: Option<String>) -> Result<(), String> {
    crate::storage::mods::read_project(&app, &mod_id)?;
    let directory = if let Some(id) = version_id { crate::artifacts::registry::version_root(&app, &mod_id, &id)? }
        else { crate::artifacts::registry::checked_path(&paths::project_dir(&app, &mod_id)?, "source")? };
    app.opener().open_path(directory.to_string_lossy().into_owned(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_task_logs(app: tauri::AppHandle, mod_id: String, task_id: String) -> Result<(), String> {
    paths::validate_id(&task_id)?;
    crate::storage::mods::read_project(&app, &mod_id)?;
    let directory = crate::artifacts::registry::checked_path(&paths::project_dir(&app, &mod_id)?, &format!("logs/{task_id}"))?;
    app.opener().open_path(directory.to_string_lossy().into_owned(), None::<&str>).map_err(|e| e.to_string())
}

/// 仅导出选定不可变版本，不重新从 source 打包；保存对话框由用户选择位置。
#[tauri::command]
pub async fn export_version(app: tauri::AppHandle, mod_id: String, version_id: String) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let root = crate::artifacts::registry::version_root(&app, &mod_id, &version_id)?;
    let file = crate::artifacts::registry::checked_path(&root, "mod.zip")?;
    let project = crate::storage::mods::read_project(&app, &mod_id)?;
    let version = project.versions.iter().find(|v| v.version_id == version_id).ok_or("版本不存在")?;
    if crate::changes::file_hash(&file)? != version.zip_sha256 { return Err("版本 ZIP 校验失败".into()); }
    let Some(target) = app.dialog().file().set_file_name(format!("mod-{version_id}.zip")).add_filter("ZIP", &["zip"]).blocking_save_file() else { return Ok(None); };
    let target = target.into_path().map_err(|e| e.to_string())?;
    let parent = target.parent().ok_or("导出路径无效")?;
    for ancestor in parent.ancestors().filter(|p| !p.as_os_str().is_empty()) { paths::reject_redirect(ancestor)?; }
    if target.exists() { paths::reject_redirect(&target)?; }
    let mods = paths::mods_root(&app)?.canonicalize().map_err(|e| e.to_string())?;
    if parent.canonicalize().map_err(|e| e.to_string())?.starts_with(&mods) { return Err("不能导出到应用项目目录并覆盖版本或源码".into()); }
    std::fs::copy(file, &target).map_err(|e| e.to_string())?;
    Ok(Some(target.to_string_lossy().into_owned()))
}
