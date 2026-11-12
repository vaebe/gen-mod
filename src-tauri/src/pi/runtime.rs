use std::{path::PathBuf, process::{Command, Stdio}};
use tauri::Manager;

pub fn resources(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../runtime/resources"))
    } else {
        Ok(app.path().resource_dir().map_err(|e| e.to_string())?.join("runtime"))
    }
}

#[derive(serde::Deserialize)]
struct RuntimeLock { node: NodeLock, pi: PiLock }
#[derive(serde::Deserialize)]
struct NodeLock { directory: String }
#[derive(serde::Deserialize)]
struct PiLock { version: String, entry: String }

pub struct RuntimePaths {
    pub node: PathBuf,
    pub cli: PathBuf,
    pub extension: PathBuf,
    pub history: PathBuf,
}

/// 编译时嵌入同一锁文件；三个入口使用对应版本的内置 Node 和资源。
pub fn paths(root: &std::path::Path) -> Result<RuntimePaths, String> {
    let lock: RuntimeLock = serde_json::from_str(include_str!("../../../runtime/runtime-lock.json")).map_err(|e| e.to_string())?;
    let portable = root.join(format!("pi-portable-v{}", lock.pi.version));
    Ok(RuntimePaths {
        node: root.join(lock.node.directory).join("node.exe"),
        cli: portable.join(lock.pi.entry),
        extension: portable.join("extensions/mod-tools.ts"),
        history: portable.join("helpers/read-session.mjs"),
    })
}

/// 明确使用随应用分发的 Node，不回退到全局安装。
pub fn command(app: &tauri::AppHandle, source: &std::path::Path, session_dir: &std::path::Path, config_dir: &std::path::Path) -> Result<Command, String> {
    let root = resources(app)?;
    let paths = paths(&root)?;
    let node = paths.node;
    let entry = paths.cli;
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
