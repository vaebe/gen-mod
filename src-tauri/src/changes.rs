use crate::{pi::tasks::TaskContext, storage::{mods, paths}};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::{Component, Path, PathBuf}};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileSnapshot { size: u64, sha256: String, text: Option<String> }
type Snapshot = BTreeMap<String, FileSnapshot>;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub kind: String,
    pub added: usize,
    pub removed: usize,
    pub before_size: Option<u64>,
    pub after_size: Option<u64>,
    pub text_available: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub change: FileChange,
    pub before: Option<String>,
    pub after: Option<String>,
    pub lines: Vec<DiffLine>,
}
#[derive(Serialize)]
pub struct DiffLine { kind: String, text: String }

/// 所有交付、定位与差异入口共用相对路径校验，拒绝 Windows ADS。
pub fn relative_path(value: &str) -> Result<PathBuf, String> {
    if value.is_empty() || value.contains([':', '\\']) { return Err("请使用有效的相对路径和 / 分隔符".into()); }
    for segment in value.split('/') {
        let stem = segment.split('.').next().unwrap_or("").to_ascii_uppercase();
        if segment.is_empty() || segment.ends_with([' ', '.']) || segment.contains(['<', '>', '"', '|', '?', '*'])
            || segment.chars().any(|c| c.is_control()) || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ((stem.starts_with("COM") || stem.starts_with("LPT")) && stem.len() == 4 && stem.as_bytes()[3].is_ascii_digit()) {
            return Err("相对路径含 Windows 不支持的文件名".into());
        }
    }
    let path = Path::new(value);
    if path.components().any(|part| !matches!(part, Component::Normal(_))) {
        return Err("相对路径包含越界或特殊组件".into());
    }
    Ok(path.to_path_buf())
}

pub fn file_hash(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 { break; }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn snapshot(root: &Path, exclusions: &[String]) -> Result<Snapshot, String> {
    fn walk(root: &Path, directory: &Path, files: &mut Snapshot, exclusions: &[String]) -> Result<(), String> {
        paths::reject_redirect(directory)?;
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            let relative = path.strip_prefix(root).map_err(|_| "文件位于源码目录之外")?.to_string_lossy().replace('\\', "/");
            if exclusions.iter().any(|excluded| relative == *excluded || relative.starts_with(&format!("{excluded}/"))) { continue; }
            paths::reject_redirect(&path)?;
            let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
            if metadata.is_dir() { walk(root, &path, files, exclusions)?; }
            else if metadata.is_file() {
                let text = if metadata.len() <= 1024 * 1024 {
                    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
                    if bytes.contains(&0) { None } else { String::from_utf8(bytes).ok() }
                } else { None };
                files.insert(relative, FileSnapshot { size: metadata.len(), sha256: file_hash(&path)?, text });
            }
        }
        Ok(())
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result, exclusions)?;
    Ok(result)
}

fn directory(app: &tauri::AppHandle, context: &TaskContext) -> Result<PathBuf, String> {
    Ok(paths::project_dir(app, &context.mod_id)?.join("logs").join(&context.task_id).join("changes"))
}

/// 仅哈希可构建输入；交付 staging 与任务专属输出不属于源码 revision。
pub fn source_fingerprint(root: &Path, exclusions: &[String]) -> Result<String,String> {
    let files = snapshot(root,exclusions)?;
    let mut hasher = Sha256::new();
    for (path,file) in files { hasher.update(path.as_bytes()); hasher.update([0]); hasher.update(file.sha256.as_bytes()); }
    Ok(format!("{:x}",hasher.finalize()))
}
pub fn source_manifest(root: &Path, exclusions: &[String]) -> Result<BTreeMap<String,String>,String> {
    Ok(snapshot(root,exclusions)?.into_iter().map(|(path,file)|(path,file.sha256)).collect())
}

