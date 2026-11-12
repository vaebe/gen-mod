use super::models::{SaveLongTermMemoryInput, SourceRef};
use crate::{pi::tasks::TaskContext, storage::{database, repository as storage}};
use rusqlite::{params, Connection};

pub(crate) fn validate_message(entries: &[serde_json::Value], task: &TaskContext, source: &SourceRef) -> Result<(), String> {
    if source.session_id.as_deref() != Some(&task.session_id) || source.task_id.as_ref().is_some_and(|id| id != &task.task_id) {
        return Err("消息来源必须属于当前会话".into());
    }
    let entry = entries.iter().find(|entry| entry["id"] == source.id && entry["message"]["role"] == "user").ok_or("原用户消息不存在")?;
    if source.sha256.as_deref() != Some(&crate::verification::evidence::digest(&entry["message"])?) {
        return Err("用户消息指纹不一致".into());
    }
    Ok(())
}

pub(crate) fn validate_command_on(db: &Connection, task: &TaskContext, source: &SourceRef) -> Result<Option<String>, String> {
    if source.session_id.is_some() || source.sha256.is_some() { return Err("命令来源包含不适用的字段".into()); }
    let task_id = source.task_id.as_deref().ok_or("命令来源缺少 taskId")?;
    let raw: String = db.query_row("SELECT payload FROM tasks WHERE task_id=?1 AND mod_id=?2", params![task_id,task.mod_id], |row| row.get(0))
        .map_err(|_| "命令任务不属于当前项目")?;
    let record: crate::pi::tasks::TaskRecord = storage::decode(raw)?;
    if record.context.mod_id != task.mod_id || record.context.task_id != task_id { return Err("命令任务身份不一致".into()); }
    if !record.commands.iter().any(|receipt| receipt.command_id == source.id && receipt.task_id == task_id
        && matches!(receipt.status.as_str(), "succeeded" | "failed" | "cancelled")) { return Err("没有对应的真实命令回执".into()); }
    Ok(crate::project_context::repository::snapshot_for_task_on(db, &task.mod_id, task_id)?.context.game_version.map(|value| value.value))
}

pub fn validate_sources(app: &tauri::AppHandle, task: &TaskContext, input: &SaveLongTermMemoryInput) -> Result<(), String> {
    if input.source_refs.is_empty() || input.source_refs.len() > 20 { return Err("记忆需要 1–20 项真实来源".into()); }
    storage::read(app, &task.mod_id)?;
    let db = database::connect(app)?;
    let mut unique = std::collections::BTreeSet::new();
    let mut versions = Vec::new();
    for source in &input.source_refs {
        if !unique.insert((&source.kind,&source.id,&source.task_id,&source.session_id)) { return Err("记忆来源重复".into()); }
        if (input.evidence_kind == "reference" && source.kind != "reference")
            || (input.evidence_kind == "user_observation" && !matches!(source.kind.as_str(), "verification" | "user_message")) {
            return Err("记忆证据类别与来源不一致".into());
        }
        let version = match source.kind.as_str() {
            "reference" => {
                if source.task_id.is_some() || source.session_id.is_some() { return Err("参考来源包含不适用的任务字段".into()); }
                let references = super::references_for_task(app, task)?;
                let reference = references.iter().find(|reference| reference.id == source.id).ok_or("参考资料已变化或不属于当前游戏")?;
                if source.sha256.as_ref().is_some_and(|hash| hash != &reference.sha256) { return Err("参考资料指纹不一致".into()); }
                crate::project_context::repository::snapshot_for_task_on(&db, &task.mod_id, &task.task_id)?.context.game_version.map(|value| value.value)
            },
            "verification" => {
                if source.task_id.is_some() || source.session_id.is_some() || source.sha256.is_some() { return Err("验收来源包含不适用的字段".into()); }
                let raw: String = db.query_row("SELECT payload FROM verification_records WHERE mod_id=?1 AND record_id=?2", params![task.mod_id,source.id], |row| row.get(0))
                    .map_err(|_| "来源不是当前项目已确认的验收记录")?;
                let record: crate::verification::models::VerificationRecord = storage::decode(raw)?;
                if record.mod_id != task.mod_id || record.record_id != source.id { return Err("验收记录身份不一致".into()); }
                record.payload.game_version
            },
            "command" => validate_command_on(&db, task, source)?,
            "user_message" => {
                validate_message(&crate::verification::evidence::user_messages(app, &task.mod_id, &task.session_id)?, task, source)?;
                // 用户消息的真实性不等于已获得可核对的结构化游戏版本。
                None
            },
            _ => return Err("模型不能使用手动或未知来源".into()),
        };
        versions.push(version);
    }
    if input.version_scope == "exact" && versions.iter().any(|version| version.is_none() || version.as_ref() != input.game_version.as_ref()) {
        return Err("精确版本缺少一致的来源依据；未知版本请保存为 unknown".into());
    }
    Ok(())
}
