mod io;
#[cfg(test)]
mod tests;

use std::{io::Write, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{Emitter, Manager};
use crate::{changes, pi::workspace::{NativeOperation, Workspaces}, storage::{mods, paths, repository}};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceImportItem { source_path: String, target_name: String }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMutationItem { source: String, target: String, status: &'static str, message: Option<String> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMutationResult { operation_id: String, source_revision: u64, items: Vec<SourceMutationItem> }
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceOperation {
    operation_id: String,
    source_relative_path: Option<String>,
    target_relative_path: String,
    temporary_relative_path: Option<String>,
    before_hash: Option<String>,
    result_hash: Option<String>,
    phase: String,
}

fn file_name(name: &str) -> Result<(), String> {
    let path = changes::relative_path(name)?;
    if path.components().count() != 1 { return Err("名称只能包含文件名，不能包含目录".into()); }
    Ok(())
}

fn checked_path(root: &Path, relative: &str, exclusions: &[String]) -> Result<PathBuf, String> {
    let mut result = root.to_path_buf();
    for ancestor in root.ancestors() { paths::reject_redirect(ancestor)?; }
    if !relative.is_empty() {
        let path = changes::relative_path(relative)?;
        let lower = relative.to_lowercase();
        for excluded in [".git", ".delivery"].into_iter().chain(exclusions.iter().map(String::as_str)) {
            let excluded = excluded.replace('\\', "/").trim_matches('/').to_lowercase();
            if !excluded.is_empty() && (lower == excluded || lower.starts_with(&format!("{excluded}/"))) {
                return Err("不能操作项目保留目录".into());
            }
        }
        for part in path.components() {
            result.push(part);
            match std::fs::symlink_metadata(&result) {
                Ok(_) => paths::reject_redirect(&result)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                Err(error) => return Err(error.to_string()),
            }
        }
    }
    Ok(result)
}

fn target_path(root: &Path, directory: &str, name: &str, exclusions: &[String]) -> Result<(String, PathBuf), String> {
    file_name(name)?;
    let parent = checked_path(root, directory, exclusions)?;
    if !parent.is_dir() { return Err("目标目录不存在".into()); }
    let relative = if directory.is_empty() { name.into() } else { format!("{directory}/{name}") };
    Ok((relative.clone(), checked_path(root, &relative, exclusions)?))
}

fn project_input(app: &tauri::AppHandle, mod_id: &str) -> Result<(PathBuf, Vec<String>), String> {
    let storage = app.state::<mods::StorageState>();
    let _guard = storage.0.lock().map_err(|_| "项目存储锁异常")?;
    crate::project_lifecycle::repository::ensure_writable(app,mod_id)?;
    let project = mods::read_project(app, mod_id)?;
    if repository::has_active_tasks(app, mod_id)? { return Err("当前项目有活动 AI 任务，请结束后重试".into()); }
    let root = paths::project_dir(app, mod_id)?.join("source");
    checked_path(&root, "", &[])?;
    let mut exclusions = vec![".delivery".into(), ".git".into()];
    exclusions.extend(crate::skills::registry::resolve_for_game(app, &project.game_id)?
        .into_iter().flat_map(|view| view.definition.exclude_paths));
    Ok((root, exclusions))
}

fn journal(app: &tauri::AppHandle, mod_id: &str, item_id: &str, kind: &str, status: &str, operation: &SourceOperation) -> Result<(), String> {
    repository::file_operation(app, item_id, mod_id, kind, status, &serde_json::to_value(operation).map_err(|e| e.to_string())?)
}

fn staging_path(project: &Path, item_id: &str, operation: &SourceOperation) -> Result<PathBuf, String> {
    paths::validate_id(&operation.operation_id)?;
    paths::validate_id(item_id)?;
    let expected = format!(".source-imports/{}/{item_id}.tmp", operation.operation_id);
    if operation.temporary_relative_path.as_deref() != Some(&expected) { return Err("导入临时路径身份不一致".into()); }
    let mut path = project.to_path_buf();
    for ancestor in project.ancestors() { paths::reject_redirect(ancestor)?; }
    for component in Path::new(&expected).components() {
        path.push(component);
        if std::fs::symlink_metadata(&path).is_ok() { paths::reject_redirect(&path)?; }
    }
    Ok(path)
}

fn create_staging_parents(project: &Path, operation_id: &str) -> Result<(), String> {
    paths::validate_id(operation_id)?;
    let mut directory = project.to_path_buf();
    for name in [".source-imports", operation_id] {
        directory.push(name);
        match std::fs::create_dir(&directory) {
            Ok(()) => {},
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {},
            Err(error) => return Err(error.to_string()),
        }
        paths::reject_redirect(&directory)?;
        if !directory.is_dir() { return Err("导入临时目录被其他文件占用".into()); }
    }
    Ok(())
}

fn finish(app: &tauri::AppHandle, mod_id: &str, root: &Path, exclusions: &[String], reservation: &crate::pi::workspace::NativeReservation,
    operation_id: String, before: String, items: Vec<SourceMutationItem>) -> Result<SourceMutationResult, String> {
    let fingerprint = changes::source_fingerprint(root, exclusions).map_err(|e| format!("文件操作已经执行，但源码状态读取失败：{e}；请刷新核对"))?;
    let source_revision = reservation.observe_fingerprint(fingerprint.clone())?;
    let result = SourceMutationResult { operation_id, source_revision, items };
    let logs = root.parent().ok_or("项目目录缺失")?.join("logs");
    paths::reject_redirect(&logs)?;
    let log = logs.join("source-operations.jsonl");
    if log.exists() { paths::reject_redirect(&log)?; }
    let logged = (|| -> Result<(), String> {
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(log).map_err(|e| e.to_string())?;
        let items: Vec<_> = result.items.iter().map(|item| json!({"target":item.target,"status":item.status,"message":item.message})).collect();
        writeln!(file, "{}", json!({"operationId":result.operation_id,"beforeFingerprint":before,"fingerprint":fingerprint,"sourceRevision":source_revision,"items":items,"timestamp":mods::now()?})).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())
    })();
    app.emit("project-source-changed", json!({"modId":mod_id,"sourceRevision":source_revision})).map_err(|e| e.to_string())?;
    logged.map_err(|e| format!("文件操作已经执行，但日志保存失败：{e}；请刷新核对"))?;
    Ok(result)
}

#[tauri::command]
pub async fn pick_source_import_files(app: tauri::AppHandle) -> Result<Option<Vec<String>>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        app.dialog().file().set_title("导入文件").blocking_pick_files().map(|files| {
            files.into_iter().map(|file| file.into_path().map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string())).collect()
        }).transpose()
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_source_files(app: tauri::AppHandle, mod_id: String, target_directory: String, items: Vec<SourceImportItem>) -> Result<SourceMutationResult, String> {
    tauri::async_runtime::spawn_blocking(move || import(&app, &mod_id, &target_directory, items)).await.map_err(|e| e.to_string())?
}

