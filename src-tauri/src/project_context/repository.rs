use super::models::*;
use crate::storage::repository::{decode, encode};
use rusqlite::{params, Connection, OptionalExtension};
pub fn initialize_on(db:&Connection,id:&str)->Result<(),String>{
    let value=ProjectContext::empty(id);
    db.execute("INSERT INTO project_contexts(mod_id,revision,payload) SELECT ?1,0,?2 WHERE EXISTS(SELECT 1 FROM projects WHERE mod_id=?1) ON CONFLICT(mod_id) DO NOTHING",params![id,encode(&value)?]).map_err(|e|e.to_string())?;
    Ok(())
}
pub fn backfill(app:&tauri::AppHandle)->Result<(),String>{
    let db=crate::storage::database::connect(app)?;
    let mut query=db.prepare("SELECT mod_id FROM projects").map_err(|e|e.to_string())?;
    for id in query.query_map([],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())? {initialize_on(&db,&id.map_err(|e|e.to_string())?)?;}
    Ok(())
}
pub fn read_on(db: &Connection, id: &str) -> Result<ProjectContext, String> {
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE mod_id=?1)",
            [id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !exists {
        return Err("项目不存在".into());
    }
    let value: Option<String> = db
        .query_row(
            "SELECT payload FROM project_contexts WHERE mod_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    value
        .map(decode)
        .transpose()
        .map(|v| v.unwrap_or_else(|| ProjectContext::empty(id)))
}
pub fn save_on(
    db: &Connection,
    value: &ProjectContext,
    expected: i64,
) -> Result<ProjectContext, String> {
    if expected < 0 || value.revision != expected {
        return Err("环境 revision 无效".into());
    }
    let mut saved = value.clone();
    saved.revision = expected.checked_add(1).ok_or("环境 revision 溢出")?;
    let payload = encode(&saved)?;
    let count=if expected==0 {
        db.execute("INSERT INTO project_contexts(mod_id,revision,payload) VALUES(?1,1,?2) ON CONFLICT(mod_id) DO UPDATE SET revision=1,payload=excluded.payload WHERE project_contexts.revision=0",params![value.mod_id,payload])
    }else{
        db.execute("UPDATE project_contexts SET revision=?3,payload=?4 WHERE mod_id=?1 AND revision=?2",params![value.mod_id,expected,saved.revision,payload])
    }.map_err(|e|e.to_string())?;
    if count != 1 {
        return Err("项目环境已变化，请重新加载；输入尚未保存".into());
    }
    Ok(saved)
}

pub fn update_automatic_on(db:&mut Connection,task:&crate::pi::tasks::TaskContext,input:&ProjectMemoryUpdateInput,cancelled:&std::sync::atomic::AtomicBool)->Result<ProjectMemoryUpdateResult,String>{
    if input.changes.is_empty() || input.changes.len()>3{return Err("项目记忆需包含 1–3 项变更".into());}
    let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    crate::knowledge::repository::ensure_running_on(&tx,task,cancelled)?;
    let mut current=read_on(&tx,&task.mod_id)?;
    if current.revision!=input.expected_revision{return Err("项目记忆已变化，请重新读取；旧快照保持不变".into());}
    let before=current.clone();
    let changes=input.changes.iter().map(|change|change.change()).collect::<Result<Vec<_>,_>>()?;
    apply_changes(&mut current,&changes)?;
    for change in &input.changes {
        if change.source_refs.is_empty() || change.source_refs.len()>20 {return Err("项目记忆缺少有效来源".into());}
        current.field_sources.insert(change.field.clone(),change.source_refs.clone());
    }
    // observedAt 是写入时间，不参与同内容去重。
    if let (Some(old),Some(new))=(&before.game_version,&mut current.game_version) {
        if old.value==new.value {new.observed_at=old.observed_at.clone();new.source=old.source.clone();new.source_id=old.source_id.clone();}
    }
    super::validate(&current)?;
    let mut changed_fields=Vec::new();
    for change in &input.changes {
        let changed=match change.field.as_str(){"gameVersion"=>current.game_version!=before.game_version,"prerequisites"=>current.prerequisites!=before.prerequisites,"confirmedRequirements"=>current.confirmed_requirements!=before.confirmed_requirements,_=>return Err("模型不能修改此项目记忆字段".into())};
        if changed || current.field_sources.get(&change.field)!=before.field_sources.get(&change.field){changed_fields.push(change.field.clone());}
    }
    if changed_fields.is_empty(){return Ok(ProjectMemoryUpdateResult{outcome:"unchanged",mod_id:task.mod_id.clone(),revision:current.revision,changed_fields});}
    current=save_on(&tx,&current,input.expected_revision)?;
    crate::knowledge::repository::ensure_running_on(&tx,task,cancelled)?;
    tx.commit().map_err(|e|e.to_string())?;
    Ok(ProjectMemoryUpdateResult{outcome:"saved",mod_id:task.mod_id.clone(),revision:current.revision,changed_fields})
}
pub fn snapshot_on(
    db: &Connection,
    id: &str,
    task: &str,
    snapshot: &str,
    created: &str,
) -> Result<ContextSnapshot, String> {
    let value = ContextSnapshot {
        snapshot_id: snapshot.into(),
        mod_id: id.into(),
        task_id: task.into(),
        created_at: created.into(),
        context: read_on(db, id)?,
    };
    db.execute("INSERT INTO context_snapshots(snapshot_id,mod_id,task_id,payload) VALUES(?1,?2,?3,?4) ON CONFLICT(task_id) DO NOTHING",params![snapshot,id,task,encode(&value)?]).map_err(|e|e.to_string())?;
    snapshot_for_task_on(db, id, task)
}
pub fn snapshot_for_task_on(
    db: &Connection,
    id: &str,
    task: &str,
) -> Result<ContextSnapshot, String> {
    let raw: String = db
        .query_row(
            "SELECT payload FROM context_snapshots WHERE mod_id=?1 AND task_id=?2",
            params![id, task],
            |r| r.get(0),
        )
        .map_err(|_| "任务环境快照不存在或不属于项目".to_string())?;
    decode(raw)
}
fn apply_changes(value: &mut ProjectContext, changes: &[ContextChange]) -> Result<(), String> {
    let mut fields = std::collections::BTreeSet::new();
    for change in changes {
        let (field, reason) = match change {
            ContextChange::GameVersion {
                reason,
                value: version,
            } => {
                value.game_version = Some(ContextField {
                    value: version.clone(),
                    source: "user".into(),
                    source_id: None,
                    observed_at: crate::storage::mods::now()?,
                });
                ("gameVersion", reason)
            }
            ContextChange::Prerequisites {
                reason,
                value: items,
            } => {
                value.prerequisites = items.clone();
                ("prerequisites", reason)
            }
            ContextChange::ConfirmedRequirements {
                reason,
                value: items,
            } => {
                value.confirmed_requirements = items.clone();
                ("requirements", reason)
            }
        };
        if !fields.insert(field) || reason.trim().is_empty() || reason.len() > 4000 {
            return Err("建议字段重复或理由无效".into());
        }
    }
    Ok(())
}
