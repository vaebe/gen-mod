use tauri::Manager;
use crate::{changes::relative_path, storage::paths};
use tauri_plugin_opener::OpenerExt;
use serde::Serialize;
use std::{fs, io::Read, path::PathBuf};

/// 目录列表不递归读取内容，展开文件夹时再加载下一层。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceEntry { name: String, path: String, kind: &'static str }

/// 文本为 UTF-8，限制与差异预览一致；不可预览项仍可定位到磁盘。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePreview {
    path: String,
    size: u64,
    text: Option<String>,
    image_mime_type: Option<&'static str>,
    preview_limit: u64,
    unavailable_reason: Option<&'static str>,
}

const IMAGE_LIMIT: u64 = 10 * 1024 * 1024;

/// 根据文件头识别浏览器支持的位图，避免把任意二进制当作图片。
fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") { Some("image/png") }
    else if bytes.starts_with(b"\xff\xd8\xff") { Some("image/jpeg") }
    else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") { Some("image/gif") }
    else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP".as_slice()) { Some("image/webp") }
    else if bytes.starts_with(b"BM") { Some("image/bmp") }
    else if bytes.starts_with(b"\x00\x00\x01\x00") { Some("image/x-icon") }
    else { None }
}

/// 空相对路径仅用于源码根目录，其余路径复用现有越界和重解析点检查。
fn source_directory(app: &tauri::AppHandle, mod_id: &str, relative: &str) -> Result<PathBuf, String> {
    let directory = if relative.is_empty() {
        let project = paths::project_dir(app, mod_id)?;
        paths::reject_redirect(&paths::mods_root(app)?)?;
        paths::reject_redirect(&project)?;
        let root = project.join("source");
        paths::reject_redirect(&root)?;
        root
    } else { source_file(app, mod_id, relative)? };
    if !directory.is_dir() { return Err("所选路径不是源码目录".into()); }
    Ok(directory)
}

/// 浏览整个 source，包括未变更文件；禁止跟随链接，不依赖任务基准。
#[tauri::command]
pub fn list_source_directory(app: tauri::AppHandle, mod_id: String, relative_path: String) -> Result<Vec<SourceEntry>, String> {
    let directory = source_directory(&app, &mod_id, &relative_path)?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().into_string().map_err(|_| "文件名不是有效的 UTF-8")?;
        let path = if relative_path.is_empty() { name.clone() } else { format!("{relative_path}/{name}") };
        let metadata = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
        let kind = if crate::changes::relative_path(&path).is_err() || paths::reject_redirect(&entry.path()).is_err() { "blocked" }
            else if metadata.is_dir() { "directory" }
            else if metadata.is_file() { "file" }
            else { "blocked" };
        entries.push(SourceEntry { name, path, kind });
    }
    entries.sort_by(|a, b| {
        (a.kind != "directory").cmp(&(b.kind != "directory"))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(entries)
}

/// 有界读取当前磁盘源码，文件在读取期间增长也不会突破内存上限。
#[tauri::command]
pub fn read_source_file(app: tauri::AppHandle, mod_id: String, relative_path: String) -> Result<SourcePreview, String> {
    const LIMIT: u64 = 1024 * 1024;
    let path = source_file(&app, &mod_id, &relative_path)?;
    if !path.is_file() { return Err("所选路径不是普通源码文件".into()); }
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() { return Err("所选路径不是普通源码文件".into()); }
    let mut bytes = Vec::new();
    (&mut file).take(16).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    let image_mime_type = image_mime(&bytes);
    let mut preview = SourcePreview { path: relative_path, size: metadata.len(), text: None, image_mime_type, preview_limit: if image_mime_type.is_some() { IMAGE_LIMIT } else { LIMIT }, unavailable_reason: None };
    if image_mime_type.is_some() {
        if metadata.len() > IMAGE_LIMIT { preview.unavailable_reason = Some("tooLarge"); }
        return Ok(preview);
    }
    if metadata.len() > LIMIT { preview.unavailable_reason = Some("tooLarge"); return Ok(preview); }
    file.take(LIMIT + 1 - bytes.len() as u64).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    preview.size = bytes.len() as u64;
    if preview.size > LIMIT { preview.unavailable_reason = Some("tooLarge"); }
    else if bytes.contains(&0) { preview.unavailable_reason = Some("binary"); }
    else {
        match String::from_utf8(bytes) {
            Ok(text) => { preview.text = Some(text); }
            Err(_) => { preview.unavailable_reason = Some("encoding"); }
        }
    }
    Ok(preview)
}

/// 只返回经源码路径校验的位图字节；有界读取后再次确认图片类型。
#[tauri::command]
pub fn read_source_image(app: tauri::AppHandle, mod_id: String, relative_path: String) -> Result<tauri::ipc::Response, String> {
    let path = source_file(&app, &mod_id, &relative_path)?;
    if !path.is_file() { return Err("所选路径不是普通图片文件".into()); }
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() { return Err("所选路径不是普通图片文件".into()); }
    if metadata.len() > IMAGE_LIMIT { return Err("图片超过 10 MiB，无法预览".into()); }
    let mut bytes = Vec::new();
    file.take(IMAGE_LIMIT + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > IMAGE_LIMIT { return Err("图片超过 10 MiB，无法预览".into()); }
    if image_mime(&bytes).is_none() { return Err("文件不是支持的位图格式".into()); }
    Ok(tauri::ipc::Response::new(bytes))
}

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
    let storage=app.state::<crate::storage::mods::StorageState>();
    let _lock=storage.0.lock().map_err(|_|"版本文件操作锁异常")?;
    let root=crate::artifacts::registry::version_root(&app,&mod_id,&version_id)?;
    let file=crate::artifacts::registry::checked_path(&root,"mod.zip")?;
    if crate::changes::file_hash(&file)? != version.zip_sha256 { return Err("版本 ZIP 校验失败".into()); }
    let target = target.into_path().map_err(|e| e.to_string())?;
    let parent = target.parent().ok_or("导出路径无效")?;
    for ancestor in parent.ancestors().filter(|p| !p.as_os_str().is_empty()) { paths::reject_redirect(ancestor)?; }
    if target.exists() { paths::reject_redirect(&target)?; }
    let mods = paths::mods_root(&app)?.canonicalize().map_err(|e| e.to_string())?;
    if parent.canonicalize().map_err(|e| e.to_string())?.starts_with(&mods) { return Err("不能导出到应用项目目录并覆盖版本或源码".into()); }
    std::fs::copy(file, &target).map_err(|e| e.to_string())?;
    Ok(Some(target.to_string_lossy().into_owned()))
}
