use super::{database, models::{ModProject, SessionReference}, paths, repository};
use rusqlite::{params, OptionalExtension};
use std::{fs, io::{BufRead, BufReader}, path::Path};

fn json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    paths::reject_redirect(path)?;
    repository::decode(String::from_utf8(fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?)
}
/// 所有验证先完成，再按项目一次提交；失败不导入任何活动元数据。
fn project(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let directory = paths::project_dir(app,id)?;
    paths::reject_redirect(&directory)?;
    let mut project: ModProject = json(&directory.join("mod.json"))?;
    if project.mod_id != id { return Err("项目 ID 与目录不一致".into()); }
    for version in &project.versions { paths::validate_id(&version.version_id)?; paths::validate_id(&version.task_id)?; }
    for installation in &project.installations { paths::validate_id(&installation.installation_id)?; paths::validate_id(&installation.version_id)?; }
    if let Some(version) = &project.latest_version_id { if !project.versions.iter().any(|v|&v.version_id==version) { return Err("最新版本引用不存在".into()); } }
    let sessions_root = directory.join("logs/sessions");
    let mut session_ids=std::collections::BTreeSet::new();
    for session in &project.sessions {
        if !session_ids.insert(session.session_id.clone()) { return Err("重复会话 ID".into()); }
        paths::validate_id(&session.session_id)?;
        if !session.session_file.is_empty() {
            let file = Path::new(&session.session_file);
            if !file.exists() { return Err(format!("历史文件缺失：{}",file.display())); }
            paths::reject_redirect(file)?;
            paths::reject_redirect(&sessions_root)?;
            if file.canonicalize().map_err(|e| e.to_string())?.parent() != Some(sessions_root.canonicalize().map_err(|e| e.to_string())?.as_path()) { return Err("历史文件越界".into()); }
            let header = BufReader::new(fs::File::open(file).map_err(|e| e.to_string())?).lines().next().ok_or("历史文件为空")?.map_err(|e| e.to_string())?;
            let header: serde_json::Value = repository::decode(header)?;
            if header["id"] != session.session_id { return Err("历史文件 ID 不匹配".into()); }
        }
    }
    let mut tasks = vec![];
    let logs = directory.join("logs");
    if logs.exists() {
        paths::reject_redirect(&logs)?;
        for entry in fs::read_dir(&logs).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let task_id = entry.file_name().to_string_lossy().into_owned();
            if paths::validate_id(&task_id).is_err() { continue; }
            paths::reject_redirect(&entry.path())?;
            let task: crate::pi::tasks::TaskRecord = json(&entry.path().join("task.json"))?;
            if task.context.task_id != task_id || task.context.mod_id != id { return Err("任务归属不一致".into()); }
            paths::validate_id(&task.context.session_id)?;
            paths::validate_id(&task.context.staging_id)?;
            let expected_source=directory.join("source").canonicalize().map_err(|e|e.to_string())?;
            if Path::new(&task.context.source_directory).canonicalize().map_err(|e|e.to_string())? != expected_source { return Err("任务源码路径归属不一致".into()); }
            let expected=directory.join("logs").join(&task_id).join("staging").join(&task.context.staging_id);
            if Path::new(&task.context.staging_directory) != expected { return Err("任务 staging 路径归属不一致".into()); }
            if !project.sessions.iter().any(|s| s.session_id == task.context.session_id) {
                // 旧的已删除会话仍可被历史任务引用；导入后归档行不进入列表。
                project.sessions.push(SessionReference { session_id: task.context.session_id.clone(), session_file: String::new(), skills: vec![], read_only: true, name: String::new() });
            }
            tasks.push(task);
        }
    }
    let mut runtime = vec![];
    if sessions_root.exists() {
        paths::reject_redirect(&sessions_root)?;
        for entry in fs::read_dir(&sessions_root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(session_id) = name.strip_suffix(".runtime.jsonl") else { continue; };
            paths::validate_id(session_id)?;
            paths::reject_redirect(&entry.path())?;
            for line in BufReader::new(fs::File::open(entry.path()).map_err(|e| e.to_string())?).lines() {
                let line = line.map_err(|e| e.to_string())?;
                if line.trim().is_empty() { continue; }
                let log: crate::pi::logs::RuntimeLogEntry = repository::decode(line)?;
                runtime.push((session_id.to_owned(), log));
            }
        }
    }
    let old: ModProject = json(&directory.join("mod.json"))?;
    let mut db = database::connect(app)?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    repository::save_on(&tx,&project,true)?;
    for session in project.sessions.iter().filter(|s| !old.sessions.iter().any(|o| o.session_id == s.session_id)) {
        tx.execute("UPDATE sessions SET deleted=1 WHERE mod_id=?1 AND session_id=?2",params![id,session.session_id]).map_err(|e| e.to_string())?;
    }
    if old.sessions.is_empty() {
        tx.execute("UPDATE projects SET payload=json_set(payload,'$.draftRequirement',?2) WHERE mod_id=?1",params![id,old.draft_requirement]).map_err(|e|e.to_string())?;
    }
    if let Some(session) = old.sessions.last() {
        tx.execute("UPDATE sessions SET draft=?3 WHERE mod_id=?1 AND session_id=?2",params![id,session.session_id,old.draft_requirement]).map_err(|e| e.to_string())?;
    }
    for task in tasks { repository::task_on(&tx,&task)?; }
    for (session_id,log) in runtime { repository::log_on(&tx,id,&session_id,&log)?; }
    tx.execute("INSERT INTO legacy_imports(mod_id,imported_at) VALUES(?1,?2) ON CONFLICT(mod_id) DO UPDATE SET error=NULL,imported_at=excluded.imported_at",params![id,super::mods::now()?]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}
pub fn import(app: &tauri::AppHandle) -> Result<(), String> {
    let root = paths::mods_root(app)?;
    if !root.exists() { return Ok(()); }
    paths::reject_redirect(&root)?;
    let db = database::connect(app)?;
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let id = entry.file_name().to_string_lossy().into_owned();
        if paths::validate_id(&id).is_err() || !entry.file_type().map_err(|e| e.to_string())?.is_dir() { continue; }
        if db.query_row::<bool,_,_>("SELECT EXISTS(SELECT 1 FROM project_removals WHERE mod_id=?1)",[&id],|r|r.get(0)).map_err(|e|e.to_string())? {continue;}
        let done: Option<String> = db.query_row("SELECT mod_id FROM projects WHERE mod_id=?1",[&id],|r| r.get(0)).optional().map_err(|e| e.to_string())?;
        if done.is_some() { continue; }
        if let Err(error) = project(app,&id) {
            db.execute("INSERT INTO legacy_imports(mod_id,error) VALUES(?1,?2) ON CONFLICT(mod_id) DO UPDATE SET error=excluded.error",params![id,error]).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
