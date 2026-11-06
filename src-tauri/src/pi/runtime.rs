use std::{path::PathBuf, process::{Command, Stdio}};
use tauri::Manager;

pub fn resources(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../runtime/resources"))
    } else {
        Ok(app.path().resource_dir().map_err(|e| e.to_string())?.join("runtime"))
    }
}

/// 明确使用随应用分发的 Node，不回退到全局安装。
pub fn command(app: &tauri::AppHandle, source: &std::path::Path, session_dir: &std::path::Path, config_dir: &std::path::Path) -> Result<Command, String> {
    let root = resources(app)?;
    let node = root.join("node-v26.12.1-win-x64/node.exe");
    let entry = root.join("pi-portable-v1.5.0/node_modules/@earendil-works/pi-coding-agent/dist/bundle/cli.js");
    if !node.is_file() || !entry.is_file() { return Err("内置运行时未准备，请执行 pnpm prepare:runtime".into()); }
    let mut command = Command::new(node);
    command.arg(entry).args(["--mode", "rpc", "--offline", "--no-approve", "--no-skills", "--no-extensions", "--no-mcp", "--no-prompt-templates", "--no-themes", "--no-context-files"]).arg("--session-dir").arg(session_dir)
        .current_dir(source).env("PI_CODING_AGENT_DIR", config_dir)
        .env("PI_OFFLINE", "1").env("PI_TELEMETRY", "0")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    Ok(command)
}
