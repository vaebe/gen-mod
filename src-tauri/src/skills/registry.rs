use super::models::{SkillDefinition, SkillView, UserRegistry};
use crate::storage::{mods, paths, models::SkillSnapshot};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, fs, path::{Path, PathBuf}};
use tauri::Manager;

pub fn builtin_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) { Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../skills")) }
    else { Ok(app.path().resource_dir().map_err(|e| e.to_string())?.join("skills")) }
}
fn user_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("skills"))
}
fn user_registry(app: &tauri::AppHandle) -> Result<UserRegistry, String> {
    let file = user_root(app)?.join("registry.json");
    if !file.exists() { return Ok(UserRegistry::default()); }
    paths::reject_redirect(file.parent().ok_or("能力目录无效")?)?;
    paths::reject_redirect(&file)?;
    serde_json::from_slice(&fs::read(file).map_err(|e| e.to_string())?).map_err(|e| format!("用户能力清单损坏：{e}"))
}

/// 对资源文件排序后连同路径计算哈希；脚本和引用资料更新也改变会话上下文。
fn inventory(root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, PathBuf>) -> Result<(), String> {
        paths::reject_redirect(dir)?;
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            paths::reject_redirect(&path)?;
            if path.is_dir() { walk(root, &path, files)?; }
            else if path.is_file() { files.insert(path.strip_prefix(root).map_err(|_| "资源路径越界")?.to_string_lossy().replace('\\', "/"), path); }
            else { return Err("能力资源必须是普通文件或目录".into()); }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files)?;
    Ok(files)
}
pub(crate) fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 64 && !name.starts_with('-') && !name.ends_with('-') && !name.contains("--")
        && name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
