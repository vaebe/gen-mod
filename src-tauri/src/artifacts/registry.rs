use super::models::{Delivery, InstallManifest};
use crate::{changes::{file_hash, relative_path}, pi::tasks::TaskRecord, storage::{mods, paths}, artifacts::models::{ArtifactFile, ArtifactVersion}};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, fs, io::Read, path::{Path, PathBuf}};
use tauri::Manager;

/// 检查 root 到文件的每层，拒绝重解析点；用于交付、版本读取和安装。
pub fn checked_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    paths::reject_redirect(root)?;
    let mut path = root.to_path_buf();
    for component in relative_path(relative)?.components() {
        path.push(component);
        paths::reject_redirect(&path)?;
    }
    if !path.canonicalize().map_err(|e| e.to_string())?.starts_with(root.canonicalize().map_err(|e| e.to_string())?) { return Err("文件路径越界".into()); }
    Ok(path)
}
pub fn manifest(root: &Path) -> Result<InstallManifest, String> {
    let path = checked_path(root, "install.json")?;
    let manifest: InstallManifest = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let mut targets = BTreeSet::new();
    let mut sources = BTreeSet::new();
    if manifest.entries.is_empty() { return Err("安装清单为空".into()); }
    for entry in &manifest.entries {
        relative_path(&entry.source)?; relative_path(&entry.target)?;
        if !targets.insert(entry.target.to_lowercase()) || !sources.insert(entry.source.to_lowercase()) { return Err("安装清单含重复文件或目标".into()); }
    }
    Ok(manifest)
}

