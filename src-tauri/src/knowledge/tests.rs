use super::{models::*, repository::*, search::*};
use rusqlite::Connection;
fn note(id: &str, status: &str, game: &str, version: &str) -> KnowledgeNote {
    KnowledgeNote {
        note_id: id.into(),
        revision: 1,
        game_id: game.into(),
        status: status.into(),
        allow_reuse: true,
        origin_mod_id: None, origin_project_name: None, user_edited: false,
        memory_key: format!("fixture:{id}"), created_at: "now".into(), updated_at: "now".into(),
        input: KnowledgeInput {
            version_scope: "exact".into(),
            game_version: Some(version.into()),
            title: "悬浮".into(),
            content: "玩家观察".into(),
            route: "event".into(),
            evidence_kind: "user_observation".into(),
            source_refs: vec![],
        },
    }
}
#[test]
fn excludes_unaccepted_wrong_version_and_private_notes() {
    let db = database();
    for item in [
        note("accepted", "active", "duckov", "a"),
        note("old", "active", "duckov", "old"),
        note("archived", "archived", "duckov", "a"),
        note("other", "active", "other", "a"),
    ] {
        insert_on(&db, "a", &item).unwrap();
    }
    let result = search_notes(&db, "duckov", Some("a"), &["悬浮".into()], 5).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].note_id, "accepted");
    assert!(search_notes(&db, "duckov", None, &["悬浮".into()], 5)
        .unwrap()
        .is_empty());
    let mut independent = note("independent", "active", "duckov", "a");
    independent.input.version_scope = "independent".into();
    independent.input.game_version = None;
    insert_on(&db, "a", &independent).unwrap();
    assert_eq!(
        search_notes(&db, "duckov", None, &[], 1).unwrap()[0].note_id,
        "independent"
    );
    let mut private = note("private", "active", "duckov", "a");
    private.allow_reuse = false;
    insert_on(&db, "a", &private).unwrap();
    assert_eq!(
        search_notes(&db, "duckov", Some("a"), &[], 5)
            .unwrap()
            .len(),
        2
    );
}

fn database() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    db.execute_batch(include_str!("../storage/schema.sql")).unwrap();
    db.execute("INSERT INTO projects(mod_id,payload,revision) VALUES('p','{}',0)", []).unwrap();
    db
}

#[test]
fn user_edit_and_delete_use_cas_and_keep_body_protection() {
    let mut db = database();
    let mut original = note("n", "active", "duckov", "a");
    original.revision = 7;
    original.input.content = "原正文".into();
    insert_global_on(&db, &original).unwrap();
    let edited = edit_on(&mut db, "n", 7, "新标题", "新正文").unwrap();
    assert_eq!(edited.revision, 8);
    assert!(edited.user_edited);
    assert!(edit_on(&mut db, "n", 7, "过期", "过期").is_err());
    assert_eq!(read_on(&db, "n").unwrap().input.content, "新正文");
    set_status_on(&mut db, "n", 8, "deleted").unwrap();
    let deleted = read_on(&db, "n").unwrap();
    assert_eq!(deleted.status, "deleted");
    assert!(deleted.input.content.is_empty());
    assert!(!deleted.allow_reuse);
    let links: i64 = db.query_row("SELECT COUNT(*) FROM memory_content_links WHERE note_id='n'", [], |row| row.get(0)).unwrap();
    assert_eq!(links, 2);
    assert!(set_status_on(&mut db, "n", 9, "active").is_err());
}

fn running_fixture() -> (Connection, crate::pi::tasks::TaskContext) {
    let db = database();
    db.execute("INSERT INTO sessions(mod_id,session_id,payload,generation) VALUES('p','s','{}',1)", []).unwrap();
    db.execute("INSERT INTO tasks(task_id,mod_id,session_id,status,created_at,payload) VALUES('task','p','s','running','now','{}')", []).unwrap();
    (db, crate::pi::tasks::TaskContext { task_id: "task".into(), mod_id: "p".into(), session_id: "s".into(), generation: 1,
        source_directory: String::new(), staging_id: "staging".into(), staging_directory: String::new(), created_at: "now".into(), context_snapshot_id: "snapshot".into(), memory_bootstrap_path:None,memory_bootstrap_sha256:None })
}

fn automatic_input() -> SaveLongTermMemoryInput {
    SaveLongTermMemoryInput { note_id: None, expected_revision: None, topic_key: "resource-load".into(), title: "资源读取".into(), content: "旧方法正文".into(),
        version_scope: "unknown".into(), game_version: None, evidence_kind: "development".into(),
        source_refs: vec![SourceRef { kind: "command".into(), id: "receipt-a".into(), task_id: Some("task".into()), ..Default::default() }] }
}

