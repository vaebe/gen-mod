use super::{models::*, repository::*};
use rusqlite::Connection;
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    db.execute_batch(include_str!("../storage/schema.sql")).unwrap();
    db.execute_batch("INSERT INTO projects(mod_id,payload) VALUES('a','{}'),('b','{}');").unwrap();
    db
}

#[test]
fn automatic_update_keeps_task_snapshot_and_checks_live_revision() {
    use std::sync::atomic::{AtomicBool,Ordering};
    let mut db=fixture();initialize_on(&db,"a").unwrap();initialize_on(&db,"a").unwrap();
    assert_eq!(read_on(&db,"a").unwrap().revision,0);
    let task:crate::pi::tasks::TaskContext=serde_json::from_value(serde_json::json!({"taskId":"t","modId":"a","sessionId":"s","generation":1,"sourceDirectory":"source","stagingId":"staging","stagingDirectory":"staging","createdAt":"now","contextSnapshotId":"snap"})).unwrap();
    db.execute("INSERT INTO sessions(mod_id,session_id,payload,generation) VALUES('a','s','{}',1)",[]).unwrap();
    let record=crate::pi::tasks::TaskRecord{context:task.clone(),status:"running".into(),finished_at:None,commands:vec![],registered_version_id:None};
    db.execute("INSERT INTO tasks VALUES('t','a','s','running','now',NULL,?1)",[serde_json::to_string(&record).unwrap()]).unwrap();
    snapshot_on(&db,"a","t","snap","now").unwrap();
    let message=serde_json::json!({"id":"e","message":{"role":"user","content":"保留原色，版本 1.2"}});
    let source=ProjectMemorySourceRef::UserMessage{session_id:"s".into(),entry_id:"e".into(),sha256:crate::verification::evidence::digest(&message["message"]).unwrap()};
    let mut input=ProjectMemoryUpdateInput{expected_revision:0,changes:vec![ProjectMemoryChange{field:"confirmedRequirements".into(),reason:"玩家明确要求".into(),value:serde_json::json!(["保留原色"]),source_refs:vec![source.clone()]}]};
    super::evidence::validate_on(&db,&task,&input,&[message.clone()]).unwrap();
    let cancelled=AtomicBool::new(false);
    let saved=update_automatic_on(&mut db,&task,&input,&cancelled).unwrap();
    assert_eq!(saved.outcome,"saved");assert_eq!(saved.revision,1);
    assert!(snapshot_for_task_on(&db,"a","t").unwrap().context.confirmed_requirements.is_empty());
    assert_eq!(read_on(&db,"a").unwrap().confirmed_requirements,vec!["保留原色"]);
    assert!(update_automatic_on(&mut db,&task,&input,&cancelled).is_err());
    input.expected_revision=1;assert_eq!(update_automatic_on(&mut db,&task,&input,&cancelled).unwrap().outcome,"unchanged");
    let mut forged=input.clone();forged.changes[0].source_refs=vec![ProjectMemorySourceRef::UserMessage{session_id:"s".into(),entry_id:"e".into(),sha256:"fake".into()}];
    assert!(super::evidence::validate_on(&db,&task,&forged,&[message.clone()]).is_err());
    forged.changes[0].source_refs=vec![ProjectMemorySourceRef::UserMessage{session_id:"other".into(),entry_id:"e".into(),sha256:crate::verification::evidence::digest(&message["message"]).unwrap()}];
    assert!(super::evidence::validate_on(&db,&task,&forged,&[message]).is_err());
    forged.changes[0].source_refs=vec![ProjectMemorySourceRef::Command{task_id:"other".into(),command_id:"receipt".into()}];
    assert!(super::evidence::validate_on(&db,&task,&forged,&[]).is_err());
    cancelled.store(true,Ordering::SeqCst);assert!(update_automatic_on(&mut db,&task,&input,&cancelled).is_err());
    assert_eq!(read_on(&db,"a").unwrap().revision,1);
    assert!(serde_json::from_value::<ProjectMemoryUpdateInput>(serde_json::json!({"expectedRevision":1,"changes":[],"referenceSets":[]})).is_err());
}
#[test]
fn live_edits_preserve_snapshot_and_revision_conflicts() {
    let db = fixture();
    let mut value = ProjectContext::empty("a");
    value.confirmed_requirements.push("original".into());
    let value = save_on(&db, &value, 0).unwrap();
    let first = snapshot_on(&db, "a", "task-a", "snap-a", "now").unwrap();
    let mut changed = value.clone();
    changed.confirmed_requirements = vec!["changed".into()];
    save_on(&db, &changed, value.revision).unwrap();
    assert_eq!(
        read_on(&db, "a").unwrap().confirmed_requirements,
        vec!["changed"]
    );
    assert!(save_on(&db, &value, value.revision).is_err());
    let old = snapshot_for_task_on(&db, "a", "task-a").unwrap();
    assert_eq!(old.context.confirmed_requirements, vec!["original"]);
    assert_eq!(
        snapshot_on(&db, "a", "task-a", "snap-b", "later")
            .unwrap()
            .snapshot_id,
        first.snapshot_id
    );
}
#[test]
fn task_snapshot_is_bound_to_project() {
    let db = fixture();
    snapshot_on(&db, "a", "task-a", "snap-a", "now").unwrap();
    assert!(snapshot_for_task_on(&db, "b", "task-a").is_err());
    assert!(snapshot_on(&db, "b", "task-a", "snap-b", "now").is_err());
    assert!(save_on(&db, &ProjectContext::empty("missing"), 0).is_err());
}

#[test]
fn model_cannot_update_reference_paths() {
    assert!(serde_json::from_value::<ContextChange>(serde_json::json!({"field":"referenceSets","reason":"test","value":"/outside"})).is_err());
}
