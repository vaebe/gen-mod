use super::file_operations::file_operation;
use crate::pi::session_repository::steering;
use crate::storage::{database, paths, repository::{decode, read, list}};
use rusqlite::params;

pub fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    database::initialize(app)?;
    crate::project_lifecycle::recovery::recover_pending(app)?;
    interrupt_previous(app)?;
    database::connect(app)?.execute("UPDATE submission_requests SET status='rejected',error='应用已重启，提交未启动' WHERE status='pending' AND task_id IS NULL",[]).map_err(|e|e.to_string())?;
    reconcile_files(app)
}

/// 重启只修复元数据，不恢复旧进程或重放输入。
pub fn interrupt_previous(app: &tauri::AppHandle) -> Result<(), String> {
    reconcile_steering(app)?;
    let db = database::connect(app)?;
    let finished = crate::storage::mods::now()?;
    for project in list(app)? {
        for version in &project.versions {
            db.execute("UPDATE tasks SET payload=json_set(payload,'$.registeredVersionId',?2) WHERE task_id=?1 AND mod_id=?3",params![version.task_id,version.version_id,project.mod_id]).map_err(|e|e.to_string())?;
        }
    }
    db.execute("UPDATE tasks SET status='interrupted',finished_at=?1,payload=json_set(payload,'$.status','interrupted','$.finishedAt',?1) WHERE status IN ('preparing','running','stopping')", [&finished]).map_err(|e| e.to_string())?;
    db.execute("UPDATE projects SET payload=json_set(payload,'$.developmentStatus','failed'),revision=revision+1 WHERE json_extract(payload,'$.developmentStatus')='running'",[]).map_err(|e|e.to_string())?;
    db.execute("UPDATE steering_inputs SET status='unknown',payload=json_set(payload,'$.status','unknown') WHERE status IN ('submitted','queued') OR (status='confirmed' AND json_extract(payload,'$.entryId') IS NULL)", []).map_err(|e| e.to_string())?;
    Ok(())
}