#[test]
fn concurrent_automatic_saves_with_new_sources_create_one_identity(){
    let (db,context)=running_fixture();
    let root=tempfile::tempdir().unwrap();let path=root.path().join("memory.sqlite3");
    db.execute("VACUUM INTO ?1",[path.to_str().unwrap()]).unwrap();
    let barrier=std::sync::Arc::new(std::sync::Barrier::new(3));
    let workers:Vec<_>=(0..2).map(|index|{
        let path=path.clone();let barrier=barrier.clone();let context=context.clone();
        std::thread::spawn(move||{
            let mut connection=Connection::open(path).unwrap();
            connection.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
            connection.pragma_update(None,"foreign_keys",true).unwrap();
            let mut input=automatic_input();input.source_refs[0].id=format!("new-source-{index}");
            barrier.wait();
            save_automatic_on(&mut connection,&context,"duckov","项目",&input,&std::sync::atomic::AtomicBool::new(false)).unwrap()
        })
    }).collect();
    barrier.wait();
    let results:Vec<_>=workers.into_iter().map(|worker|worker.join().unwrap()).collect();
    assert_eq!(results[0].note_id,results[1].note_id);
    assert_eq!(results.iter().filter(|result|result.outcome=="saved").count(),1);
    assert_eq!(results.iter().filter(|result|result.outcome=="unchanged").count(),1);
    let mut connection=Connection::open(path).unwrap();
    assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM knowledge_notes",[],|r|r.get(0)).unwrap(),1);
    connection.execute("UPDATE tasks SET status='finished' WHERE task_id=?1",[&context.task_id]).unwrap();
    connection.execute("UPDATE sessions SET generation=2 WHERE mod_id=?1 AND session_id=?2",rusqlite::params![context.mod_id,context.session_id]).unwrap();
    connection.execute("INSERT INTO tasks(task_id,mod_id,session_id,status,created_at,payload) VALUES('next-task','p','s','running','later','{}')",[]).unwrap();
    let mut next=context;next.task_id="next-task".into();next.generation=2;
    let mut input=automatic_input();input.source_refs[0].id="next-task-source".into();
    let reused=save_automatic_on(&mut connection,&next,"duckov","项目",&input,&std::sync::atomic::AtomicBool::new(false)).unwrap();
    assert_eq!(reused.outcome,"unchanged");assert_eq!(reused.note_id,results[0].note_id);
}

#[test]
fn new_source_and_topic_cannot_recreate_user_edited_or_deleted_content() {
    let (mut db, task) = running_fixture();
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    let mut input = automatic_input();
    let first = save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).unwrap();
    assert_eq!(first.outcome, "saved");
    input.source_refs[0].id = "receipt-b".into();
    assert_eq!(save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).unwrap().outcome, "unchanged");
    let edited = edit_on(&mut db, &first.note_id, 1, "玩家纠正", "新方法正文").unwrap();
    input.topic_key = "different-topic".into();
    input.source_refs[0].id = "receipt-c".into();
    assert_eq!(save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).unwrap().outcome, "user_managed");
    assert_eq!(read_on(&db, &first.note_id).unwrap().input.content, "新方法正文");
    set_status_on(&mut db, &first.note_id, edited.revision, "deleted").unwrap();
    assert_eq!(save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).unwrap().outcome, "user_managed");
    let count: i64 = db.query_row("SELECT COUNT(*) FROM knowledge_notes", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn automatic_update_requires_cas_and_an_uncancelled_live_task() {
    let (mut db, task) = running_fixture();
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    let mut input = automatic_input();
    let first = save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).unwrap();
    input.content = "改进后的方法".into();
    assert!(save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).is_err());
    input.note_id = Some(first.note_id.clone()); input.expected_revision = Some(1);
    let updated = save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).unwrap();
    assert_eq!(updated.revision, 2);
    assert!(save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).is_err());
    input.expected_revision = Some(2); input.content = "迟到的方法".into();
    cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).is_err());
    cancelled.store(false, std::sync::atomic::Ordering::SeqCst);
    db.execute("UPDATE tasks SET status='cancelled'", []).unwrap();
    assert!(save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).is_err());
    assert_eq!(read_on(&db, &first.note_id).unwrap().input.content, "改进后的方法");
}

