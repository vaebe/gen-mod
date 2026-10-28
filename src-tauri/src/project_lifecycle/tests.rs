use super::repository::*;
use rusqlite::Connection;

pub(super) fn database() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    for sql in [include_str!("../storage/migrations/001_initial.sql"),include_str!("../storage/migrations/002_session_model.sql"),include_str!("../storage/migrations/003_project_context.sql"),include_str!("../storage/migrations/004_delivery_metadata.sql"),include_str!("../storage/migrations/005_verification.sql"),include_str!("../storage/migrations/006_knowledge.sql"),include_str!("../storage/migrations/007_inspection.sql"),include_str!("../storage/migrations/008_assets.sql"),include_str!("../storage/migrations/009_memory.sql"),include_str!("../storage/migrations/010_project_lifecycle.sql")] {
        db.execute_batch(sql).unwrap();
    }
    db
}
fn seed(db: &Connection, id: &str) {
    let payload = serde_json::json!({"modId":id,"name":"项目","gameId":"duckov","gameSkillName":"duckov-mod","draftRequirement":"需求","updatedAt":"now","developmentStatus":"draft","versions":[],"installations":[],"installedTargets":[]});
    db.execute("INSERT INTO projects(mod_id,payload,revision) VALUES(?1,?2,0)",rusqlite::params![id,payload.to_string()]).unwrap();
}

