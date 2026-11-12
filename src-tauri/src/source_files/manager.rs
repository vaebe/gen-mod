use super::*;
use std::fs;
use tauri_plugin_opener::OpenerExt;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransferItem {
    source: String,
    target: String,
    decision: Option<String>,
}

fn plain(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() { paths::reject_redirect(ancestor)?; }
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() && !metadata.is_dir() { return Err("不支持链接或特殊文件".into()); }
    Ok(())
}

fn child(base: &str, name: &str) -> String {
    if base.is_empty() { name.into() } else { format!("{base}/{name}") }
}

fn item(source: &str, target: &str, status: &'static str, message: Option<String>) -> SourceMutationItem {
    SourceMutationItem { source: source.into(), target: target.into(), status, message }
}

// 每个文件先写临时文件，再提交；失败不先删除已有目标。
fn copy_file(from: &Path, target: &Path, replace: bool) -> Result<(), String> {
    plain(from)?;
    let parent = target.parent().ok_or("目标目录不存在")?;
    plain(parent)?;
    let temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    let before = changes::file_hash(from)?;
    fs::copy(from, temporary.path()).map_err(|e| e.to_string())?;
    if before != changes::file_hash(temporary.path())? || before != changes::file_hash(from)? {
        return Err("复制期间源文件发生变化，请重试".into());
    }
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    if replace {
        plain(target)?;
        if !target.is_file() { return Err("不能用文件替换目录".into()); }
        temporary.persist(target).map_err(|e| e.to_string())?;
    } else {
        temporary.persist_noclobber(target).map_err(|e| e.to_string())?;
    }
    Ok(())
}

struct Transfer<'a> {
    root: &'a Path,
    exclusions: &'a [String],
    external_source: bool,
    external_target: bool,
    moving: bool,
    results: Vec<SourceMutationItem>,
}

impl Transfer<'_> {
    // 冲突逐项解决后，仅清理已搬空且有对应目标的源目录。
    fn finish_move(&mut self, source: &str, target: &str) -> Result<(), String> {
        let from = checked_path(self.root, source, self.exclusions)?;
        if !from.exists() || !from.is_dir() { return Ok(()); }
        plain(&from)?;
        let destination = self.target_path(target)?;
        if !destination.is_dir() { return Ok(()); }
        plain(&destination)?;
        let actual = from.canonicalize().map_err(|e| e.to_string())?;
        let destination_actual = destination.canonicalize().map_err(|e| e.to_string())?;
        if destination_actual == actual { return Ok(()); }
        if destination_actual.starts_with(&actual) { return Err("不能移动到自身及其子目录".into()); }
        for entry in fs::read_dir(&from).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().into_string().map_err(|_| "文件名不是 UTF-8")?;
            if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                self.finish_move(&child(source, &name), &child(target, &name))?;
            }
        }
        if fs::read_dir(&from).map_err(|e| e.to_string())?.next().is_none() {
            fs::remove_dir(&from).map_err(|e| e.to_string())?;
            self.results.push(item(source, target, "moved", None));
        }
        Ok(())
    }

    fn source_path(&self, path: &str) -> Result<PathBuf, String> {
        let source = if self.external_source { PathBuf::from(path) } else { checked_path(self.root, path, self.exclusions)? };
        if !self.external_source && path.is_empty() { return Err("不能操作整个工作目录".into()); }
        plain(&source)?;
        Ok(source)
    }

    fn target_path(&self, path: &str) -> Result<PathBuf, String> {
        if path.is_empty() { return Err("目标文件名不能为空".into()); }
        if !self.external_target { return checked_path(self.root, path, self.exclusions); }
        let target = PathBuf::from(path);
        let parent = target.parent().ok_or("目标目录不存在")?;
        plain(parent)?;
        if target.exists() { plain(&target)?; }
        Ok(target)
    }

    fn execute(&mut self, source: &str, requested: &str, decision: &str) -> Result<bool, String> {
        let from = self.source_path(source)?;
        let mut relative = requested.to_owned();
        let mut target = self.target_path(&relative)?;
        let actual = from.canonicalize().map_err(|e| e.to_string())?;
        let parent = target.parent().ok_or("目标目录不存在")?.canonicalize().map_err(|e| e.to_string())?;
        let exists = target.try_exists().map_err(|e| e.to_string())?;
        let same_entry = exists && target.canonicalize().map_err(|e| e.to_string())? == actual;
        if from.is_dir() && parent.starts_with(&actual) { return Err("不能把目录复制或移动到自身及其子目录".into()); }
        if self.moving && same_entry {
            self.results.push(item(source, requested, "skipped", Some("文件已在目标位置".into())));
            return Ok(false);
        }
        // 原位置复制整个目录先处理冲突，不能把原目录当成合并目标。
        if exists && (same_entry || !(from.is_dir() && target.is_dir())) {
            match decision {
                "skip" => { self.results.push(item(source, &relative, "skipped", None)); return Ok(false); }
                "keep" => {
                    let name = target.file_name().and_then(|n| n.to_str()).ok_or("文件名无效")?.to_owned();
                    let base = Path::new(&name);
                    let stem = if from.is_dir() { name.as_str() } else { base.file_stem().and_then(|n| n.to_str()).unwrap_or(&name) };
                    let extension = if from.is_dir() { None } else { base.extension().and_then(|n| n.to_str()) };
                    let directory = requested.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
                    let mut number = 1;
                    loop {
                        let name = format!("{stem} ({number}){}", extension.map(|e| format!(".{e}")).unwrap_or_default());
                        relative = child(directory, &name);
                        target = self.target_path(&relative)?;
                        if !target.try_exists().map_err(|e| e.to_string())? { break; }
                        number += 1;
                    }
                }
                "replace" if from.is_file() && target.is_file() && !same_entry => {},
                _ => {
                    self.results.push(item(source, &relative, "conflict", Some(if from.is_file() && target.is_file() && !same_entry { "同名文件已存在" } else { "类型不同或源目标相同，请保留两份或跳过" }.into())));
                    return Ok(false);
                }
            }
        }
        if from.is_dir() {
            if !target.exists() { fs::create_dir(&target).map_err(|e| e.to_string())?; }
            let mut complete = true;
            for entry in fs::read_dir(&from).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let name = entry.file_name().into_string().map_err(|_| "文件名不是 UTF-8")?;
                let nested_source = child(source, &name);
                let nested_target = child(&relative, &name);
                match self.execute(&nested_source, &nested_target, decision) {
                    Ok(done) => complete &= done,
                    Err(error) => { complete = false; self.results.push(item(&nested_source, &nested_target, "failed", Some(error))); }
                }
            }
            if self.moving && complete { fs::remove_dir(&from).map_err(|e| e.to_string())?; }
            if complete { self.results.push(item(source, &relative, if self.moving { "moved" } else { "copied" }, None)); }
            return Ok(complete);
        }
        let replace = target.exists() && decision == "replace";
        if self.moving && !replace {
            io::move_entry_no_replace(&from, &target)?;
        } else {
            let hash = changes::file_hash(&from)?;
            copy_file(&from, &target, replace)?;
            if self.moving {
                if changes::file_hash(&from)? != hash { return Err("目标已复制，但源文件发生变化，保留源文件".into()); }
                fs::remove_file(&from).map_err(|e| format!("目标已复制，源文件未移除：{e}"))?;
            }
        }
        self.results.push(item(source, &relative, if self.moving { "moved" } else { "copied" }, None));
        Ok(true)
    }
}