/// 启动核对孤立产物与中断操作；不删除已有目录，不重新执行安装。
pub fn reconcile_files(app: &tauri::AppHandle) -> Result<(),String> {
    let db = database::connect(app)?;
    let pending = {
        let mut query = db.prepare("SELECT operation_id,mod_id,kind,payload FROM file_operations WHERE status='pending'").map_err(|e|e.to_string())?;
        let rows = query.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).map_err(|e|e.to_string())?;
        rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?
    };
    for (id,mod_id,kind,payload) in pending {
        let payload: serde_json::Value = decode(payload)?;
        if kind=="project_delete" {continue;}
        if kind=="install" {
            let record = serde_json::from_value(payload["record"].clone()).map_err(|e|e.to_string())?;
            crate::install::repository::recover_attempt_on(&mut database::connect(app)?, &mod_id, record)?;
            file_operation(app,&id,&mod_id,&kind,"reconciled",&payload)?;
        } else if kind=="delete" {
            let version_id=payload["versionId"].as_str().ok_or("删除恢复记录缺少版本 ID")?; paths::validate_id(version_id)?;
            let tombstone=payload["tombstone"].as_str().ok_or("删除恢复记录缺少目录")?;
            if !tombstone.starts_with(".delete-") { return Err("删除恢复目录无效".into()); }
            paths::validate_id(tombstone.trim_start_matches(".delete-"))?;
            let artifacts=paths::project_dir(app,&mod_id)?.join("artifacts"); paths::reject_redirect(&artifacts)?;
            let temporary=artifacts.join(tombstone); let original=artifacts.join(version_id);
            let project=read(app,&mod_id)?;
            crate::artifacts::delivery_metadata::mark_deleted_on(&db,&mod_id,version_id,!project.versions.iter().any(|v|v.version_id==version_id))?;
            if project.versions.iter().any(|v|v.version_id==version_id) && temporary.exists() && !original.exists() {
                paths::reject_redirect(&temporary)?;
                std::fs::rename(&temporary,&original).map_err(|e|e.to_string())?;
                file_operation(app,&id,&mod_id,&kind,"reconciled",&payload)?;
            } else { file_operation(app,&id,&mod_id,&kind,"requires_review",&payload)?; }
        } else if matches!(kind.as_str(),"source_import"|"source_move") {
            crate::source_files::reconcile(app,&mod_id,&id,&kind,&payload)?;
        } else if kind=="session_delete" {
            let session_id=payload["sessionId"].as_str().ok_or("会话删除记录缺少身份")?; paths::validate_id(session_id)?;
            let original=payload["original"].as_str().ok_or("会话删除记录缺少文件")?;
            let temporary=payload["tombstone"].as_str().ok_or("会话删除记录缺少临时文件")?;
            let tombstone_id=temporary.strip_prefix("deleted-").and_then(|v|v.strip_suffix(".tmp")).ok_or("会话临时文件无效")?;
            paths::validate_id(tombstone_id)?;
            let relative=crate::changes::relative_path(original)?;
            if relative.components().count()!=1 { return Err("会话删除恢复文件越界".into()); }
            let root=paths::project_dir(app,&mod_id)?.join("logs/sessions"); paths::reject_redirect(root.parent().ok_or("会话目录无效")?)?; paths::reject_redirect(&root)?;
            let original=root.join(relative); let temporary=root.join(temporary);
            let active=read(app,&mod_id)?.sessions.iter().any(|s|s.session_id==session_id);
            if temporary.exists() {
                paths::reject_redirect(&temporary)?;
                if active && !original.exists() { std::fs::rename(&temporary,&original).map_err(|e|e.to_string())?; }
                else if !active { std::fs::remove_file(&temporary).map_err(|e|e.to_string())?; }
                else { file_operation(app,&id,&mod_id,&kind,"requires_review",&payload)?; continue; }
            }
            file_operation(app,&id,&mod_id,&kind,"reconciled",&payload)?;
        } else if kind=="register" {
            let project=read(app,&mod_id)?;
            let registered=project.versions.iter().any(|v|payload["versionId"]==v.version_id);
            if let Some(version)=project.versions.iter().find(|v|payload["versionId"]==v.version_id){crate::artifacts::delivery_metadata::ensure_anchor_on(&db,&mod_id,version)?;crate::artifacts::delivery_metadata::mark_deleted_on(&db,&mod_id,&version.version_id,false)?;}
            file_operation(app,&id,&mod_id,&kind,if registered {"reconciled"} else {"requires_review"},&payload)?;
        } else { file_operation(app,&id,&mod_id,&kind,"requires_review",&payload)?; }
    }
    for project in list(app)? {
        let artifacts=paths::project_dir(app,&project.mod_id)?.join("artifacts");
        if !artifacts.exists() { continue; } paths::reject_redirect(&artifacts)?;
        for entry in std::fs::read_dir(artifacts).map_err(|e|e.to_string())? {
            let entry=entry.map_err(|e|e.to_string())?; let id=entry.file_name().to_string_lossy().into_owned();
            if paths::validate_id(&id).is_ok() && !project.versions.iter().any(|v|v.version_id==id) {
                file_operation(app,&format!("orphan-{id}"),&project.mod_id,"orphan","requires_review",&serde_json::json!({"versionId":id,"reason":"目录没有登记元数据，保留文件待核对"}))?;
            }
        }
    }
    Ok(())
}

/// 仅补齐已经确认且保存消息时间的关联；未响应输入不按正文猜测认领。
fn reconcile_steering(app: &tauri::AppHandle) -> Result<(),String> {
    let db=database::connect(app)?;
    let inputs={
        let mut query=db.prepare("SELECT payload FROM steering_inputs WHERE status='confirmed' AND json_extract(payload,'$.entryId') IS NULL").map_err(|e|e.to_string())?;
        let rows=query.query_map([],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;
        rows.map(|r|decode::<serde_json::Value>(r.map_err(|e|e.to_string())?)).collect::<Result<Vec<_>,_>>()?
    };
    for mut input in inputs {
        let mod_id=input["modId"].as_str().ok_or("引导归属缺失")?;
        let session_id=input["sessionId"].as_str().ok_or("引导会话缺失")?;
        let project=read(app,mod_id)?;
        let Some(session)=project.sessions.iter().find(|s|s.session_id==session_id) else {continue;};
        let file=std::path::Path::new(&session.session_file);
        if !file.is_file() {continue;} paths::reject_redirect(file)?;
        use std::io::BufRead;
        let mut matches=vec![];
        for line in std::io::BufReader::new(std::fs::File::open(file).map_err(|e|e.to_string())?).lines() {
            let line=line.map_err(|e|e.to_string())?;
            let Ok(entry)=serde_json::from_str::<serde_json::Value>(&line) else {continue;};
            if entry["type"]=="message" && entry["message"]["role"]=="user" && entry["message"]["timestamp"]==input["messageTimestamp"] && crate::pi::bridge::input_fingerprint(&entry["message"],None).is_ok_and(|hash|input["fingerprint"]==hash) { matches.push(entry["id"].clone()); }
        }
        if matches.len()==1 {input["entryId"]=matches.remove(0);steering(app,&input)?;}
    }
    Ok(())
}