/// 仅由当前 prepare_delivery 成功结束事件调用；助手正文与前端没有登记入口。
pub fn register(app: &tauri::AppHandle, task: &mut TaskRecord, delivery: Delivery, input: &serde_json::Value, known_secrets: &[String], cancelled: Option<&std::sync::atomic::AtomicBool>) -> Result<ArtifactVersion, String> {
    if delivery.task_id != task.context.task_id || delivery.staging_id != task.context.staging_id { return Err("交付不属于当前任务".into()); }
    app.state::<crate::pi::workspace::Workspaces>().validate_delivery(app,&task.context,&delivery.lease_id,delivery.source_revision)?;
    let storage = app.state::<mods::StorageState>();
    let _lock = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    crate::project_lifecycle::repository::ensure_writable(app,&task.context.mod_id)?;
    let project = mods::read_project(app, &task.context.mod_id)?;
    if let Some(id) = &task.registered_version_id { return project.versions.iter().find(|v| &v.version_id == id).cloned().ok_or("已登记版本记录缺失".into()); }
    if task.status != "running" { return Err("只有活动任务能够首次登记版本".into()); }
    let summary = input["summary"].as_str().map(str::trim).filter(|value| !value.is_empty()).ok_or("请补充本次实际交付变化的简短摘要后重试")?;
    if delivery.command_ids.is_empty() { return Err("交付缺少成功命令".into()); }
    let mut ids = BTreeSet::new();
    let mut successful = vec![];
    for id in &delivery.command_ids {
        if !ids.insert(id) { return Err("交付命令 ID 重复".into()); }
        let receipt = task.commands.iter().find(|c| &c.command_id == id).ok_or("交付使用未知命令 ID")?;
        if receipt.status != "succeeded" || receipt.exit_code != Some(0) { return Err("交付使用失败或取消的命令".into()); }
        if receipt.source_revision != delivery.source_revision { return Err("构建后源码已变化，请重新构建".into()); }
        successful.push(receipt);
    }
    let mut requires_fresh_output = false;
    for view in crate::skills::registry::task_views(app, &task.context)? {
        requires_fresh_output |= !view.definition.required_command_scripts.is_empty();
        for script in &view.definition.required_command_scripts {
            let expected = checked_path(Path::new(&view.definition.path), script)?.canonicalize().map_err(|e| e.to_string())?;
            if !successful.iter().any(|c| c.args.iter().any(|arg| Path::new(arg).canonicalize().is_ok_and(|actual| actual == expected))) { return Err(format!("缺少本任务必要脚本成功记录：{script}")); }
        }
    }
    let staging = PathBuf::from(&task.context.staging_directory);
    let project_root = paths::project_dir(app, &task.context.mod_id)?;
    let expected = checked_path(&project_root, &format!("logs/{}/staging/{}", task.context.task_id, task.context.staging_id))?;
    if staging.canonicalize().map_err(|e| e.to_string())? != expected.canonicalize().map_err(|e| e.to_string())? { return Err("staging 路径不一致".into()); }
    if Path::new(&delivery.manifest_path) != staging.join("install.json") || Path::new(&delivery.zip_path) != staging.join("mod.zip") { return Err("交付返回了非当前清单或 ZIP".into()); }
    let install = manifest(&staging)?;
    let prepared: Vec<ArtifactFile> = serde_json::from_slice(&fs::read(checked_path(&staging, "files.json")?).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let inputs = input["files"].as_array().ok_or("缺少实际工具文件参数")?;
    if inputs.len() != prepared.len() || install.entries.len() != prepared.len() { return Err("交付文件集不一致".into()); }
    let mut hashes = BTreeMap::new();
    for file in &prepared {
        if hashes.insert(file.path.clone(), (file.size, file.sha256.clone())).is_some() { return Err("交付文件重复".into()); }
        let copied = checked_path(&staging.join("files"), &file.path)?;
        if !copied.is_file() || fs::metadata(&copied).map_err(|e| e.to_string())?.len() != file.size || file_hash(&copied)? != file.sha256 { return Err("交付文件内容发生变化".into()); }
        let original = inputs.iter().find(|i| i["target"].as_str() == Some(&file.path)).and_then(|i| i["source"].as_str()).ok_or("工具参数与交付文件不一致")?;
        if requires_fresh_output && !original.starts_with(&format!(".delivery/{}/", task.context.task_id)) { return Err("需要编译的能力必须交付本任务全新输出目录中的文件".into()); }
        let source = checked_path(Path::new(&task.context.source_directory), original)?;
        if file_hash(&source)? != file.sha256 { return Err("交付内容与本任务源码输出不一致".into()); }
    }
    for entry in &install.entries { if !hashes.contains_key(&entry.source) { return Err("安装清单引用未知文件".into()); } }
    let zip_path = checked_path(&staging, "mod.zip")?;
    let mut archive = zip::ZipArchive::new(fs::File::open(&zip_path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let mut found = BTreeSet::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        let name = entry.name().replace('\\', "/");
        relative_path(name.trim_end_matches('/'))?;
        if entry.is_dir() { continue; }
        if entry.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000) { return Err("ZIP 含符号链接".into()); }
        let (size, hash) = hashes.get(&name).ok_or("ZIP 含清单外文件")?;
        if entry.size() != *size || !found.insert(name.clone()) { return Err("ZIP 文件大小或重复项错误".into()); }
        let mut actual = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop { let count = entry.read(&mut buffer).map_err(|e| e.to_string())?; if count == 0 { break; } actual.update(&buffer[..count]); }
        if format!("{:x}", actual.finalize()) != *hash { return Err("ZIP 与交付文件内容不一致".into()); }
    }
    if found.len() != hashes.len() { return Err("ZIP 缺少文件".into()); }
    let restricted=super::content_check::restricted_for_task(app,&task.context)?;
    let check_files=prepared.iter().map(|f|(f.path.clone(),f.path.clone())).collect::<Vec<_>>();
    let report=super::content_check::check_files(&staging.join("files"),&check_files,known_secrets,&restricted)?;
    if report.findings.iter().any(|f|f.level=="block"){return Err(format!("最终交付内容检查拒绝：{}；需要新任务",serde_json::to_string(&report).map_err(|e|e.to_string())?));}
    let cases:Vec<super::delivery_metadata::VerificationCase>=input.get("verificationCases").map(|v|serde_json::from_value(v.clone())).transpose().map_err(|e|e.to_string())?.unwrap_or_default();
    super::delivery_metadata::validate_cases(&cases)?;
    let version = ArtifactVersion { summary: summary.to_string(), session_id: task.context.session_id.clone(), version_id: uuid::Uuid::new_v4().to_string(), task_id: task.context.task_id.clone(), created_at: mods::now()?, skills: crate::skills::registry::snapshots(&crate::skills::registry::task_views(app,&task.context)?), files: prepared, zip_sha256: file_hash(&zip_path)? };
    let artifacts = project_root.join("artifacts");
    paths::reject_redirect(&artifacts)?;
    let temporary = tempfile::Builder::new().prefix(".register-").tempdir_in(&artifacts).map_err(|e| e.to_string())?;
    fs::create_dir(temporary.path().join("files")).map_err(|e| e.to_string())?;
    for file in &version.files {
        let destination = temporary.path().join("files").join(relative_path(&file.path)?);
        fs::create_dir_all(destination.parent().ok_or("文件目录无效")?).map_err(|e| e.to_string())?;
        fs::copy(checked_path(&staging.join("files"), &file.path)?, destination).map_err(|e| e.to_string())?;
    }
    fs::copy(zip_path, temporary.path().join("mod.zip")).map_err(|e| e.to_string())?;
    mods::write_json(&temporary.path().join("install.json"), &install)?;
    mods::write_json(&temporary.path().join("version.json"), &version)?;
    let final_directory = artifacts.join(&version.version_id);
    let operation_id = uuid::Uuid::new_v4().to_string();
    let operation = serde_json::json!({"versionId":version.version_id,"taskId":task.context.task_id});
    crate::application::file_operations::file_operation(app,&operation_id,&project.mod_id,"register","pending",&operation)?;
    fs::rename(temporary.path(), &final_directory).map_err(|e| e.to_string())?;
    if let Err(error) = (||->Result<(),String>{
        app.state::<crate::pi::workspace::Workspaces>().validate_delivery(app,&task.context,&delivery.lease_id,delivery.source_revision)?;
        let mut db=crate::storage::database::connect(app)?;
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        if cancelled.is_some_and(|v|v.load(std::sync::atomic::Ordering::SeqCst)){return Err("交付工具已结束或取消，未登记版本".into());}
        let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM tasks WHERE task_id=?1 AND status='running')",[&task.context.task_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !active{return Err("任务已停止，不能登记".into());}
        let current=crate::storage::repository::read_on(&tx,&project.mod_id)?;
        super::repository::register_on(&tx,&project.mod_id,current.revision,&version,&mods::now()?)?;
        super::delivery_metadata::ensure_anchor_on(&tx,&project.mod_id,&version)?;
        let mut metadata=super::delivery_metadata::read_on(&tx,&project.mod_id,&version.version_id)?;
        metadata.context_snapshot_id=Some(task.context.context_snapshot_id.clone());metadata.check_report=Some(report);metadata.cases=cases;
        super::delivery_metadata::save_on(&tx,&metadata)?;
        let mut registered_task=task.clone();registered_task.registered_version_id=Some(version.version_id.clone());
        crate::pi::tasks::task_on(&tx,&registered_task)?;
        tx.commit().map_err(|e|e.to_string())
    })() {
        // 元数据失败时只清理本次新版本，保留所有既有版本。
        let actual = final_directory.canonicalize().map_err(|e| e.to_string())?;
        if actual.starts_with(artifacts.canonicalize().map_err(|e| e.to_string())?) { let _ = fs::remove_dir_all(actual); }
        return Err(error);
    }
    crate::application::file_operations::file_operation(app,&operation_id,&project.mod_id,"register","done",&operation)?;
    task.registered_version_id = Some(version.version_id.clone());
    Ok(version)
}