fn frontmatter(text: &str) -> Result<(String, String), String> {
    let mut lines = text.lines();
    if lines.next() != Some("---") { return Err("SKILL.md 缺少 YAML frontmatter".into()); }
    let mut yaml = String::new();
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" { closed = true; break; }
        yaml.push_str(line); yaml.push('\n');
    }
    if !closed { return Err("SKILL.md frontmatter 缺少结束标记".into()); }
    #[derive(serde::Deserialize)]
    struct Frontmatter { name: String, description: String }
    let front: Frontmatter = serde_yaml_ng::from_str(&yaml).map_err(|e| format!("SKILL.md frontmatter 无效：{e}"))?;
    if !valid_name(&front.name) || front.description.trim().is_empty() { return Err("SKILL.md 需有效 name 和 description".into()); }
    Ok((front.name, front.description.trim().into()))
}
pub(crate) fn validate(definition: &SkillDefinition, root: &Path) -> Result<(String, Vec<String>), String> {
    if !valid_name(&definition.name) || definition.version.trim().is_empty()
        || !matches!(definition.category.as_str(), "common" | "game")
        || (definition.category == "game" && definition.game_id.as_deref().unwrap_or("").is_empty()) {
        return Err("能力名称、版本、分类或游戏标识无效".into());
    }
    for path in &definition.exclude_paths { crate::changes::relative_path(path)?; }
    let files = inventory(root)?;
    for script in &definition.required_command_scripts {
        crate::changes::relative_path(script)?;
        if !files.contains_key(script) { return Err(format!("必需命令脚本不存在：{script}")); }
    }
    if let Some(file)=&definition.inspection_file {
        crate::changes::relative_path(file)?;
        let path=files.get(file).ok_or("调查规则文件不存在")?;
        let rule:crate::game_inspection::models::InspectionRule=serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        crate::game_inspection::scanner::validate_rules(&rule)?;
    }
    let entry = files.get("SKILL.md").ok_or("能力目录缺少 SKILL.md")?;
    let (name, description) = frontmatter(&fs::read_to_string(entry).map_err(|e| e.to_string())?)?;
    if name != definition.name || description != definition.description.trim() { return Err("能力清单与 SKILL.md 的 name/description 不一致".into()); }
    let mut hash = Sha256::new();
    hash.update(serde_json::to_vec(&serde_json::json!({
        "category": definition.category, "gameId": definition.game_id, "version": definition.version,
        "dependencies": definition.dependencies, "excludePaths": definition.exclude_paths,
        "requiredCommandScripts": definition.required_command_scripts, "inspectionFile":definition.inspection_file
    })).map_err(|e| e.to_string())?);
    let mut scripts = vec![];
    for (relative, path) in &files {
        hash.update(relative.as_bytes()); hash.update([0]); hash.update(crate::changes::file_hash(path)?.as_bytes()); hash.update([0]);
        if relative.starts_with("scripts/") { scripts.push(relative.clone()); }
        if path.extension().is_some_and(|value| value == "md") {
            let markdown = fs::read_to_string(path).map_err(|e| e.to_string())?;
            // 验证 Markdown 的本地引用；导入过程中只读取，不执行脚本。
            for rest in markdown.split("](").skip(1) {
                let target = rest.split(')').next().unwrap_or("").split_whitespace().next().unwrap_or("").trim_matches(['<', '>']);
                if target.is_empty() || target.starts_with('#') || target.starts_with("https://") || target.starts_with("http://") { continue; }
                let relative = target.split('#').next().unwrap_or("");
                let joined = path.parent().ok_or("引用路径无效")?.join(relative);
                let actual = joined.canonicalize().map_err(|_| format!("能力引用不存在：{relative}"))?;
                if !actual.starts_with(root.canonicalize().map_err(|e| e.to_string())?) { return Err(format!("能力引用越界：{relative}")); }
                paths::reject_redirect(&actual)?;
            }
        }
    }
    Ok((format!("{:x}", hash.finalize()), scripts))
}
pub fn list(app: &tauri::AppHandle) -> Result<Vec<SkillView>, String> {
    let root = builtin_root(app)?;
    let builtin: Vec<SkillDefinition> = serde_json::from_slice(&fs::read(root.join("registry.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let user = user_registry(app)?;
    let imported_root = user_root(app)?;
    let mut names = BTreeSet::new();
    let mut views = vec![];
    for (definition, source, base) in builtin.into_iter().map(|d| (d, "builtin", root.clone())).chain(user.imported.into_iter().map(|d| (d, "user", imported_root.clone()))) {
        if !names.insert(definition.name.clone()) { return Err(format!("重复的能力名称：{}", definition.name)); }
        let relative = crate::changes::relative_path(&definition.path)?;
        let directory = base.join(relative);
        let (content_hash, scripts, error) = match validate(&definition, &directory) {
            Ok((hash, scripts)) => (hash, scripts, None),
            Err(error) => (String::new(), vec![], Some(error)),
        };
        let enabled = !user.disabled.contains(&definition.name);
        let mut definition = definition;
        definition.path = directory.to_string_lossy().into_owned();
        views.push(SkillView { definition, source: source.into(), enabled, content_hash, scripts, error });
    }
    Ok(views)
}

/// 仅解析所选游戏与其通用依赖，拒绝环和跨游戏的隐式加载。
pub fn resolve(app: &tauri::AppHandle, name: &str) -> Result<Vec<SkillView>, String> {
    let views = list(app)?;
    let game = views.iter().find(|v| v.definition.name == name).ok_or("所选能力不存在")?;
    if game.definition.category != "game" { return Err("项目必须选择游戏能力".into()); }
    fn visit(name: &str, game: &str, views: &[SkillView], stack: &mut BTreeSet<String>, done: &mut BTreeSet<String>, out: &mut Vec<String>) -> Result<(), String> {
        if done.contains(name) { return Ok(()); }
        if !stack.insert(name.into()) { return Err(format!("能力依赖循环：{name}")); }
        let view = views.iter().find(|v| v.definition.name == name).ok_or_else(|| format!("缺少依赖：{name}"))?;
        if let Some(error) = &view.error { return Err(format!("能力 {name} 无效：{error}")); }
        if !view.enabled { return Err(format!("能力已禁用：{name}")); }
        if name != game && view.definition.category != "common" { return Err("游戏能力只能依赖通用能力".into()); }
        for dependency in &view.definition.dependencies { visit(dependency, game, views, stack, done, out)?; }
        stack.remove(name); done.insert(name.into()); out.push(name.into()); Ok(())
    }
    let mut names = vec![];
    visit(name, name, &views, &mut BTreeSet::new(), &mut BTreeSet::new(), &mut names)?;
    Ok(names.into_iter().map(|n| views.iter().find(|v| v.definition.name == n).unwrap()).map(|v| SkillView { definition: v.definition.clone(), source: v.source.clone(), enabled: v.enabled, content_hash: v.content_hash.clone(), scripts: v.scripts.clone(), error: v.error.clone() }).collect())
}
pub fn snapshots(views: &[SkillView]) -> Vec<SkillSnapshot> {
    views.iter().map(|v| SkillSnapshot { name: v.definition.name.clone(), version: v.definition.version.clone(), content_hash: v.content_hash.clone() }).collect()
}
fn same(a: &[SkillSnapshot], b: &[SkillSnapshot]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.name == b.name && a.version == b.version && a.content_hash == b.content_hash)
}
pub fn refresh_sessions(app: &tauri::AppHandle, project: &mut crate::storage::models::ModProject) -> Result<(), String> {
    let current = resolve(app, &project.game_skill_name).map(|v| snapshots(&v));
    for session in &mut project.sessions {
        if current.as_ref().map_or(true, |v| !same(&session.skills, v)) { session.read_only = true; }
    }
    Ok(())
}

#[tauri::command]
pub fn list_skills(app: tauri::AppHandle) -> Result<Vec<SkillView>, String> { list(&app) }

/// 按注册名称定位入口文件，复用能力清单路径，不接受前端传入任意文件路径。
fn skill_file(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    let skill = list(app)?.into_iter().find(|v| v.definition.name == name).ok_or("能力不存在")?;
    let root = PathBuf::from(skill.definition.path);
    paths::reject_redirect(&root)?;
    let file = root.join("SKILL.md");
    paths::reject_redirect(&file)?;
    if !file.is_file() { return Err("SKILL.md 不是普通文件".into()); }
    Ok(file)
}

/// 返回已安装能力的完整 Markdown 文本，仅供详情查看。
#[tauri::command]
pub fn read_skill_content(app: tauri::AppHandle, name: String) -> Result<String, String> {
    fs::read_to_string(skill_file(&app, &name)?).map_err(|e| e.to_string())
}

/// 在系统文件资源管理器中选中已安装能力的入口文件。
#[tauri::command]
pub fn reveal_skill_file(app: tauri::AppHandle, name: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().reveal_item_in_dir(skill_file(&app, &name)?.to_string_lossy().into_owned()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_skill_enabled(app: tauri::AppHandle, state: tauri::State<mods::StorageState>, name: String, enabled: bool) -> Result<(), String> {
    crate::pi::bridge::while_idle(&app, || {
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    list(&app)?.iter().find(|v| v.definition.name == name).ok_or("能力不存在")?;
    let mut user = user_registry(&app)?;
    user.disabled.retain(|n| *n != name);
    if !enabled { user.disabled.push(name); }
    let root = user_root(&app)?;
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    paths::reject_redirect(&root)?;
    mods::write_json(&root.join("registry.json"), &user)
    })
}
#[tauri::command]
pub fn import_skill(app: tauri::AppHandle, state: tauri::State<mods::StorageState>, directory: String, mut definition: SkillDefinition) -> Result<(), String> {
    crate::pi::bridge::while_idle(&app, || {
    let _lock = state.0.lock().map_err(|_| "项目存储锁异常")?;
    if list(&app)?.iter().any(|v| v.definition.name == definition.name) { return Err("名称已存在；内置能力不可覆盖，请使用不同名称".into()); }
    let root = PathBuf::from(directory);
    if !root.is_absolute() { return Err("请选择能力目录的绝对路径".into()); }
    validate(&definition, &root)?;
    let target_root = user_root(&app)?;
    fs::create_dir_all(&target_root).map_err(|e| e.to_string())?;
    paths::reject_redirect(&target_root)?;
    let target = target_root.join(&definition.name);
    if target.exists() { return Err("能力目标目录已存在".into()); }
    let temporary = tempfile::Builder::new().prefix(".import-").tempdir_in(&target_root).map_err(|e| e.to_string())?;
    for (relative, source) in inventory(&root)? {
        let destination = temporary.path().join(crate::changes::relative_path(&relative)?);
        fs::create_dir_all(destination.parent().ok_or("资源路径无效")?).map_err(|e| e.to_string())?;
        fs::copy(source, destination).map_err(|e| e.to_string())?;
    }
    definition.path = definition.name.clone();
    validate(&definition, temporary.path())?;
    let mut registry = user_registry(&app)?;
    registry.imported.push(definition);
    fs::rename(temporary.path(), &target).map_err(|e| e.to_string())?;
    if let Err(error) = mods::write_json(&target_root.join("registry.json"), &registry) {
        let actual = target.canonicalize().map_err(|e| e.to_string())?;
        if actual.starts_with(target_root.canonicalize().map_err(|e| e.to_string())?) { let _ = fs::remove_dir_all(&actual); }
        return Err(error);
    }
    Ok(())
    })
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn builtin_resources_match_registry_and_include_inspection_rules(){
 let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../skills");
 let definitions:Vec<SkillDefinition>=serde_json::from_slice(&fs::read(root.join("registry.json")).unwrap()).unwrap();
 for definition in definitions {let(hash,_)=validate(&definition,&root.join(&definition.path)).unwrap();assert_eq!(hash.len(),64);}
 }
}
