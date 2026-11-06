use std::path::PathBuf;
use tauri::Manager;

/// 开发与安装包使用相同的资源布局，不依赖当前工作目录。
pub fn root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
    } else {
        app.path().resource_dir().map_err(|error| error.to_string())
    }
}
