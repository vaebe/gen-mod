use super::{database, models::ModProject, paths};
use rusqlite::{Connection, OptionalExtension};

pub fn encode<T: serde::Serialize>(value: &T) -> Result<String, String> { serde_json::to_string(value).map_err(|e| e.to_string()) }
pub fn decode<T: serde::de::DeserializeOwned>(value: String) -> Result<T, String> { serde_json::from_str(&value).map_err(|e| e.to_string()) }

/// 从 projects 与未归档 sessions 组合既有 IPC DTO。
pub fn read_on(db: &Connection, id: &str) -> Result<ModProject, String> {
    let (payload, revision, archived): (String, i64, bool) = db.query_row("SELECT payload,revision,archived FROM projects WHERE mod_id=?1", [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e| e.to_string())?.ok_or("项目不存在")?;
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

pub fn list(app: &tauri::AppHandle) -> Result<Vec<ModProject>, String> {
    let db = database::connect(app)?;
    let mut query = db.prepare("SELECT mod_id FROM projects WHERE archived=0 ORDER BY json_extract(payload,'$.updatedAt') DESC").map_err(|e| e.to_string())?;
    let ids = query.query_map([], |r| r.get::<_,String>(0)).map_err(|e| e.to_string())?;
    ids.map(|id| read_on(&db,&id.map_err(|e| e.to_string())?)).collect()
}