fn import(app: &tauri::AppHandle, mod_id: &str, directory: &str, items: Vec<SourceImportItem>) -> Result<SourceMutationResult, String> {
    let reservation = app.state::<Workspaces>().try_reserve_native(mod_id, NativeOperation::SourceChange)?;
    let (root, exclusions) = project_input(app, mod_id)?;
    import_reserved(app,mod_id,directory,items,root,exclusions,reservation,None)
}

fn import_reserved(app:&tauri::AppHandle,mod_id:&str,directory:&str,items:Vec<SourceImportItem>,root:PathBuf,exclusions:Vec<String>,reservation:crate::pi::workspace::NativeReservation,expected_hash:Option<&str>)->Result<SourceMutationResult,String> {
    let before = changes::source_fingerprint(&root, &exclusions)?;
    reservation.observe_fingerprint(before.clone())?;
    let operation_id = uuid::Uuid::new_v4().to_string();
    let mut names = std::collections::BTreeSet::new();
    // 整批先验证目标，重名项跳过；独立文件的 I/O 失败不取消其他项。
    let prepared: Vec<_> = items.into_iter().map(|item| {
        let target = target_path(&root, directory, &item.target_name, &exclusions);
        let duplicate = !names.insert(item.target_name.to_lowercase());
        (item, target, duplicate)
    }).collect();
    let mut results = Vec::new();
    for (item, target, duplicate) in prepared {
        let mut result = SourceMutationItem { source: item.source_path.clone(), target: item.target_name.clone(), status: "failed", message: None };
        let copied = (|| -> Result<(), String> {
            let (relative, target) = target?;
            result.target = relative.clone();
            if duplicate || target.exists() { result.status = "conflict"; return Err("同名文件已存在，请改名或跳过".into()); }
            let original = Path::new(&item.source_path);
            if !original.is_absolute() { return Err("导入来源必须为文件选择器返回的绝对路径".into()); }
            io::validate_plain_file(original)?;
            let item_id = uuid::Uuid::new_v4().to_string();
            let mut operation = SourceOperation { operation_id: operation_id.clone(), source_relative_path: None,
                target_relative_path: relative, temporary_relative_path: Some(format!(".source-imports/{operation_id}/{item_id}.tmp")),
                before_hash: None, result_hash: None, phase: "allocated".into() };
            journal(app, mod_id, &item_id, "source_import", "pending", &operation)?;
            let project = root.parent().ok_or("项目目录缺失")?;
            let staged = staging_path(project, &item_id, &operation)?;
            create_staging_parents(project, &operation_id)?;
            operation.result_hash = Some(io::copy_to_staging(original, &staged)?);
            if expected_hash.is_some_and(|hash|operation.result_hash.as_deref()!=Some(hash)) {return Err("旧素材指纹不一致，未导入源码".into());}
            operation.phase = "ready".into();
            journal(app, mod_id, &item_id, "source_import", "pending", &operation)?;
            checked_path(&root, &operation.target_relative_path, &exclusions)?;
            if let Err(error) = io::move_no_replace(&staged, &target) {
                if target.exists() { result.status = "conflict"; }
                return Err(error);
            }
            result.status = "imported";
            operation.phase = "committed".into();
            if let Err(error) = journal(app, mod_id, &item_id, "source_import", "done", &operation) {
                result.message = Some(format!("文件已导入，操作记录待恢复：{error}"));
            }
            Ok(())
        })();
        if let Err(error) = copied { result.message = Some(error); }
        results.push(result);
    }
    finish(app, mod_id, &root, &exclusions, &reservation, operation_id, before, results)
}

