use crate::storage::{models::ModProject,repository::read_on};
use rusqlite::{Connection,OptionalExtension,params};

pub fn has_unresolved_removal_on(db: &Connection, id: &str) -> Result<bool,String> {
    db.query_row("SELECT EXISTS(SELECT 1 FROM file_operations WHERE mod_id=?1 AND kind='project_delete' AND status IN ('pending','requires_review'))",[id],|r|r.get(0)).map_err(|e|e.to_string())
}
pub fn ensure_writable_on(db: &Connection,id: &str) -> Result<(),String> {
    let archived: Option<bool>=db.query_row("SELECT archived FROM projects WHERE mod_id=?1 AND NOT EXISTS(SELECT 1 FROM project_removals WHERE mod_id=?1)",[id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    match archived { None=>return Err("项目不存在或已删除".into()),Some(true)=>return Err("项目已归档，请先恢复后再修改".into()),_=>{} }
    if has_unresolved_removal_on(db,id)? { return Err("项目移除恢复尚未完成，暂不能修改".into()); }
    Ok(())
}
pub fn ensure_writable(app: &tauri::AppHandle,id: &str) -> Result<(),String> {
    ensure_writable_on(&crate::storage::database::connect(app)?,id)
}
pub fn ensure_idle_on(db: &Connection,id: &str) -> Result<(),String> {
    if db.query_row::<bool,_,_>("SELECT EXISTS(SELECT 1 FROM tasks WHERE mod_id=?1 AND status IN ('preparing','running','stopping'))",[id],|r|r.get(0)).map_err(|e|e.to_string())? {return Err("项目有活动任务，请结束后重试".into());}
    Ok(())
}
pub fn set_archived_on(db: &mut Connection,id: &str,expected:i64,archived:bool) -> Result<ModProject,String> {
    let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    if has_unresolved_removal_on(&tx,id)? { return Err("项目移除恢复尚未完成".into()); }
    ensure_idle_on(&tx,id)?;
    if tx.execute("UPDATE projects SET archived=?3,revision=revision+1 WHERE mod_id=?1 AND revision=?2 AND NOT EXISTS(SELECT 1 FROM project_removals WHERE mod_id=?1)",params![id,expected,archived]).map_err(|e|e.to_string())?!=1 {return Err("项目已变化，请重新加载后操作".into());}
    let project=read_on(&tx,id)?;
    tx.commit().map_err(|e|e.to_string())?;
    Ok(project)
}

pub fn delete_private_rows_on(db: &mut Connection,id: &str,operation: &str,removed_at: &str) -> Result<(),String> {
    let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    // 跨项目 supersedes 属于损坏数据，不能通过删除其他项目来补偿。
    let cross:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM verification_records a JOIN verification_records b ON a.supersedes=b.record_id WHERE a.mod_id<>b.mod_id AND (a.mod_id=?1 OR b.mod_id=?1))",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if cross {return Err("验收记录存在跨项目引用，未删除任何数据".into());}
    for table in ["task_memory_uses","verification_records","feedback_drafts","delivery_metadata","context_snapshots","context_proposals","project_contexts","inspection_reports","project_assets","runtime_logs","steering_inputs","session_stats","tasks","sessions","legacy_imports"] {
        tx.execute(&format!("DELETE FROM {table} WHERE mod_id=?1"),[id]).map_err(|e|e.to_string())?;
    }
    tx.execute("DELETE FROM file_operations WHERE mod_id=?1 AND operation_id<>?2",params![id,operation]).map_err(|e|e.to_string())?;
    if tx.execute("DELETE FROM projects WHERE mod_id=?1",[id]).map_err(|e|e.to_string())?!=1 {return Err("项目已不存在".into());}
    tx.execute("INSERT INTO project_removals VALUES(?1,?2,?3)",params![id,operation,removed_at]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e|e.to_string())
}
