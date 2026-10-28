use crate::{pi::tasks::TaskContext, storage::{database, repository as storage}};
use serde_json::{json, Value};

pub(crate) fn record_uses_on(db: &rusqlite::Connection, task: &TaskContext, notes: &[Value]) -> Result<(), String> {
    for note in notes {
        let id = note["note"]["noteId"].as_str().ok_or("记忆使用记录缺少条目 ID")?;
        let revision = note["note"]["revision"].as_i64().ok_or("记忆使用记录缺少 revision")?;
        db.execute("INSERT INTO task_memory_uses(task_id,mod_id,note_id,revision,payload) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(task_id,note_id,revision) DO NOTHING",
            rusqlite::params![task.task_id,task.mod_id,id,revision,storage::encode(note)?]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn search_for_task(app: &tauri::AppHandle, task: &TaskContext, terms: &[String], cancelled: &std::sync::atomic::AtomicBool) -> Result<Value, String> {
    let project = storage::read(app, &task.mod_id)?;
    // 资料文件哈希先读取，避免将磁盘 I/O 放在 SQLite 写事务内。
    let references = super::references_for_task(app, task)?;
    let mut db = database::connect(app)?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    super::repository::ensure_running_on(&tx, task, cancelled)?;
    let version = crate::project_context::repository::snapshot_for_task_on(&tx, &task.mod_id, &task.task_id)?.context.game_version.map(|value| value.value);
    let notes = super::search::search_notes(&tx, &project.game_id, version.as_deref(), terms, 5)?;
    let notes = notes.into_iter().map(|note| {
        let needs_review = note.input.version_scope == "unknown" || note.input.source_refs.iter().any(|source| source.kind == "reference" && !references.iter().any(|reference| reference.id == source.id));
        let removed = if let Some(id) = &note.origin_mod_id {
            !tx.query_row("SELECT EXISTS(SELECT 1 FROM projects WHERE mod_id=?1)", [id], |row| row.get::<_, bool>(0)).map_err(|e| e.to_string())?
        } else { false };
        Ok(json!({"note":note,"needsReview":needs_review,"originRemoved":removed}))
    }).collect::<Result<Vec<Value>, String>>()?;
    record_uses_on(&tx, task, &notes)?;
    super::repository::ensure_running_on(&tx, task, cancelled)?;
    tx.commit().map_err(|e| e.to_string())?;
    let mut selected: Vec<_> = references.into_iter().filter(|reference| terms.is_empty() || terms.iter().any(|term| format!("{} {}",reference.title,reference.path).to_lowercase().contains(&term.to_lowercase()))).collect();
    selected.sort_by(|left,right| left.id.cmp(&right.id));
    selected.truncate(5);
    Ok(json!({"notes":notes,"references":selected}))
}