pub fn restore_legacy(app:&tauri::AppHandle,mod_id:&str,path:&Path,name:&str,hash:&str)->Result<serde_json::Value,String>{
    let reservation=app.state::<Workspaces>().try_reserve_native(mod_id,NativeOperation::SourceChange)?;
    let (root,exclusions)=project_input(app,mod_id)?;
    io::validate_plain_file(path)?;
    if changes::file_hash(path)?!=hash{return Err("旧素材指纹不一致，原文件保留".into());}
    for relative in ["assets","assets/legacy"] {
        let directory=checked_path(&root,relative,&exclusions)?;
        match std::fs::create_dir(&directory){Ok(())=>{},Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists && directory.is_dir()=>paths::reject_redirect(&directory)?,Err(e)=>return Err(e.to_string())}
    }
    let (relative,target)=target_path(&root,"assets/legacy",name,&exclusions)?;
    match legacy_target_status(&target,hash)? {
        "alreadyPresent"=>return Ok(json!({"sourcePath":relative,"status":"alreadyPresent"})),
        "conflict"=>return Ok(json!({"sourcePath":relative,"status":"conflict"})),
        _=>{},
    }
    let result=import_reserved(app,mod_id,"assets/legacy",vec![SourceImportItem{source_path:path.to_string_lossy().into_owned(),target_name:name.into()}],root,exclusions,reservation,Some(hash))?;
    let item=result.items.first().ok_or("素材恢复没有文件结果")?;
    if item.status!="imported" {return Err(item.message.clone().unwrap_or_else(||"素材未恢复".into()));}
    Ok(json!({"sourcePath":relative,"status":"restored","sourceRevision":result.source_revision}))
}

fn legacy_target_status(target:&Path,hash:&str)->Result<&'static str,String>{
    match std::fs::symlink_metadata(target){Ok(_)=>{},Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok("missing"),Err(e)=>return Err(e.to_string())}
    io::validate_plain_file(target)?;
    Ok(if changes::file_hash(target)?==hash{"alreadyPresent"}else{"conflict"})
}

