use super::*;
fn asset(derived:bool)->ProjectAsset{
    let id=uuid::Uuid::new_v4().to_string();
    ProjectAsset{asset_id:id.clone(),mod_id:"p".into(),original_name:"original.jpg".into(),relative_path:format!("assets/{}/{id}.{}",if derived{"derived"}else{"originals"},if derived{"png"}else{"jpg"}),sha256:"hash".into(),width:1,height:1,media_type:if derived{"image/png"}else{"image/jpeg"}.into(),input_asset_id:if derived{Some("old".into())}else{None},recipe:None}
}
fn database()->rusqlite::Connection{
    let db=rusqlite::Connection::open_in_memory().unwrap();db.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE projects(mod_id TEXT PRIMARY KEY);INSERT INTO projects VALUES('p'),('other');").unwrap();
    db.execute_batch(include_str!("../storage/migrations/008_assets.sql")).unwrap();db
}
#[test]
fn original_and_derived_legacy_records_are_readable_without_decoding(){
    let db=database();let root=tempfile::tempdir().unwrap();
    for derived in [false,true]{
        let mut value=asset(derived);let path=root.path().join(&value.relative_path);std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(&path,b"opaque original bytes").unwrap();
        value.sha256=crate::changes::file_hash(&path).unwrap();
        assert_eq!(asset_path_on(root.path(),&value).unwrap(),path);
        assert_eq!(reconcile_metadata_on(&db,"p",&value,None,true).unwrap(),"reconciled");
        assert!(repository::read_on(&db,"other",&value.asset_id).is_err());
        assert_eq!(std::fs::read(path).unwrap(),b"opaque original bytes");
    }
    assert_eq!(repository::list_on(&db,"p").unwrap().len(),2);
}
#[test]
fn missing_bad_hash_and_interrupted_task_never_register_success(){
    let db=database();let root=tempfile::tempdir().unwrap();let value=asset(false);
    assert!(asset_path_on(root.path(),&value).is_err());
    assert_eq!(reconcile_metadata_on(&db,"p",&value,None,false).unwrap(),"requires_review");
    let path=root.path().join(&value.relative_path);std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(&path,b"changed").unwrap();
    let valid=asset_path_on(root.path(),&value).and_then(|p|crate::changes::file_hash(&p)).is_ok_and(|hash|hash==value.sha256);
    assert!(!valid);assert_eq!(reconcile_metadata_on(&db,"p",&value,None,valid).unwrap(),"requires_review");
    assert_eq!(reconcile_metadata_on(&db,"p",&value,Some("interrupted-task"),true).unwrap(),"interrupted");
    assert!(repository::list_on(&db,"p").unwrap().is_empty());
}