#[tauri::command]
pub async fn transfer_source_entries(app: tauri::AppHandle, mod_id: String, operation: String, items: Vec<TransferItem>, finalize: Option<Vec<TransferItem>>) -> Result<SourceMutationResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if !matches!(operation.as_str(), "copy" | "move" | "import" | "export") { return Err("未知文件操作".into()); }
        let reservation = app.state::<Workspaces>().try_reserve_native(&mod_id, NativeOperation::SourceChange)?;
        let (root, exclusions) = project_input(&app, &mod_id)?;
        let before = changes::source_fingerprint(&root, &exclusions)?;
        let mods_root = paths::mods_root(&app)?.canonicalize().map_err(|e| e.to_string())?;
        let mut transfer = Transfer { root: &root, exclusions: &exclusions, external_source: operation == "import", external_target: operation == "export", moving: operation == "move", results: vec![] };
        for entry in items {
            let result = (|| -> Result<bool, String> {
                if operation == "export" {
                    let target = Path::new(&entry.target);
                    let parent = target.parent().ok_or("导出目标无效")?.canonicalize().map_err(|e| e.to_string())?;
                    if parent.starts_with(&mods_root) || mods_root.starts_with(&parent.join(target.file_name().ok_or("导出文件名无效")?)) { return Err("不能导出到应用项目目录或其父目录并覆盖项目".into()); }
                }
                transfer.execute(&entry.source.replace('\\', "/"), &entry.target.replace('\\', "/"), entry.decision.as_deref().unwrap_or("ask"))
            })();
            if let Err(error) = result { transfer.results.push(item(&entry.source, &entry.target, "failed", Some(error))); }
        }
        if operation == "move" {
            for entry in finalize.unwrap_or_default() {
                if entry.source.is_empty() { continue; }
                if let Err(error) = transfer.finish_move(&entry.source, &entry.target) {
                    transfer.results.push(item(&entry.source, &entry.target, "failed", Some(error)));
                }
            }
        }
        finish(&app, &mod_id, &root, &exclusions, &reservation, uuid::Uuid::new_v4().to_string(), before, transfer.results)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_source_file(app: tauri::AppHandle, mod_id: String, parent_directory: String, name: String) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let reservation = app.state::<Workspaces>().try_reserve_native(&mod_id, NativeOperation::SourceChange)?;
        let (root, exclusions) = project_input(&app, &mod_id)?;
        let (relative, target) = target_path(&root, &parent_directory, &name, &exclusions)?;
        fs::OpenOptions::new().write(true).create_new(true).open(target).map_err(|e| e.to_string())?.sync_all().map_err(|e| e.to_string())?;
        let revision = reservation.observe_fingerprint(changes::source_fingerprint(&root, &exclusions)?)?;
        app.emit("project-source-changed", json!({"modId":mod_id,"sourceRevision":revision})).map_err(|e| e.to_string())?;
        Ok(json!({"path":relative,"sourceRevision":revision}))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_source_text(app: tauri::AppHandle, mod_id: String, path: String, expected: String, text: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if text.len() > 1024 * 1024 { return Err("文本超过 1 MiB，不能在此编辑".into()); }
        let reservation = app.state::<Workspaces>().try_reserve_native(&mod_id, NativeOperation::SourceChange)?;
        let (root, exclusions) = project_input(&app, &mod_id)?;
        let file = checked_path(&root, &path, &exclusions)?;
        io::validate_plain_file(&file)?;
        if fs::metadata(&file).map_err(|e| e.to_string())?.len() > 1024 * 1024 { return Err("磁盘文件超过编辑限制".into()); }
        if fs::read_to_string(&file).map_err(|e| e.to_string())? != expected { return Err("文件已被外部修改，请重新加载后合并你的修改".into()); }
        let mut temporary = tempfile::NamedTempFile::new_in(file.parent().ok_or("文件目录缺失")?).map_err(|e| e.to_string())?;
        temporary.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        checked_path(&root, &path, &exclusions)?;
        if fs::read_to_string(&file).map_err(|e| e.to_string())? != expected { return Err("保存期间磁盘文件变化，修改保留在编辑器中".into()); }
        temporary.persist(&file).map_err(|e| e.to_string())?;
        let revision = reservation.observe_fingerprint(changes::source_fingerprint(&root, &exclusions)?)?;
        app.emit("project-source-changed", json!({"modId":mod_id,"sourceRevision":revision})).map_err(|e| e.to_string())?;
        Ok(())
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn recycle_source_entries(app: tauri::AppHandle, mod_id: String, entries: Vec<String>) -> Result<SourceMutationResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let reservation = app.state::<Workspaces>().try_reserve_native(&mod_id, NativeOperation::SourceChange)?;
        let (root, exclusions) = project_input(&app, &mod_id)?;
        let before = changes::source_fingerprint(&root, &exclusions)?;
        let mut results = vec![];
        for relative in entries {
            let result = (|| -> Result<(), String> {
                if relative.is_empty() { return Err("不能删除工作目录".into()); }
                let path = checked_path(&root, &relative, &exclusions)?;
                plain(&path)?;
                if path.is_dir() { directory_fingerprint(&root, &relative, None, &exclusions)?; }
                recycle(&path)
            })();
            results.push(item(&relative, "", if result.is_ok() { "deleted" } else { "failed" }, result.err()));
        }
        finish(&app, &mod_id, &root, &exclusions, &reservation, uuid::Uuid::new_v4().to_string(), before, results)
    }).await.map_err(|e| e.to_string())?
}

