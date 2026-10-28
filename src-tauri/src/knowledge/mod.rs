pub mod models;
pub mod repository;
pub mod search;
pub mod evidence;
pub mod usage;
#[cfg(test)]
mod tests;
use crate::storage::{database, repository as storage};
use models::*;
pub fn references_for_task(app: &tauri::AppHandle, task: &crate::pi::tasks::TaskContext) -> Result<Vec<KnowledgeReference>, String> {
    let mut refs = vec![];
    for view in crate::skills::registry::task_views(app, task)? {
        let root = std::path::Path::new(&view.definition.path);
        let dir = root.join("references");
        if !dir.is_dir() {
            continue;
        }
        crate::storage::paths::reject_redirect(&dir)?;
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().is_none_or(|v| v != "md") {
                continue;
            }
            crate::storage::paths::reject_redirect(&path)?;
            let hash = crate::changes::file_hash(&path)?;
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let relative = format!("references/{}", entry.file_name().to_string_lossy());
            refs.push(KnowledgeReference {
                id: format!("{}:{}:{}", view.definition.name, relative, hash),
                path: path.to_string_lossy().into_owned(),
                sha256: hash,
                title: text
                    .lines()
                    .next()
                    .unwrap_or(&relative)
                    .trim_start_matches('#')
                    .trim()
                    .into(),
            });
        }
    }
    Ok(refs)
}
#[tauri::command]
pub fn get_long_term_memory(app: tauri::AppHandle, note_id: String) -> Result<KnowledgeNote, String> {
    let note = repository::read_on(&database::connect(&app)?, &note_id)?;
    if note.status == "deleted" { return Err("记忆已删除".into()); }
    Ok(note)
}

#[tauri::command]
pub fn list_long_term_memories(app: tauri::AppHandle, query: Option<MemoryListQuery>, status: Option<String>, page: usize, page_size: usize) -> Result<MemoryPage, String> {
    if page == 0 || !(1..=50).contains(&page_size) { return Err("页码从 1 开始，每页 1–50 条".into()); }
    if status.as_ref().is_some_and(|v| !matches!(v.as_str(), "active" | "candidate" | "archived")) { return Err("记忆状态筛选无效".into()); }
    let offset = (page - 1).checked_mul(page_size).and_then(|value| i64::try_from(value).ok()).ok_or("页码无效")?;
    let db = database::connect(&app)?;
    let query = query.unwrap_or_default();
    let predicate = "WHERE status<>'deleted' AND (?1 IS NULL OR game_id=?1) AND (?2 IS NULL OR status=?2) AND (?3='' OR instr(lower(json_extract(payload,'$.title')||' '||json_extract(payload,'$.content')),lower(?3))>0)";
    let values = rusqlite::params![query.game_id,status,query.text];
    let total = db.query_row(&format!("SELECT COUNT(*) FROM knowledge_notes {predicate}"), values, |row| row.get(0)).map_err(|e| e.to_string())?;
    let mut statement = db.prepare(&format!("{} {predicate} ORDER BY updated_at DESC,note_id LIMIT ?4 OFFSET ?5", repository::NOTE_SELECT)).map_err(|e| e.to_string())?;
    let rows = statement.query_map(rusqlite::params![query.game_id,status,query.text,page_size as i64,offset], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
    let notes = rows.map(|row| storage::decode(row.map_err(|e| e.to_string())?)).collect::<Result<Vec<_>,_>>()?;
    Ok(MemoryPage { notes, total })
}

#[tauri::command]
pub fn save_long_term_memory_entry(app: tauri::AppHandle, note_id: Option<String>, expected_revision: Option<i64>, input: MemoryEntryInput) -> Result<KnowledgeNote, String> {
    let mut db = database::connect(&app)?;
    if let Some(id) = note_id {
        return repository::edit_entry_on(&mut db, &id, expected_revision.ok_or("修改记忆需要 revision")?, &input);
    }
    if expected_revision.is_some() { return Err("新记忆不能指定旧 revision".into()); }
    repository::validate_entry(&input)?;
    let note_id = uuid::Uuid::new_v4().to_string();
    let now = crate::storage::mods::now()?;
    let note = KnowledgeNote { note_id: note_id.clone(), revision: 1, game_id: input.game_id, status: "active".into(), allow_reuse: true,
        origin_mod_id: None, origin_project_name: None, user_edited: true, memory_key: format!("manual:{note_id}"), created_at: Some(now.clone()), updated_at: Some(now),
        input: KnowledgeInput { title: input.title, content: input.content, version_scope: input.version_scope, game_version: input.game_version,
            route: "manual".into(), evidence_kind: "manual".into(), source_refs: vec![SourceRef { kind: "manual".into(), id: note_id, ..Default::default() }] } };
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    repository::insert_global_on(&tx, &note)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(note)
}

#[tauri::command]
pub fn set_long_term_memory_status(app: tauri::AppHandle, note_id: String, expected_revision: i64, status: String) -> Result<(), String> {
    repository::set_status_on(&mut database::connect(&app)?, &note_id, expected_revision, &status)
}

pub fn save_for_task(app: &tauri::AppHandle, task: &crate::pi::tasks::TaskContext, input: SaveLongTermMemoryInput,
    cancelled: &std::sync::atomic::AtomicBool) -> Result<MemorySaveResult, String> {
    let project = storage::read(app, &task.mod_id)?;
    evidence::validate_sources(app, task, &input)?;
    repository::save_automatic_on(&mut database::connect(app)?, task, &project.game_id, &project.name, &input, cancelled)
}
