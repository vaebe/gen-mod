use std::path::PathBuf;
use tauri::Manager;

/// 只接受应用生成的 ID，拒绝任意路径及非标准 UUID。
pub fn validate_id(id: &str) -> Result<(), String> {
    let parsed = uuid::Uuid::parse_str(id).map_err(|_| "无效的项目 ID".to_string())?;
    if parsed.to_string() != id {
        return Err("项目 ID 必须使用标准格式".into());
    }
    Ok(())
}

pub fn mods_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("mods"))
}

pub fn project_dir(app: &tauri::AppHandle, id: &str) -> Result<PathBuf, String> {
    validate_id(id)?;
    Ok(mods_root(app)?.join(id))
}

/// 元数据目录不得被重解析点重定向到应用工作区之外。
pub fn reject_redirect(path: &std::path::Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!("不允许符号链接：{}", path.display()));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(format!("不允许重解析点：{}", path.display()));
        }
    }
    Ok(())
}