/// 在任何 prompt/tool 写入之前保存文本基准，之后历史差异不读可变源码。
pub fn begin(app: &tauri::AppHandle, context: &TaskContext, skill_exclusions: &[String]) -> Result<(), String> {
    let directory = directory(app, context)?;
    fs::create_dir_all(directory.join("before")).map_err(|e| e.to_string())?;
    let mut exclusions = vec!["bin".into(), "obj".into(), ".git".into(), "node_modules".into(), "artifacts".into()];
    for exclusion in skill_exclusions { relative_path(exclusion)?; exclusions.push(exclusion.clone()); }
    let before = snapshot(Path::new(&context.source_directory), &exclusions)?;
    for (path, value) in &before {
        if let Some(text) = &value.text {
            let output = directory.join("before").join(relative_path(path)?);
            fs::create_dir_all(output.parent().ok_or("基准路径无效")?).map_err(|e| e.to_string())?;
            fs::write(output, text).map_err(|e| e.to_string())?;
        }
    }
    mods::write_json(&directory.join("exclusions.json"), &exclusions)?;
    mods::write_json(&directory.join("before.json"), &before)?;
    mods::write_json(&directory.join("after.json"), &before)?;
    mods::write_json(&directory.join("changes.json"), &Vec::<FileChange>::new())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    paths::reject_redirect(path)?;
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub fn refresh(app: &tauri::AppHandle, context: &TaskContext) -> Result<(), String> {
    let directory = directory(app, context)?;
    let exclusions: Vec<String> = read_json(&directory.join("exclusions.json"))?;
    let before: Snapshot = read_json(&directory.join("before.json"))?;
    let after = snapshot(Path::new(&context.source_directory), &exclusions)?;
    let mut keys: Vec<_> = before.keys().chain(after.keys()).cloned().collect();
    keys.sort(); keys.dedup();
    let mut changes = vec![];
    for path in keys {
        let old = before.get(&path);
        let new = after.get(&path);
        if old.map(|f| &f.sha256) == new.map(|f| &f.sha256) { continue; }
        let available = old.map_or(true, |f| f.text.is_some()) && new.map_or(true, |f| f.text.is_some());
        let (mut added, mut removed) = (0, 0);
        if available {
            let diff = similar::TextDiff::from_lines(old.and_then(|f| f.text.as_deref()).unwrap_or(""), new.and_then(|f| f.text.as_deref()).unwrap_or(""));
            for line in diff.iter_all_changes() {
                match line.tag() { similar::ChangeTag::Insert => added += 1, similar::ChangeTag::Delete => removed += 1, _ => {} }
            }
        }
        changes.push(FileChange { path, kind: if old.is_none() { "added" } else if new.is_none() { "deleted" } else { "modified" }.into(), added, removed, before_size: old.map(|f| f.size), after_size: new.map(|f| f.size), text_available: available });
    }
    mods::write_json(&directory.join("after.json"), &after)?;
    mods::write_json(&directory.join("changes.json"), &changes)
}

fn context(app: &tauri::AppHandle, mod_id: &str, task_id: &str) -> Result<TaskContext, String> {
    paths::validate_id(task_id)?;
    mods::read_project(app, mod_id)?;
    let task = crate::pi::tasks::task(app, mod_id, task_id)?;
    if task.context.mod_id != mod_id || task.context.task_id != task_id { return Err("任务不属于当前项目".into()); }
    Ok(task.context)
}

#[tauri::command]
pub fn list_task_changes(app: tauri::AppHandle, mod_id: String, task_id: String) -> Result<Vec<FileChange>, String> {
    let context = context(&app, &mod_id, &task_id)?;
    read_json(&directory(&app, &context)?.join("changes.json"))
}

#[tauri::command]
pub fn get_task_file_diff(app: tauri::AppHandle, mod_id: String, task_id: String, relative_path: String) -> Result<FileDiff, String> {
    self::relative_path(&relative_path)?;
    let context = context(&app, &mod_id, &task_id)?;
    let directory = directory(&app, &context)?;
    let changes: Vec<FileChange> = read_json(&directory.join("changes.json"))?;
    let change = changes.into_iter().find(|item| item.path == relative_path).ok_or("任务中没有此文件的差异")?;
    let before: Snapshot = read_json(&directory.join("before.json"))?;
    let after: Snapshot = read_json(&directory.join("after.json"))?;
    let before = before.get(&relative_path).and_then(|f| f.text.clone());
    let after = after.get(&relative_path).and_then(|f| f.text.clone());
    let lines = if change.text_available {
        similar::TextDiff::from_lines(before.as_deref().unwrap_or(""), after.as_deref().unwrap_or("")).iter_all_changes().map(|line| DiffLine { kind: match line.tag() { similar::ChangeTag::Delete => "delete", similar::ChangeTag::Insert => "insert", _ => "equal" }.into(), text: line.to_string() }).collect()
    } else { vec![] };
    Ok(FileDiff { change, before, after, lines })
}

/// 工具租约内记录的操作归本任务，区别于整个共享目录的累计差异。
#[tauri::command]
pub fn list_task_operations(app: tauri::AppHandle, mod_id: String, task_id: String) -> Result<Vec<serde_json::Value>,String> {
    context(&app,&mod_id,&task_id)?;
    let path=paths::project_dir(&app,&mod_id)?.join("logs").join(task_id).join("workspace-operations.jsonl");
    if !path.exists() { return Ok(vec![]); } paths::reject_redirect(&path)?;
    use std::io::BufRead;
    std::io::BufReader::new(fs::File::open(path).map_err(|e|e.to_string())?).lines().map(|line|serde_json::from_str(&line.map_err(|e|e.to_string())?).map_err(|e|e.to_string())).collect()
}