#[tauri::command]
pub async fn move_source_file(app: tauri::AppHandle, mod_id: String, source: String, target_directory: String, target_name: String) -> Result<SourceMutationResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let reservation = app.state::<Workspaces>().try_reserve_native(&mod_id, NativeOperation::SourceChange)?;
        let (root, exclusions) = project_input(&app, &mod_id)?;
        let before = changes::source_fingerprint(&root, &exclusions)?;
        reservation.observe_fingerprint(before.clone())?;
        let original = checked_path(&root, &source, &exclusions)?;
        io::validate_plain_file(&original)?;
        let (relative, target) = target_path(&root, &target_directory, &target_name, &exclusions)?;
        if source.to_lowercase() == relative.to_lowercase() { return Err("同路径或仅修改大小写的改名暂不支持".into()); }
        let operation_id = uuid::Uuid::new_v4().to_string();
        let mut item = SourceMutationItem { source, target: relative.clone(), status: "conflict", message: None };
        if target.exists() { item.message = Some("目标已存在，请改名或跳过".into()); }
        else {
            let hash = changes::file_hash(&original)?;
            let mut operation = SourceOperation { operation_id: operation_id.clone(), source_relative_path: Some(item.source.clone()), target_relative_path: relative,
                temporary_relative_path: None, before_hash: Some(hash.clone()), result_hash: Some(hash), phase: "ready".into() };
            journal(&app, &mod_id, &operation_id, "source_move", "pending", &operation)?;
            match io::move_no_replace(&original, &target) {
                Ok(()) => {
                    item.status = "moved";
                    operation.phase = "committed".into();
                    if let Err(error) = journal(&app, &mod_id, &operation_id, "source_move", "done", &operation) { item.message = Some(format!("文件已移动，操作记录待恢复：{error}")); }
                },
                Err(error) => { item.status = if target.exists() { "conflict" } else { "failed" }; item.message = Some(error); },
            }
        }
        finish(&app, &mod_id, &root, &exclusions, &reservation, operation_id, before, vec![item])
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_source_directory(app: tauri::AppHandle, mod_id: String, parent_directory: String, name: String) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let reservation = app.state::<Workspaces>().try_reserve_native(&mod_id, NativeOperation::SourceChange)?;
        let (root, exclusions) = project_input(&app, &mod_id)?;
        let (relative, target) = target_path(&root, &parent_directory, &name, &exclusions)?;
        match std::fs::create_dir(&target) {
            Ok(()) => {},
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && target.is_dir() => { paths::reject_redirect(&target)?; },
            Err(error) => return Err(error.to_string()),
        }
        let revision = reservation.observe_fingerprint(changes::source_fingerprint(&root, &exclusions)?)?;
        app.emit("project-source-changed", json!({"modId":mod_id,"sourceRevision":revision})).map_err(|e| e.to_string())?;
        Ok(json!({"path":relative,"sourceRevision":revision}))
    }).await.map_err(|e| e.to_string())?
}

fn recover_operation(project: &Path, item_id: &str, kind: &str, operation: &SourceOperation, exclusions: &[String]) -> Result<&'static str, String> {
    paths::validate_id(&operation.operation_id)?;
    let source = project.join("source");
    let target = checked_path(&source, &operation.target_relative_path, exclusions)?;
    let matches_hash = |path: &Path| -> Result<bool, String> {
        io::validate_plain_file(path)?;
        Ok(operation.result_hash.as_ref().is_some_and(|hash| changes::file_hash(path).as_ref().ok() == Some(hash)))
    };
    if !matches!(operation.phase.as_str(), "allocated" | "ready" | "committed") { return Err("文件操作阶段无效".into()); }
    if kind == "source_import" {
        let staged = staging_path(project, item_id, operation)?;
        if target.exists() && !staged.exists() && matches_hash(&target)? { return Ok("reconciled"); }
        if !target.exists() && staged.exists() && matches_hash(&staged)? {
            std::fs::remove_file(staged).map_err(|e| e.to_string())?;
            return Ok("interrupted");
        }
        if !target.exists() && !staged.exists() { return Ok("interrupted"); }
    } else if kind == "source_move" {
        let original = checked_path(&source, operation.source_relative_path.as_deref().ok_or("移动缺少原路径")?, exclusions)?;
        if !original.exists() && target.exists() && matches_hash(&target)? { return Ok("reconciled"); }
        if original.exists() && !target.exists() && matches_hash(&original)? { return Ok("interrupted"); }
    } else { return Err("未知源码文件恢复类型".into()); }
    Ok("requires_review")
}

pub fn reconcile(app: &tauri::AppHandle, mod_id: &str, item_id: &str, kind: &str, payload: &serde_json::Value) -> Result<(), String> {
    let operation: SourceOperation = serde_json::from_value(payload.clone()).map_err(|e| e.to_string())?;
    let project = mods::read_project(app, mod_id)?;
    let exclusions: Vec<_> = crate::skills::registry::resolve_for_game(app, &project.game_id)?.into_iter().flat_map(|view| view.definition.exclude_paths).collect();
    let status = recover_operation(&paths::project_dir(app, mod_id)?, item_id, kind, &operation, &exclusions).unwrap_or("requires_review");
    repository::file_operation(app, item_id, mod_id, kind, status, payload)
}
