use super::{database, models::ModProject, paths};
use crate::pi::{tasks::TaskRecord, logs::RuntimeLogEntry};
use rusqlite::{params, Connection, OptionalExtension};

pub fn encode<T: serde::Serialize>(value: &T) -> Result<String, String> { serde_json::to_string(value).map_err(|e| e.to_string()) }
pub fn decode<T: serde::de::DeserializeOwned>(value: String) -> Result<T, String> { serde_json::from_str(&value).map_err(|e| e.to_string()) }

/// 从 projects 与未归档 sessions 组合既有 IPC DTO。
pub fn read_on(db: &Connection, id: &str) -> Result<ModProject, String> {
    let (payload, revision, archived): (String, i64, bool) = db.query_row("SELECT payload,revision,archived FROM projects WHERE mod_id=?1", [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e| e.to_string())?.ok_or_else(|| {
        let error: Option<String> = db.query_row("SELECT error FROM legacy_imports WHERE mod_id=?1", [id], |r| r.get(0)).optional().ok().flatten().flatten();
        error.map_or_else(|| "项目不存在".into(), |e| format!("项目迁移失败，禁止执行：{e}"))
    })?;
    let mut payload: serde_json::Value = decode(payload)?;
    let mut statement = db.prepare("SELECT payload FROM sessions WHERE mod_id=?1 AND deleted=0 ORDER BY rowid").map_err(|e| e.to_string())?;
    let sessions: Vec<serde_json::Value> = statement.query_map([id], |r| r.get::<_,String>(0)).map_err(|e| e.to_string())?.map(|r| decode(r.map_err(|e| e.to_string())?)).collect::<Result<_,_>>()?;
    // projects.payload 不存 sessions，必须先组合完整 DTO，不能提前反序列化。
    payload.as_object_mut().ok_or("项目元数据不是 JSON 对象")?.insert("sessions".into(), serde_json::Value::Array(sessions));
    let mut project: ModProject = serde_json::from_value(payload).map_err(|e| e.to_string())?;
    project.revision = revision;
    project.archived = archived;
    Ok(project)
}
pub fn read(app: &tauri::AppHandle, id: &str) -> Result<ModProject, String> {
    paths::validate_id(id)?;
    let db=database::connect(app)?;
    let project=read_on(&db,id)?;
    paths::reject_redirect(&paths::mods_root(app)?)?;
    let directory=paths::project_dir(app,id)?;
    match std::fs::symlink_metadata(&directory) {
        Ok(_)=>paths::reject_redirect(&directory)?,
        Err(e) if e.kind()==std::io::ErrorKind::NotFound && crate::project_lifecycle::repository::has_unresolved_removal_on(&db,id)?=>{},
        Err(e)=>return Err(e.to_string()),
    }
    Ok(project)
}

/// 乐观 revision 拒绝陈旧整份 DTO；调用方需在存储锁内读取并提交。
pub fn save_on(db: &Connection, project: &ModProject, creating: bool) -> Result<(), String> {
    if !creating { crate::project_lifecycle::repository::ensure_writable_on(db,&project.mod_id)?; }
    else if db.query_row::<bool,_,_>("SELECT EXISTS(SELECT 1 FROM project_removals WHERE mod_id=?1)",[&project.mod_id],|r|r.get(0)).map_err(|e|e.to_string())? { return Err("已移除的项目不能重新导入".into()); }
    let initial: Option<String> = if !creating && !project.sessions.is_empty() {
        db.query_row("SELECT json_extract(payload,'$.draftRequirement') FROM projects WHERE mod_id=?1 AND NOT EXISTS(SELECT 1 FROM sessions WHERE mod_id=?1 AND deleted=0)",[&project.mod_id],|r|r.get(0)).optional().map_err(|e|e.to_string())?.flatten()
    } else { None };
    let mut payload = serde_json::to_value(project).map_err(|e| e.to_string())?;
    payload.as_object_mut().ok_or("项目 DTO 无效")?.remove("sessions");
    // initialDraft 仅用于尚无会话的新项目，其他草稿存入 sessions。
    if !project.sessions.is_empty() { payload["draftRequirement"] = "".into(); }
    if creating {
        db.execute("INSERT INTO projects(mod_id,payload,revision) VALUES(?1,?2,0)", params![project.mod_id,encode(&payload)?]).map_err(|e| e.to_string())?;
    } else if db.execute("UPDATE projects SET payload=?2,revision=revision+1 WHERE mod_id=?1 AND revision=?3", params![project.mod_id,encode(&payload)?,project.revision]).map_err(|e| e.to_string())? != 1 { return Err("项目元数据已更新，请重新读取后操作".into()); }
    db.execute("UPDATE sessions SET deleted=1 WHERE mod_id=?1", [&project.mod_id]).map_err(|e| e.to_string())?;
    for session in &project.sessions {
        db.execute("INSERT INTO sessions(mod_id,session_id,payload) VALUES(?1,?2,?3) ON CONFLICT(mod_id,session_id) DO UPDATE SET payload=excluded.payload,deleted=0", params![project.mod_id,session.session_id,encode(session)?]).map_err(|e| e.to_string())?;
    }
    if let (Some(draft),Some(session)) = (initial,project.sessions.first()) {
        db.execute("UPDATE sessions SET draft=?3 WHERE mod_id=?1 AND session_id=?2",params![project.mod_id,session.session_id,draft]).map_err(|e|e.to_string())?;
    }
    Ok(())
}
pub fn save(app: &tauri::AppHandle, project: &mut ModProject) -> Result<(), String> {
    let mut db = database::connect(app)?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    save_on(&tx, project, false)?;
    tx.commit().map_err(|e| e.to_string())?;
    project.revision += 1;
    Ok(())
}
pub fn create(app: &tauri::AppHandle, project: &ModProject) -> Result<(), String> {
    let mut db = database::connect(app)?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    save_on(&tx,project,true)?;
    crate::project_context::repository::initialize_on(&tx,&project.mod_id)?;
    tx.commit().map_err(|e| e.to_string())
}
/// 运行事件的局部更新在短事务内重读，避免版本、标题和部署状态互相覆盖。
pub fn update(app: &tauri::AppHandle, id: &str, operation: impl FnOnce(&mut ModProject) -> Result<(),String>) -> Result<ModProject,String> {
    let mut db = database::connect(app)?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    let mut project = read_on(&tx,id)?;
    crate::project_lifecycle::repository::ensure_writable_on(&tx,id)?;
    operation(&mut project)?;
    save_on(&tx,&project,false)?;
    tx.commit().map_err(|e|e.to_string())?;
    project.revision += 1;
    Ok(project)
}
pub fn list(app: &tauri::AppHandle) -> Result<Vec<ModProject>, String> {
    let db = database::connect(app)?;
    let mut query = db.prepare("SELECT mod_id FROM projects WHERE archived=0 ORDER BY json_extract(payload,'$.updatedAt') DESC").map_err(|e| e.to_string())?;
    let ids = query.query_map([], |r| r.get::<_,String>(0)).map_err(|e| e.to_string())?;
    ids.map(|id| read_on(&db,&id.map_err(|e| e.to_string())?)).collect()
}
pub fn task_on(db: &Connection, task: &TaskRecord) -> Result<(), String> {
    db.execute("INSERT INTO tasks(task_id,mod_id,session_id,status,created_at,finished_at,payload) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(task_id) DO UPDATE SET status=excluded.status,finished_at=excluded.finished_at,payload=excluded.payload", params![task.context.task_id,task.context.mod_id,task.context.session_id,task.status,task.context.created_at,task.finished_at,encode(task)?]).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn save_task(app: &tauri::AppHandle, task: &TaskRecord) -> Result<(), String> { task_on(&database::connect(app)?,task) }
pub fn tasks(app: &tauri::AppHandle, mod_id: &str) -> Result<Vec<TaskRecord>, String> {
    let db = database::connect(app)?;
    let mut query = db.prepare("SELECT payload FROM tasks WHERE mod_id=?1 ORDER BY created_at DESC").map_err(|e| e.to_string())?;
    let rows = query.query_map([mod_id], |r| r.get::<_,String>(0)).map_err(|e| e.to_string())?.map(|r| decode(r.map_err(|e| e.to_string())?)).collect::<Result<Vec<_>,_>>();
    rows
}
pub fn task(app: &tauri::AppHandle, mod_id: &str, task_id: &str) -> Result<TaskRecord, String> {
    let payload = database::connect(app)?.query_row("SELECT payload FROM tasks WHERE task_id=?1 AND mod_id=?2",params![task_id,mod_id],|r| r.get(0)).map_err(|e| e.to_string())?;
    decode(payload)
}
pub fn log_on(db: &Connection, mod_id: &str, session_id: &str, entry: &RuntimeLogEntry) -> Result<(), String> {
    if db.execute("INSERT INTO runtime_logs(mod_id,session_id,task_id,timestamp,payload) SELECT ?1,?2,?3,?4,?5 WHERE EXISTS(SELECT 1 FROM projects WHERE mod_id=?1 AND archived=0) AND NOT EXISTS(SELECT 1 FROM project_removals WHERE mod_id=?1) AND NOT EXISTS(SELECT 1 FROM file_operations WHERE mod_id=?1 AND kind='project_delete' AND status IN ('pending','requires_review'))",params![mod_id,session_id,entry.task_id,entry.timestamp,encode(entry)?]).map_err(|e| e.to_string())?!=1 {return Err("项目已不可写，未保存迟到日志".into());}
    db.execute("UPDATE sessions SET last_activity_at=?3 WHERE mod_id=?1 AND session_id=?2",params![mod_id,session_id,entry.timestamp]).map_err(|e|e.to_string())?;
    Ok(())
}
pub fn log(app: &tauri::AppHandle, mod_id: &str, session_id: &str, entry: &RuntimeLogEntry) -> Result<(), String> { log_on(&database::connect(app)?,mod_id,session_id,entry) }
pub fn logs(app: &tauri::AppHandle, mod_id: &str, session_id: &str) -> Result<Vec<RuntimeLogEntry>, String> {
    let db = database::connect(app)?;
    let mut query = db.prepare("SELECT payload FROM (SELECT log_id,payload FROM runtime_logs WHERE mod_id=?1 AND session_id=?2 ORDER BY log_id DESC LIMIT 200) ORDER BY log_id").map_err(|e| e.to_string())?;
    let rows = query.query_map(params![mod_id,session_id], |r| r.get::<_,String>(0)).map_err(|e| e.to_string())?.map(|r| decode(r.map_err(|e| e.to_string())?)).collect::<Result<Vec<_>,_>>();
    rows
}
/// 重启只修复元数据，不恢复旧进程或重放输入。
pub fn interrupt_previous(app: &tauri::AppHandle) -> Result<(), String> {
    reconcile_steering(app)?;
    let db = database::connect(app)?;
    let finished = super::mods::now()?;
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

/// 草稿与强度属于目标会话，不通过整份项目 DTO 覆盖。
pub fn save_preferences(app: &tauri::AppHandle, mod_id: &str, session_id: &str, draft: Option<&str>, thinking: Option<&str>) -> Result<(),String> {
    let db = database::connect(app)?;
    crate::project_lifecycle::repository::ensure_writable_on(&db,mod_id)?;
    let count = db.execute("UPDATE sessions SET draft=COALESCE(?3,draft),thinking_level=COALESCE(?4,thinking_level) WHERE mod_id=?1 AND session_id=?2 AND deleted=0",params![mod_id,session_id,draft,thinking]).map_err(|e| e.to_string())?;
    if count != 1 { return Err("会话不存在或已删除".into()); } Ok(())
}
pub fn model_preferences(app: &tauri::AppHandle, mod_id: &str, session_id: &str) -> Result<(Option<crate::config::ModelSelection>, String), String> {
    let db = database::connect(app)?;
    let (provider, model, thinking): (Option<String>, Option<String>, String) = db.query_row("SELECT provider_id,model_id,thinking_level FROM sessions WHERE mod_id=?1 AND session_id=?2 AND deleted=0", params![mod_id,session_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e| e.to_string())?;
    let selection = match (provider, model) {
        (Some(provider_id), Some(model_id)) => Some(crate::config::ModelSelection { provider_id, model_id }),
        (None, None) => None,
        _ => return Err("会话模型记录不完整，请检查数据库".into()),
    };
    Ok((selection, thinking))
}

pub fn save_model(app: &tauri::AppHandle, mod_id: &str, session_id: &str, selection: Option<&crate::config::ModelSelection>) -> Result<(), String> {
    let mut db = database::connect(app)?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    crate::project_lifecycle::repository::ensure_writable_on(&tx,mod_id)?;
    // 更换模型后旧的上下文窗口统计不再适用，不能在重开聊天时继续显示。
    tx.execute("DELETE FROM session_stats WHERE mod_id=?1 AND session_id=?2 AND EXISTS(SELECT 1 FROM sessions WHERE mod_id=?1 AND session_id=?2 AND (provider_id IS NOT ?3 OR model_id IS NOT ?4))", params![mod_id,session_id,selection.map(|value| &value.provider_id),selection.map(|value| &value.model_id)]).map_err(|e| e.to_string())?;
    let count = tx.execute("UPDATE sessions SET provider_id=?3,model_id=?4 WHERE mod_id=?1 AND session_id=?2 AND deleted=0", params![mod_id,session_id,selection.map(|value| &value.provider_id),selection.map(|value| &value.model_id)]).map_err(|e| e.to_string())?;
    if count != 1 { return Err("会话不存在或已删除".into()); }
    tx.commit().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn read_session_metadata(app: tauri::AppHandle, mod_id: String, session_id: String) -> Result<serde_json::Value,String> {
    read(&app,&mod_id)?.sessions.iter().find(|s|s.session_id==session_id).ok_or("会话不属于项目")?;
    let db = database::connect(&app)?;
    let (draft,thinking,generation):(String,String,i64) = db.query_row("SELECT draft,thinking_level,generation FROM sessions WHERE mod_id=?1 AND session_id=?2",params![mod_id,session_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
    let stats: Option<String> = db.query_row("SELECT payload FROM session_stats WHERE mod_id=?1 AND session_id=?2",params![mod_id,session_id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    let (selection, _) = model_preferences(&app, &mod_id, &session_id)?;
    let model_state = crate::config::session_model_state(&app, selection, &thinking);
    Ok(serde_json::json!({"draft":draft,"thinkingLevel":thinking,"modelState":model_state,"generation":generation,"contextUsage":stats.map(decode::<serde_json::Value>).transpose()?}))
}
pub fn save_stats(app: &tauri::AppHandle, mod_id: &str, session_id: &str, generation: u64, usage: &serde_json::Value) -> Result<(),String> {
    let db = database::connect(app)?;
    let sampled = super::mods::now()?;
    let mut usage = usage.clone();
    if usage["tokens"].is_null() {
        let old: Option<String> = db.query_row("SELECT payload FROM session_stats WHERE mod_id=?1 AND session_id=?2",params![mod_id,session_id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        if let Some(old) = old {
            let previous: serde_json::Value=decode(old)?;
            if usage["contextWindow"].is_null() { usage["contextWindow"]=previous["contextWindow"].clone(); }
            usage["previous"]=if previous["tokens"].is_null() {previous["previous"].clone()} else {previous};
        }
    }
    usage["generation"] = generation.into(); usage["sampledAt"] = sampled.clone().into(); usage["estimated"] = true.into();
    db.execute("INSERT INTO session_stats VALUES(?1,?2,?3,?4) ON CONFLICT(mod_id,session_id) DO UPDATE SET sampled_at=excluded.sampled_at,payload=excluded.payload",params![mod_id,session_id,sampled,encode(&usage)?]).map_err(|e|e.to_string())?; Ok(())
}
#[tauri::command]
pub fn list_migration_errors(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>,String> {
    let db = database::connect(&app)?;
    let mut query = db.prepare("SELECT mod_id,error FROM legacy_imports WHERE error IS NOT NULL").map_err(|e|e.to_string())?;
    let rows = query.query_map([],|r|Ok(serde_json::json!({"modId":r.get::<_,String>(0)?,"error":r.get::<_,String>(1)?}))).map_err(|e|e.to_string())?;
    rows.map(|r|r.map_err(|e|e.to_string())).collect()
}

pub fn steering(app: &tauri::AppHandle, input: &serde_json::Value) -> Result<(),String> {
    let mut db = database::connect(app)?;
    let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    crate::project_lifecycle::repository::ensure_writable_on(&tx,input["modId"].as_str().ok_or("引导缺少项目身份")?)?;
    tx.execute("INSERT INTO steering_inputs VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(input_id) DO UPDATE SET status=excluded.status,payload=excluded.payload",params![input["inputId"].as_str(),input["modId"].as_str(),input["sessionId"].as_str(),input["taskId"].as_str(),input["generation"].as_i64(),input["submittedAt"].as_str(),input["status"].as_str(),encode(input)?]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e|e.to_string())
}
#[tauri::command]
pub fn list_steering_inputs(app: tauri::AppHandle, mod_id: String, session_id: String) -> Result<Vec<serde_json::Value>,String> {
    read(&app,&mod_id)?.sessions.iter().find(|s|s.session_id==session_id).ok_or("会话不属于项目")?;
    let db = database::connect(&app)?;
    let mut query = db.prepare("SELECT payload FROM steering_inputs WHERE mod_id=?1 AND session_id=?2 ORDER BY rowid").map_err(|e|e.to_string())?;
    let rows = query.query_map(params![mod_id,session_id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;
    rows.map(|r|decode(r.map_err(|e|e.to_string())?)).collect()
}

/// 文件操作日志与业务提交分开持久，部署复制不能被数据库回滚冒充已回退。
pub fn file_operation(app: &tauri::AppHandle, id: &str, mod_id: &str, kind: &str, status: &str, payload: &serde_json::Value) -> Result<(),String> {
    database::connect(app)?.execute("INSERT INTO file_operations VALUES(?1,?2,?3,?4,?5) ON CONFLICT(operation_id) DO UPDATE SET status=excluded.status,payload=excluded.payload",params![id,mod_id,kind,status,encode(payload)?]).map_err(|e|e.to_string())?;
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
            let mut record: super::models::InstallationRecord = serde_json::from_value(payload["record"].clone()).map_err(|e|e.to_string())?;
            update(app,&mod_id,|project| {
                if !project.installations.iter().any(|r|r.installation_id==record.installation_id) {
                    record.status=super::models::InstallationStatus::Failed; record.rollback=super::models::RollbackStatus::NotAttempted;
                    record.error=Some("上次安装记录未提交，目标文件可能已变化；未自动回退".into());
                    project.installed_targets.retain(|t|t.target_directory!=record.target_directory);
                    project.installed_targets.push(super::models::InstalledTarget{target_directory:record.target_directory.clone(),version_id:None,state:super::models::TargetState::Uncertain});
                    project.installations.push(record.clone());
                }
                Ok(())
            })?;
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
        } else if matches!(kind.as_str(),"asset_import"|"asset_process") {
            crate::assets::reconcile(app,&mod_id,&id,&kind,&payload)?;
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
#[tauri::command]
pub fn list_recovery_issues(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>,String> {
    let db=database::connect(&app)?;
    let mut query=db.prepare("SELECT mod_id,kind,payload FROM file_operations WHERE status='requires_review'").map_err(|e|e.to_string())?;
    let rows=query.query_map([],|r|Ok(serde_json::json!({"modId":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"detail":r.get::<_,String>(2)?}))).map_err(|e|e.to_string())?;
    rows.map(|r|r.map_err(|e|e.to_string())).collect()
}

pub fn has_active_tasks(app: &tauri::AppHandle, mod_id: &str) -> Result<bool,String> {
    database::connect(app)?.query_row("SELECT EXISTS(SELECT 1 FROM tasks WHERE mod_id=?1 AND status IN ('preparing','running','stopping'))",[mod_id],|r|r.get(0)).map_err(|e|e.to_string())
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
