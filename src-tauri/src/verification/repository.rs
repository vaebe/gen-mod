use super::models::*;
use crate::storage::repository::{decode, encode};
use rusqlite::{params, Connection, OptionalExtension};
pub fn validate_payload(db: &Connection, id: &str, p: &FeedbackPayload) -> Result<(), String> {
    let metadata = crate::artifacts::delivery_metadata::read_on(db, id, &p.version_id)?;
    if p.observations.trim().is_empty()
        || p.observations.len() > 16000
        || p.source_refs.is_empty()
        || p.source_refs.len() > 30
        || p.case_results.len() > 100
        || p.game_version.as_ref().is_some_and(|v| v.len() > 200)
    {
        return Err("反馈内容为空或超过上限".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for c in &p.case_results {
        if !ids.insert(&c.case_id)
            || !metadata.cases.iter().any(|m| m.id == c.case_id)
            || c.observed.len() > 4000
        {
            return Err("验收条目不存在、重复或过长".into());
        }
    }
    Ok(())
}
pub fn propose_on(
    db: &Connection,
    id: &str,
    payload: FeedbackPayload,
    evidence: Vec<StoredEvidence>,
) -> Result<FeedbackDraft, String> {
    validate_payload(db, id, &payload)?;
    let draft = FeedbackDraft {
        draft_id: uuid::Uuid::new_v4().to_string(),
        mod_id: id.into(),
        revision: 1,
        status: "pending".into(),
        payload,
        evidence,
    };
    db.execute(
        "INSERT INTO feedback_drafts VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            draft.draft_id,
            id,
            draft.payload.version_id,
            draft.revision,
            draft.status,
            encode(&draft)?
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(draft)
}
pub fn read_on(db: &Connection, id: &str, draft: &str) -> Result<FeedbackDraft, String> {
    decode(
        db.query_row(
            "SELECT payload FROM feedback_drafts WHERE mod_id=?1 AND draft_id=?2",
            params![id, draft],
            |r| r.get(0),
        )
        .map_err(|_| "反馈草稿不存在或不属于项目".to_string())?,
    )
}
pub fn edit_on(
    db: &Connection,
    id: &str,
    draft: &str,
    revision: i64,
    payload: FeedbackPayload,
    evidence: Vec<StoredEvidence>,
) -> Result<FeedbackDraft, String> {
    validate_payload(db, id, &payload)?;
    let old = read_on(db, id, draft)?;
    if old.payload.version_id != payload.version_id {
        return Err("编辑不能切换草稿对应版本，请新建反馈".into());
    }
    let updated = FeedbackDraft {
        payload,
        evidence,
        revision: revision.checked_add(1).ok_or("revision 溢出")?,
        ..old
    };
    if db.execute("UPDATE feedback_drafts SET revision=?4,payload=?5 WHERE mod_id=?1 AND draft_id=?2 AND revision=?3 AND status='pending'",params![id,draft,revision,updated.revision,encode(&updated)?]).map_err(|e|e.to_string())?!=1{return Err("草稿已变化或已处理，请重载；输入已保留".into());}
    Ok(updated)
}
pub fn confirm_on(
    db: &mut Connection,
    id: &str,
    draft: &str,
    revision: i64,
    supersedes: Option<String>,
) -> Result<VerificationRecord, String> {
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let old = read_on(&tx, id, draft)?;
    if old.revision != revision {
        return Err("草稿已变化，请重新查看后确认".into());
    }
    let existing: Option<String> = tx
        .query_row(
            "SELECT payload FROM verification_records WHERE source_draft_id=?1",
            [draft],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(raw) = existing {
        let record: VerificationRecord = decode(raw)?;
        if record.supersedes != supersedes {
            return Err("重复确认的更正目标不同".into());
        }
        return Ok(record);
    }
    if old.status != "pending" {
        return Err("草稿已拒绝，不能确认".into());
    }
    if let Some(target) = &supersedes {
        let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM verification_records WHERE record_id=?1 AND mod_id=?2 AND version_id=?3)",params![target,id,old.payload.version_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !valid {
            return Err("更正目标不属于当前版本".into());
        }
    }
    let record = VerificationRecord {
        record_id: uuid::Uuid::new_v4().to_string(),
        mod_id: id.into(),
        version_id: old.payload.version_id.clone(),
        source_draft_id: draft.into(),
        confirmed_at: crate::storage::mods::now()?,
        payload: old.payload.clone(),
        evidence: old.evidence.clone(),
        supersedes,
    };
    let confirmed = FeedbackDraft {
        status: "confirmed".into(),
        ..old
    };
    tx.execute(
        "UPDATE feedback_drafts SET status='confirmed',payload=?2 WHERE draft_id=?1",
        params![draft, encode(&confirmed)?],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO verification_records VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            record.record_id,
            draft,
            id,
            record.version_id,
            record.confirmed_at,
            encode(&record)?,
            record.supersedes
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(record)
}
pub fn reject_on(db: &Connection, id: &str, draft: &str, revision: i64) -> Result<(), String> {
    let mut old = read_on(db, id, draft)?;
    old.status = "rejected".into();
    if db.execute("UPDATE feedback_drafts SET status='rejected',payload=?4 WHERE mod_id=?1 AND draft_id=?2 AND revision=?3 AND status='pending'",params![id,draft,revision,encode(&old)?]).map_err(|e|e.to_string())?!=1{return Err("草稿已变化或已处理".into());}
    Ok(())
}
pub fn list_on(
    db: &Connection,
    id: &str,
    version: Option<&str>,
) -> Result<VerificationList, String> {
    let mut drafts=db.prepare("SELECT payload FROM feedback_drafts WHERE mod_id=?1 AND (?2 IS NULL OR version_id=?2) ORDER BY rowid DESC").map_err(|e|e.to_string())?;
    let rows = drafts
        .query_map(params![id, version], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let drafts = rows
        .map(|r| decode(r.map_err(|e| e.to_string())?))
        .collect::<Result<_, _>>()?;
    let mut records=db.prepare("SELECT payload FROM verification_records WHERE mod_id=?1 AND (?2 IS NULL OR version_id=?2) ORDER BY rowid DESC").map_err(|e|e.to_string())?;
    let rows = records
        .query_map(params![id, version], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    Ok(VerificationList {
        drafts,
        records: rows
            .map(|r| decode(r.map_err(|e| e.to_string())?))
            .collect::<Result<_, _>>()?,
    })
}