#[test]
fn message_and_command_evidence_reject_forgery_and_other_projects() {
    let (db, task) = running_fixture();
    let entry = serde_json::json!({"id":"entry","message":{"role":"user","content":"加载失败"}});
    let mut source = SourceRef { kind: "user_message".into(), id: "entry".into(), session_id: Some(task.session_id.clone()),
        sha256: Some(crate::verification::evidence::digest(&entry["message"]).unwrap()), ..Default::default() };
    assert!(super::evidence::validate_message(&[entry.clone()], &task, &source).is_ok());
    source.sha256 = Some("forged".into());
    assert!(super::evidence::validate_message(&[entry.clone()], &task, &source).is_err());
    source.session_id = Some("other-session".into());
    assert!(super::evidence::validate_message(&[entry], &task, &source).is_err());
    crate::project_context::repository::snapshot_on(&db, &task.mod_id, &task.task_id, &task.context_snapshot_id, &task.created_at).unwrap();
    let record = serde_json::json!({"context":task,"status":"running","commands":[{"taskId":"task","commandId":"actual-receipt","toolCallId":"tool","exitCode":1,"status":"failed","logPath":"fixture.log","program":"build","args":[],"sourceRevision":0,"startedAt":"now","durationMs":0}]});
    db.execute("UPDATE tasks SET payload=?1 WHERE task_id='task'", [record.to_string()]).unwrap();
    let mut command = SourceRef { kind: "command".into(), id: "actual-receipt".into(), task_id: Some("task".into()), ..Default::default() };
    assert_eq!(super::evidence::validate_command_on(&db, &task, &command).unwrap(), None);
    command.id = "invented-receipt".into();
    assert!(super::evidence::validate_command_on(&db, &task, &command).is_err());
    command.id = "actual-receipt".into();
    let mut other = task.clone(); other.mod_id = "other-project".into();
    assert!(super::evidence::validate_command_on(&db, &other, &command).is_err());
}

#[test]
fn historical_usage_keeps_the_returned_revision_after_edit_and_delete() {
    let (mut db, task) = running_fixture();
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    let saved = save_automatic_on(&mut db, &task, "duckov", "项目", &automatic_input(), &cancelled).unwrap();
    let packet = serde_json::json!({"note":read_on(&db, &saved.note_id).unwrap(),"needsReview":true,"originRemoved":false});
    super::usage::record_uses_on(&db, &task, &[packet.clone()]).unwrap();
    super::usage::record_uses_on(&db, &task, &[packet]).unwrap();
    edit_on(&mut db, &saved.note_id, 1, "更正", "更正后的正文").unwrap();
    set_status_on(&mut db, &saved.note_id, 2, "deleted").unwrap();
    let (count, revision, body): (i64,i64,String) = db.query_row("SELECT COUNT(*),revision,json_extract(payload,'$.note.content') FROM task_memory_uses WHERE task_id='task'", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!((count,revision,body.as_str()), (1,1,"旧方法正文"));
}

#[test]
fn retrieval_includes_unknown_and_explicit_manual_global_but_not_automatic_global() {
    let (db, _) = running_fixture();
    let mut unknown = note("unknown", "active", "duckov", "old");
    unknown.input.version_scope = "unknown".into(); unknown.input.game_version = None;
    insert_on(&db, "p", &unknown).unwrap();
    let mut global = note("global", "active", "*", "old");
    global.input.version_scope = "independent".into(); global.input.game_version = None;
    global.input.evidence_kind = "manual".into(); global.user_edited = true;
    insert_on(&db, "p", &global).unwrap();
    let mut invalid = global.clone(); invalid.note_id = "invalid-global".into(); invalid.memory_key = "invalid-key".into(); invalid.user_edited = false;
    insert_on(&db, "p", &invalid).unwrap();
    insert_on(&db, "p", &note("exact-old", "active", "duckov", "old")).unwrap();
    let notes = search_notes(&db, "duckov", None, &[], 5).unwrap();
    assert_eq!(notes.len(), 2);
    assert!(notes.iter().any(|note| note.note_id == "unknown"));
    assert!(notes.iter().any(|note| note.note_id == "global"));
    assert_eq!(search_notes(&db, "duckov", None, &[], 1).unwrap().len(), 1);
}

#[test]
fn identical_automatic_content_keeps_separate_exact_version_identity() {
    let (mut db, task) = running_fixture();
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    let mut input = automatic_input(); input.version_scope = "exact".into(); input.game_version = Some("a".into());
    let first = save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).unwrap();
    input.game_version = Some("b".into());
    let second = save_automatic_on(&mut db, &task, "duckov", "项目", &input, &cancelled).unwrap();
    assert_ne!(first.note_id, second.note_id);
    assert_eq!(search_notes(&db, "duckov", Some("b"), &[], 5).unwrap()[0].note_id, second.note_id);
}