fn fixture() -> (Connection,tempfile::TempDir,String,String) {
    let db=database();let root=tempfile::tempdir().unwrap();
    let id=uuid::Uuid::new_v4().to_string();let other=uuid::Uuid::new_v4().to_string();
    for project in [&id,&other] {
        seed(&db,project);
        for child in ["source","logs","artifacts","assets"] {
            let directory=root.path().join(project).join(child);std::fs::create_dir_all(&directory).unwrap();std::fs::write(directory.join("keep.bin"),b"original").unwrap();
        }
    }
    (db,root,id,other)
}
fn removal(db:&Connection,root:&std::path::Path,id:&str)->super::recovery::Removal {
    let value=super::recovery::Removal{mod_id:id.into(),operation_id:uuid::Uuid::new_v4().to_string(),revision:0,directory_identity:super::recovery::directory_identity(&root.join(id)).unwrap()};
    super::recovery::journal_on(db,&value,"pending").unwrap();value
}
#[test]
fn permanent_delete_preserves_global_memory_other_project_and_external_files() {
    let (mut db,root,id,other)=fixture();
    let external=tempfile::tempdir().unwrap();std::fs::write(external.path().join("installed.bin"),b"installed").unwrap();
    let note=serde_json::json!({"content":"跨项目方法"});
    db.execute("INSERT INTO knowledge_notes(note_id,origin_mod_id,game_id,revision,status,allow_reuse,user_edited,memory_key,payload) VALUES('n',?1,'duckov',1,'active',1,1,'manual:n',?2)",rusqlite::params![id,note.to_string()]).unwrap();
    db.execute("INSERT INTO memory_content_links VALUES('duckov','hash','n')",[]).unwrap();
    db.execute("INSERT INTO sessions(mod_id,session_id,payload) VALUES(?1,'s','{\"sessionId\":\"s\",\"sessionFile\":\"\",\"skills\":[],\"readOnly\":false}')",[&id]).unwrap();
    db.execute("INSERT INTO tasks VALUES('t',?1,'s','finished','now','now','{}')",[&id]).unwrap();
    db.execute("INSERT INTO task_memory_uses VALUES('t',?1,'n',1,'old copy')",[&id]).unwrap();
    for table in ["project_contexts","context_snapshots","context_proposals","delivery_metadata","inspection_reports","project_assets"] {
        let sql=match table {
            "project_contexts"=>"INSERT INTO project_contexts VALUES(?1,1,'{}')",
            "context_snapshots"=>"INSERT INTO context_snapshots VALUES('snap',?1,'t','{}')",
            "context_proposals"=>"INSERT INTO context_proposals VALUES('proposal',?1,'pending','{}')",
            "delivery_metadata"=>"INSERT INTO delivery_metadata VALUES(?1,'v','t','{}')",
            "inspection_reports"=>"INSERT INTO inspection_reports VALUES('report',?1,'path','{}')",
            _=>"INSERT INTO project_assets VALUES('asset',?1,'{}')"};db.execute(sql,[&id]).unwrap();
    }
    db.execute("INSERT INTO feedback_drafts VALUES('f',?1,'v',1,'confirmed','{}')",[&id]).unwrap();
    db.execute("INSERT INTO verification_records VALUES('vr','f',?1,'v','now','{}',NULL)",[&id]).unwrap();
    db.execute("INSERT INTO runtime_logs VALUES(1,?1,'s','t','now','{}')",[&id]).unwrap();
    db.execute("INSERT INTO steering_inputs VALUES('input',?1,'s','t',1,'now','confirmed','{}')",[&id]).unwrap();
    db.execute("INSERT INTO session_stats VALUES(?1,'s','now','{}')",[&id]).unwrap();
    db.execute("INSERT INTO legacy_imports VALUES(?1,NULL,'now')",[&id]).unwrap();
    let removal=super::recovery::begin_delete(&mut db,root.path(),&id,0).unwrap();
    assert!(!root.path().join(&id).exists());
    let result=super::recovery::finish_delete(&db,root.path(),&removal);
    assert!(result.removed && !result.cleanup_pending);
    assert_eq!(std::fs::read(root.path().join(other).join("source/keep.bin")).unwrap(),b"original");
    assert_eq!(std::fs::read(external.path().join("installed.bin")).unwrap(),b"installed");
    assert_eq!(db.query_row::<i64,_,_>("SELECT COUNT(*) FROM knowledge_notes",[],|r|r.get(0)).unwrap(),1);
    assert_eq!(db.query_row::<i64,_,_>("SELECT COUNT(*) FROM memory_content_links",[],|r|r.get(0)).unwrap(),1);
    for table in ["task_memory_uses","project_contexts","context_snapshots","context_proposals","delivery_metadata","inspection_reports","project_assets","verification_records","feedback_drafts","runtime_logs","steering_inputs","session_stats","tasks","sessions","legacy_imports"] {
        assert_eq!(db.query_row::<i64,_,_>(&format!("SELECT COUNT(*) FROM {table} WHERE mod_id=?1"),[&id],|r|r.get(0)).unwrap(),0,"{table}");
    }
    assert!(!db.prepare("PRAGMA foreign_key_check").unwrap().exists([]).unwrap());
    assert!(ensure_writable_on(&db,&id).is_err());
    super::recovery::recover_one(&mut db,root.path(),&removal).unwrap();
}
#[test]
fn deletion_recovery_matrix_preserves_conflicts_and_never_replays_delete() {
    for phase in ["allocated","renamed","committed","conflict"] {
        let (mut db,root,id,_)=fixture();let value=removal(&db,root.path(),&id);
        let (original,temporary)=super::recovery::controlled_paths(root.path(),&id,&value.operation_id).unwrap();
        if phase!="allocated" {std::fs::rename(&original,&temporary).unwrap();}
        if phase=="committed" {delete_private_rows_on(&mut db,&id,&value.operation_id,"now").unwrap();}
        if phase=="conflict" {std::fs::create_dir(&original).unwrap();std::fs::write(original.join("unrelated"),b"keep").unwrap();}
        let result=super::recovery::recover_one(&mut db,root.path(),&value);
        if phase=="conflict" {
            assert!(result.is_err());assert!(temporary.exists());
            assert_eq!(std::fs::read(original.join("unrelated")).unwrap(),b"keep");
            assert!(ensure_writable_on(&db,&id).is_err());
            assert!(super::recovery::recover_one(&mut db,root.path(),&value).is_err());
        }else {
            result.unwrap();assert!(!temporary.exists());
            assert_eq!(original.exists(),phase!="committed");
            assert_eq!(ensure_writable_on(&db,&id).is_ok(),phase!="committed");
        }
    }
}
#[test]
fn database_failure_rolls_back_and_restores_original_directory() {
    let (mut db,root,id,_)=fixture();
    db.execute_batch("CREATE TRIGGER fail_removal BEFORE INSERT ON project_removals BEGIN SELECT RAISE(ABORT,'fixture failure'); END;").unwrap();
    assert!(super::recovery::begin_delete(&mut db,root.path(),&id,0).is_err());
    assert!(root.path().join(&id).join("source/keep.bin").is_file());
    ensure_writable_on(&db,&id).unwrap();
    assert!(!db.prepare("PRAGMA foreign_key_check").unwrap().exists([]).unwrap());
}
#[cfg(windows)]
#[test]
fn cleanup_stops_at_a_junction_and_keeps_external_files(){
    use std::os::windows::process::CommandExt;
    let (mut db,root,id,_)=fixture();let external=tempfile::tempdir().unwrap();
    std::fs::write(external.path().join("installed.bin"),b"keep external").unwrap();
    let link=root.path().join(&id).join("external-link");
    let result=std::process::Command::new("cmd.exe").args(["/C","mklink","/J"]).arg(&link).arg(external.path()).creation_flags(0x08000000).output().unwrap();
    assert!(result.status.success(),"junction fixture creation failed: {}",String::from_utf8_lossy(&result.stderr));
    let removal=super::recovery::begin_delete(&mut db,root.path(),&id,0).unwrap();
    let result=super::recovery::finish_delete(&db,root.path(),&removal);assert!(result.removed && result.cleanup_pending);
    assert_eq!(std::fs::read(external.path().join("installed.bin")).unwrap(),b"keep external");
    let (_,temporary)=super::recovery::controlled_paths(root.path(),&id,&removal.operation_id).unwrap();
    std::fs::remove_dir(temporary.join("external-link")).unwrap();
    super::recovery::recover_one(&mut db,root.path(),&removal).unwrap();
}
#[test]
fn archive_restore_cas_and_read_have_no_side_effects() {
    let mut db = database();
    seed(&db,"p"); seed(&db,"other");
    db.execute("INSERT INTO project_contexts VALUES('p',4,'{\"memory\":\"keep\"}')",[]).unwrap();
    let archived = set_archived_on(&mut db,"p",0,true).unwrap();
    assert!(archived.archived);
    assert_eq!(archived.revision,1);
    assert!(ensure_writable_on(&db,"p").is_err());
    assert!(crate::storage::repository::save_on(&db,&archived,false).is_err());
    let raw: String = db.query_row("SELECT payload FROM projects WHERE mod_id='p'",[],|r|r.get(0)).unwrap();
    for _ in 0..3 { assert!(crate::storage::repository::read_on(&db,"p").unwrap().archived); }
    assert_eq!(raw,db.query_row::<String,_,_>("SELECT payload FROM projects WHERE mod_id='p'",[],|r|r.get(0)).unwrap());
    assert!(set_archived_on(&mut db,"p",0,false).is_err());
    let restored=set_archived_on(&mut db,"p",1,false).unwrap();
    assert!(!restored.archived);
    ensure_writable_on(&db,"p").unwrap();
    assert_eq!(db.query_row::<i64,_,_>("SELECT revision FROM project_contexts WHERE mod_id='p'",[],|r|r.get(0)).unwrap(),4);
    assert_eq!(crate::storage::repository::read_on(&db,"other").unwrap().revision,0);
}
#[test]
fn unresolved_removals_block_writes_until_verified_recovery() {
    let db=database(); seed(&db,"p");
    for status in ["pending","requires_review","interrupted"] {
        db.execute("INSERT OR REPLACE INTO file_operations VALUES('op','p','project_delete',?1,'{}')",[status]).unwrap();
        assert_eq!(ensure_writable_on(&db,"p").is_ok(),status=="interrupted");
        let project=crate::storage::repository::read_on(&db,"p").unwrap();
        assert_eq!(crate::storage::repository::save_on(&db,&project,false).is_ok(),status=="interrupted");
    }
    db.execute("INSERT INTO project_removals VALUES('p','op','now')",[]).unwrap();
    assert!(ensure_writable_on(&db,"p").is_err());
    assert!(ensure_writable_on(&db,"missing").is_err());
}

#[test]
fn legacy_game_skill_name_does_not_replace_stored_game_id_and_missing_official_keeps_project_readable() {
    let db=database();seed(&db,"legacy");
    db.execute("UPDATE projects SET payload=json_set(payload,'$.gameSkillName','removed-legacy-skill') WHERE mod_id='legacy'",[]).unwrap();
    let project=crate::storage::repository::read_on(&db,"legacy").unwrap();
    assert_eq!(project.game_id,"duckov");assert_eq!(project.game_skill_name,"removed-legacy-skill");
    assert!(crate::skills::registry::resolve_views_for_game(vec![],&project.game_id).is_err());
    assert_eq!(crate::storage::repository::read_on(&db,"legacy").unwrap().game_id,"duckov");
}
