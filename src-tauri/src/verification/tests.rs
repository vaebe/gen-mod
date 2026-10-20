use super::{models::*, repository::*};
use rusqlite::Connection;
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE projects(mod_id TEXT PRIMARY KEY);INSERT INTO projects VALUES('a'),('b');").unwrap();
    db.execute_batch(include_str!(
        "../storage/migrations/004_delivery_metadata.sql"
    ))
    .unwrap();
    db.execute_batch(include_str!("../storage/migrations/005_verification.sql"))
        .unwrap();
    let version = crate::storage::models::ArtifactVersion {
        version_id: "v1".into(),
        task_id: "t1".into(),
        created_at: "now".into(),
        skills: vec![],
        files: vec![],
        zip_sha256: "hash".into(),
    };
    crate::artifacts::delivery_metadata::ensure_anchor_on(&db, "a", &version).unwrap();
    db
}
fn payload() -> FeedbackPayload {
    FeedbackPayload {
        version_id: "v1".into(),
        game_version: None,
        observations: "切场景失效".into(),
        result: VerificationResult::Failed,
        case_results: vec![],
        source_refs: vec![EvidenceRef {
            session_id: "s".into(),
            entry_id: "e".into(),
            attachment_index: None,
            sha256: "hash".into(),
        }],
    }
}
#[test]
fn stale_confirmation_does_not_confirm_edited_draft() {
    let mut db = fixture();
    let first = propose_on(&db, "a", payload(), vec![]).unwrap();
    let mut next = payload();
    next.observations = "已补充日志".into();
    edit_on(&db, "a", &first.draft_id, first.revision, next, vec![]).unwrap();
    assert!(confirm_on(&mut db, "a", &first.draft_id, first.revision, None).is_err());
    assert!(list_on(&db, "a", Some("v1")).unwrap().records.is_empty());
}
#[test]
fn confirmation_is_idempotent_project_scoped_and_survives_deleted_package() {
    let mut db = fixture();
    let draft = propose_on(&db, "a", payload(), vec![]).unwrap();
    assert!(confirm_on(&mut db, "b", &draft.draft_id, draft.revision, None).is_err());
    let record = confirm_on(&mut db, "a", &draft.draft_id, draft.revision, None).unwrap();
    assert_eq!(
        confirm_on(&mut db, "a", &draft.draft_id, draft.revision, None)
            .unwrap()
            .record_id,
        record.record_id
    );
    crate::artifacts::delivery_metadata::mark_deleted_on(&db, "a", "v1", true).unwrap();
    assert_eq!(list_on(&db, "a", Some("v1")).unwrap().records.len(), 1);
    assert!(list_on(&db, "a", Some("v2")).unwrap().records.is_empty());
}

#[test]
fn rejection_prevents_confirmation_and_corrections_append() {
    let mut db = fixture();
    let draft = propose_on(&db, "a", payload(), vec![]).unwrap();
    reject_on(&db, "a", &draft.draft_id, draft.revision).unwrap();
    assert!(confirm_on(&mut db, "a", &draft.draft_id, draft.revision, None).is_err());
    let next = propose_on(&db, "a", payload(), vec![]).unwrap();
    let first = confirm_on(&mut db, "a", &next.draft_id, next.revision, None).unwrap();
    let correction = propose_on(&db, "a", payload(), vec![]).unwrap();
    let second = confirm_on(
        &mut db,
        "a",
        &correction.draft_id,
        correction.revision,
        Some(first.record_id.clone()),
    )
    .unwrap();
    assert_eq!(second.supersedes, Some(first.record_id));
    assert_eq!(list_on(&db, "a", Some("v1")).unwrap().records.len(), 2);
    let mut forged = serde_json::to_value(payload()).unwrap();
    forged["confirmedAt"] = "fake".into();
    assert!(serde_json::from_value::<FeedbackPayload>(forged).is_err());
}