fn recycle(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // 路径通过环境传递，不拼进 PowerShell 代码。仅调用系统回收站，无永久删除回退。
        let script = include_str!("recycle.ps1");
        let powershell = std::env::var_os("SystemRoot").map(PathBuf::from).ok_or("SystemRoot 缺失")?.join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let output = std::process::Command::new(powershell).creation_flags(0x08000000).args(["-NoProfile", "-NonInteractive", "-STA", "-Command", script]).env("GENMOD_RECYCLE_PATH", path).output().map_err(|e| e.to_string())?;
        if !output.status.success() { return Err(format!("移入回收站失败：{}", String::from_utf8_lossy(&output.stderr))); }
        if path.exists() { return Err("文件未移入回收站".into()); }
        Ok(())
    }
    #[cfg(not(windows))]
    { let _ = path; Err("系统回收站当前仅支持 Windows".into()) }
}

#[tauri::command]
pub fn source_entry_action(app: tauri::AppHandle, mod_id: String, path: String, action: String) -> Result<String, String> {
    let root = paths::project_dir(&app, &mod_id)?.join("source");
    let project = mods::read_project(&app, &mod_id)?;
    let exclusions = crate::skills::registry::resolve_for_game(&app, &project.game_id)?.into_iter().flat_map(|v| v.definition.exclude_paths).collect::<Vec<_>>();
    let target = checked_path(&root, &path, &exclusions)?;
    plain(&target)?;
    let absolute = target.to_string_lossy().into_owned();
    match action.as_str() {
        "open" => { project_input(&app, &mod_id)?; app.opener().open_path(&absolute, None::<&str>).map_err(|e| e.to_string())?; }
        "reveal" => { app.opener().reveal_item_in_dir(&absolute).map_err(|e| e.to_string())?; }
        "path" | "reference" => {},
        _ => return Err("未知文件动作".into()),
    }
    Ok(absolute)
}