pub fn version_root(app: &tauri::AppHandle, mod_id: &str, version_id: &str) -> Result<PathBuf, String> {
    paths::validate_id(version_id)?;
    let project = mods::read_project(app, mod_id)?;
    if !project.versions.iter().any(|v| v.version_id == version_id) { return Err("版本不属于所选项目".into()); }
    checked_path(&paths::project_dir(app, mod_id)?, &format!("artifacts/{version_id}"))
}

#[tauri::command]
pub fn get_install_hint(app: tauri::AppHandle, storage: tauri::State<mods::StorageState>, mod_id: String, version_id: String) -> Result<super::models::InstallHint, String> {
    let _lock = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    let manifest = manifest(&version_root(&app, &mod_id, &version_id)?)?;
    let project = mods::read_project(&app, &mod_id)?;
    let hint = crate::games::list(&app)?.into_iter()
        .find(|game| game.id == project.game_id)
        .and_then(|game| game.install_hint);
    Ok(hint.unwrap_or(super::models::InstallHint { description: manifest.target_directory_hint, example_path: String::new() }))
}

/// 用户手动删除应用内的版本包；已部署文件、源码及安装历史保持不动。
#[tauri::command]
pub fn delete_version(app: tauri::AppHandle, storage: tauri::State<mods::StorageState>, mod_id: String, version_id: String) -> Result<crate::storage::models::ModProject, String> {
    let _lock = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    crate::project_lifecycle::repository::ensure_writable(&app,&mod_id)?;
    let mut project = mods::read_project(&app, &mod_id)?;
    let project_root = paths::project_dir(&app, &mod_id)?;
    let artifacts = checked_path(&project_root, "artifacts")?.canonicalize().map_err(|e| e.to_string())?;
    let root = version_root(&app, &mod_id, &version_id)?.canonicalize().map_err(|e| e.to_string())?;
    // 递归清理前验证绝对路径只能是当前项目 artifacts 的直接子目录。
    if root.parent() != Some(artifacts.as_path()) || !root.is_dir() { return Err("版本目录越界，不能删除".into()); }
    let temporary = artifacts.join(format!(".delete-{}", uuid::Uuid::new_v4()));
    project.updated_at = mods::now()?;
    let operation_id = uuid::Uuid::new_v4().to_string();
    let operation = serde_json::json!({"versionId":version_id,"tombstone":temporary.file_name().ok_or("删除目录无效")?.to_string_lossy()});
    crate::application::file_operations::file_operation(&app,&operation_id,&mod_id,"delete","pending",&operation)?;
    fs::rename(&root, &temporary).map_err(|e| e.to_string())?;
    super::delivery_metadata::ensure_anchor_on(&crate::storage::database::connect(&app)?,&mod_id,project.versions.iter().find(|v|v.version_id==version_id).ok_or("版本不存在")?)?;
    if let Err(error) = (||->Result<(),String>{
        let mut db=crate::storage::database::connect(&app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        super::repository::delete_on(&tx,&mod_id,project.revision,&version_id,&project.updated_at)?;
        super::delivery_metadata::mark_deleted_on(&tx,&mod_id,&version_id,true)?;
        project=crate::storage::repository::read_on(&tx,&mod_id)?;
        tx.commit().map_err(|e|e.to_string())?;Ok(())
    })() {
        fs::rename(&temporary, &root).map_err(|restore| format!("{error}；版本目录恢复失败：{restore}"))?;
        return Err(error);
    }
    let actual = temporary.canonicalize().map_err(|e| e.to_string())?;
    if actual.parent() != Some(artifacts.as_path()) { return Err("版本清理路径越界".into()); }
    fs::remove_dir_all(actual).map_err(|e| format!("版本记录已删除，残留目录清理失败：{e}"))?;
    crate::application::file_operations::file_operation(&app,&operation_id,&mod_id,"delete","done",&operation)?;
    Ok(project)
}
