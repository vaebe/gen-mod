use super::models::*;
use crate::storage::repository::{decode, encode};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

// 列是状态与归属的唯一来源，旧 payload 只能提供正文、范围与来源。
pub const NOTE_SELECT: &str = "SELECT json_set(payload,'$.noteId',note_id,'$.originModId',origin_mod_id,'$.gameId',game_id,'$.revision',revision,'$.status',status,'$.allowReuse',json(CASE WHEN allow_reuse<>0 THEN 'true' ELSE 'false' END),'$.userEdited',json(CASE WHEN user_edited<>0 THEN 'true' ELSE 'false' END),'$.memoryKey',memory_key,'$.createdAt',created_at,'$.updatedAt',updated_at) FROM knowledge_notes";

pub fn content_hash(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.replace("\r\n", "\n").trim().as_bytes()))
}

pub fn remember_content_on(db: &Connection, game: &str, id: &str, content: &str) -> Result<(), String> {
    if !content.trim().is_empty() {
        db.execute("INSERT OR IGNORE INTO memory_content_links(game_id,content_sha256,note_id) VALUES(?1,?2,?3)",
            params![game,content_hash(content),id]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn backfill_content_links_on(db: &Connection) -> Result<(), String> {
    let mut query = db.prepare(&format!("{NOTE_SELECT} ORDER BY note_id")).map_err(|e| e.to_string())?;
    let rows = query.query_map([], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
    for row in rows {
        let note: KnowledgeNote = decode(row.map_err(|e| e.to_string())?)?;
        remember_content_on(db, &note.game_id, &note.note_id, &note.input.content)?;
    }
    Ok(())
}

/// 由迁移事务调用，正文指纹失败时必须连同表切换一并回滚。
pub fn migrate_memory_on(db: &Connection) -> Result<(), String> {
    db.execute_batch(include_str!("../storage/migrations/009_memory.sql")).map_err(|e| e.to_string())?;
    backfill_content_links_on(db)
}

pub fn read_on(db: &Connection, id: &str) -> Result<KnowledgeNote, String> {
    let raw: Option<String> = db.query_row(&format!("{NOTE_SELECT} WHERE note_id=?1"), [id], |row| row.get(0))
        .optional().map_err(|e| e.to_string())?;
    decode(raw.ok_or("记忆不存在")?)
}

pub fn insert_global_on(db: &Connection, note: &KnowledgeNote) -> Result<(), String> {
    db.execute("INSERT INTO knowledge_notes(note_id,origin_mod_id,game_id,revision,status,allow_reuse,user_edited,memory_key,payload,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![note.note_id,note.origin_mod_id,note.game_id,note.revision,note.status,note.allow_reuse,note.user_edited,note.memory_key,encode(note)?,note.created_at,note.updated_at])
        .map_err(|e| e.to_string())?;
    remember_content_on(db, &note.game_id, &note.note_id, &note.input.content)
}

#[cfg(test)]
pub fn insert_on(db: &Connection, mod_id: &str, note: &KnowledgeNote) -> Result<(), String> {
    let mut note = note.clone();
    note.origin_mod_id = Some(mod_id.into());
    if note.memory_key.is_empty() { note.memory_key = format!("legacy:{}", note.note_id); }
    insert_global_on(db, &note)
}

pub fn write_on(db: &Connection, note: &KnowledgeNote, expected: i64) -> Result<(), String> {
    if db.execute("UPDATE knowledge_notes SET origin_mod_id=?2,game_id=?3,revision=?4,status=?5,allow_reuse=?6,user_edited=?7,payload=?8,updated_at=?9 WHERE note_id=?1 AND revision=?10 AND status<>'deleted'",
        params![note.note_id,note.origin_mod_id,note.game_id,note.revision,note.status,note.allow_reuse,note.user_edited,encode(note)?,note.updated_at,expected]).map_err(|e| e.to_string())? != 1 {
        return Err("记忆已变化或删除，请重新读取后操作".into());
    }
    remember_content_on(db, &note.game_id, &note.note_id, &note.input.content)
}

pub fn validate_entry(input: &MemoryEntryInput) -> Result<(), String> {
    if input.title.trim().is_empty() || input.title.chars().count() > 120 || input.content.trim().is_empty() || input.content.chars().count() > 2000 {
        return Err("记忆标题应为 1–120 字符，正文应为 1–2000 字符".into());
    }
    if input.game_id.trim().is_empty() || input.game_id.len() > 120 { return Err("适用游戏无效".into()); }
    match input.version_scope.as_str() {
        "exact" if input.game_id != "*" && input.game_version.as_ref().is_some_and(|v| !v.trim().is_empty() && v.chars().count() <= 120) => {},
        "independent" | "unknown" if input.game_version.is_none() && (input.game_id != "*" || input.version_scope == "independent") => {},
        _ => return Err("记忆适用版本范围无效".into()),
    }
    Ok(())
}

pub fn edit_entry_on(db: &mut Connection, id: &str, expected: i64, input: &MemoryEntryInput) -> Result<KnowledgeNote, String> {
    validate_entry(input)?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let mut note = read_on(&tx, id)?;
    if note.status == "deleted" || note.revision != expected { return Err("记忆已变化或删除，请重新读取后操作".into()); }
    remember_content_on(&tx, &note.game_id, id, &note.input.content)?;
    note.input.title = input.title.clone();
    note.input.content = input.content.clone();
    note.game_id = input.game_id.clone();
    note.input.version_scope = input.version_scope.clone();
    note.input.game_version = input.game_version.clone();
    note.input.evidence_kind = "manual".into();
    note.user_edited = true;
    note.revision += 1;
    note.updated_at = Some(crate::storage::mods::now()?);
    write_on(&tx, &note, expected)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(note)
}

#[cfg(test)]
pub fn edit_on(db: &mut Connection, id: &str, expected: i64, title: &str, content: &str) -> Result<KnowledgeNote, String> {
    let old = read_on(db, id)?;
    edit_entry_on(db, id, expected, &MemoryEntryInput { title: title.into(), content: content.into(), game_id: old.game_id,
        version_scope: old.input.version_scope, game_version: old.input.game_version })
}

pub fn set_status_on(db: &mut Connection, id: &str, expected: i64, status: &str) -> Result<(), String> {
    if !matches!(status, "active" | "archived" | "deleted") { return Err("记忆状态无效".into()); }
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let mut note = read_on(&tx, id)?;
    if note.status == "deleted" || note.revision != expected { return Err("记忆已变化或删除，请重新读取后操作".into()); }
    remember_content_on(&tx, &note.game_id, id, &note.input.content)?;
    note.status = status.into();
    note.user_edited = true;
    note.revision += 1;
    note.updated_at = Some(crate::storage::mods::now()?);
    if status == "active" { note.allow_reuse = true; }
    if status == "deleted" {
        note.input.title.clear(); note.input.content.clear(); note.input.route.clear(); note.input.source_refs.clear();
        note.input.game_version = None; note.input.version_scope = "unknown".into(); note.input.evidence_kind = "manual".into();
        note.origin_mod_id = None; note.origin_project_name = None; note.allow_reuse = false;
    }
    write_on(&tx, &note, expected)?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn ensure_running_on(db: &Connection, task: &crate::pi::tasks::TaskContext, cancelled: &std::sync::atomic::AtomicBool) -> Result<(), String> {
    crate::project_lifecycle::repository::ensure_writable_on(db,&task.mod_id)?;
    if cancelled.load(std::sync::atomic::Ordering::SeqCst) { return Err("任务工具已取消或结束".into()); }
    let generation = i64::try_from(task.generation).map_err(|_| "任务 generation 无效")?;
    let live: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM tasks t JOIN sessions s ON s.mod_id=t.mod_id AND s.session_id=t.session_id JOIN projects p ON p.mod_id=t.mod_id WHERE t.task_id=?1 AND t.mod_id=?2 AND t.session_id=?3 AND t.status='running' AND s.deleted=0 AND s.generation=?4)",
        params![task.task_id,task.mod_id,task.session_id,generation], |row| row.get(0)).map_err(|e| e.to_string())?;
    if !live { return Err("任务已结束、会话已更换或项目不存在".into()); }
    Ok(())
}

pub fn save_automatic_on(db: &mut Connection, task: &crate::pi::tasks::TaskContext, game: &str, project_name: &str,
    input: &SaveLongTermMemoryInput, cancelled: &std::sync::atomic::AtomicBool) -> Result<MemorySaveResult, String> {
    validate_entry(&MemoryEntryInput { title: input.title.clone(), content: input.content.clone(), game_id: game.into(),
        version_scope: input.version_scope.clone(), game_version: input.game_version.clone() })?;
    if input.topic_key.trim().is_empty() || input.topic_key.len() > 200 || input.source_refs.is_empty() || input.source_refs.len() > 20
        || !matches!(input.evidence_kind.as_str(), "reference" | "user_observation" | "development")
        || input.source_refs.iter().any(|source| !matches!(source.kind.as_str(), "reference" | "verification" | "command" | "user_message")) {
        return Err("记忆主题、证据类型或来源无效".into());
    }
    let identity = serde_json::json!([game,input.topic_key.trim().to_lowercase(),input.version_scope,input.game_version]);
    let key = format!("{:x}", Sha256::digest(encode(&identity)?.as_bytes()));
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    ensure_running_on(&tx, task, cancelled)?;
    let mut matches = std::collections::BTreeMap::new();
    if let Some(id) = &input.note_id {
        let note = read_on(&tx, id)?;
        if note.game_id != game && note.game_id != "*" { return Err("不能更新其他游戏的记忆".into()); }
        if input.expected_revision.is_none() { return Err("更新已有记忆必须带 expectedRevision".into()); }
        matches.insert(note.note_id.clone(), note);
    } else if input.expected_revision.is_some() { return Err("expectedRevision 缺少对应 noteId".into()); }
    let mut query = tx.prepare("SELECT note_id FROM knowledge_notes WHERE memory_key=?1 OR note_id IN (SELECT note_id FROM memory_content_links WHERE game_id IN (?2,'*') AND content_sha256=?3)").map_err(|e| e.to_string())?;
    let ids = query.query_map(params![key,game,content_hash(&input.content)], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
    for id in ids {
        let note = read_on(&tx, &id.map_err(|e| e.to_string())?)?;
        matches.insert(note.note_id.clone(), note);
    }
    drop(query);
    let result = |note: &KnowledgeNote, outcome| MemorySaveResult { outcome, note_id: note.note_id.clone(), revision: note.revision,
        title: if note.status == "deleted" { String::new() } else { note.input.title.clone() } };
    if let Some(note) = matches.values().find(|note| note.user_edited || matches!(note.status.as_str(), "deleted" | "archived" | "candidate")) {
        return Ok(result(note, "user_managed"));
    }
    // 正文指纹跨适用版本检查用户保护；普通 AI 条目的不同版本不能被误合并。
    let mut candidates: Vec<_> = matches.into_values().filter(|note| note.memory_key == key || input.note_id.as_ref() == Some(&note.note_id)
        || (note.input.version_scope == input.version_scope && note.input.game_version == input.game_version)).collect();
    if candidates.len() > 1 { return Err("记忆身份存在冲突，请先读取现有条目，不自动合并".into()); }
    let now = crate::storage::mods::now()?;
    let saved = if let Some(mut note) = candidates.pop() {
        if let Some(expected) = input.expected_revision {
            if expected != note.revision { return Err("记忆 revision 已变化，请重新读取".into()); }
        }
        if content_hash(&note.input.content) == content_hash(&input.content) { return Ok(result(&note, "unchanged")); }
        if input.note_id.as_ref() != Some(&note.note_id) || input.expected_revision != Some(note.revision) {
            return Err(format!("记忆已存在（noteId={}，revision={}），更新需要绑定该条目与 revision", note.note_id,note.revision));
        }
        if note.memory_key != key { return Err("记忆主题或版本身份不匹配，不能改写现有条目".into()); }
        let expected = note.revision;
        note.revision += 1;
        note.input.title = input.title.clone(); note.input.content = input.content.clone();
        note.input.source_refs = input.source_refs.clone(); note.input.evidence_kind = input.evidence_kind.clone();
        note.updated_at = Some(now);
        write_on(&tx, &note, expected)?;
        note
    } else {
        let note = KnowledgeNote { note_id: uuid::Uuid::new_v4().to_string(), revision: 1, game_id: game.into(), status: "active".into(), allow_reuse: true,
            origin_mod_id: Some(task.mod_id.clone()), origin_project_name: Some(project_name.into()), user_edited: false,
            memory_key: key, created_at: Some(now.clone()), updated_at: Some(now),
            input: KnowledgeInput { title: input.title.clone(), content: input.content.clone(), version_scope: input.version_scope.clone(), game_version: input.game_version.clone(),
                route: input.topic_key.clone(), evidence_kind: input.evidence_kind.clone(), source_refs: input.source_refs.clone() } };
        insert_global_on(&tx, &note)?;
        note
    };
    ensure_running_on(&tx, task, cancelled)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(result(&saved, "saved"))
}
