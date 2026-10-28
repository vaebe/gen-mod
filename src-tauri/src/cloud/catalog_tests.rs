use super::catalog::*;
use crate::skills::official;
use std::fs;
pub(super) fn sample()->Catalog {
    serde_json::from_value(serde_json::json!({"games":[{"id":"duckov","name":"Duckov","description":"desc"}],"skills":[{"id":"skill-id","name":"mod-workflow","description":"当在 GenMod 工作台新建、续改、排查、整理玩家反馈或交付游戏 MOD 时使用。","kind":"common","gameId":null,"versions":[{"id":"version-id","skillId":"skill-id","version":"2.1.0","contentHash":"ec595d7587ddc7c3bcac08679b73c823438689822b7ed7e6ce2bd8c57d1f7a91","fileId":"file-id","uploadedAt":"2026-10-23T00:00:00Z","manifest":{"category":"common","gameId":null,"version":"2.1.0","dependencies":[],"excludePaths":[".delivery","build"],"requiredCommandScripts":[],"inspectionFile":null}}]}]})).unwrap()
}
#[test] fn checking_and_declining_do_not_install_or_confirm(){
    let root=tempfile::tempdir().unwrap();let catalog=sample();
    let updates=check_on(root.path(),&catalog).unwrap();
    assert_eq!(updates.new_games.len(),1);assert_eq!(updates.versions.len(),1);
    assert!(!root.path().join("mod-workflow/2.1.0").exists());assert!(official::confirmed_on(root.path()).unwrap().versions.is_empty());
}
#[test] fn confirmation_validates_server_hash_and_preserves_old_files(){
    let root=tempfile::tempdir().unwrap();let catalog=sample();check_on(root.path(),&catalog).unwrap();
    fs::create_dir_all(root.path().join("mod-workflow/old")).unwrap();fs::write(root.path().join("mod-workflow/old/keep"),b"old").unwrap();
    official::install_on(root.path(),&catalog.skills[0],&catalog.skills[0].versions[0],include_bytes!("fixtures/mod-workflow.zip")).unwrap();
    assert_eq!(official::confirmed_on(root.path()).unwrap().versions.len(),1);
    assert!(root.path().join("mod-workflow/2.1.0/SKILL.md").exists());assert_eq!(fs::read(root.path().join("mod-workflow/old/keep")).unwrap(),b"old");
    assert!(check_on(root.path(),&catalog).unwrap().versions.is_empty());
}
#[test] fn invalid_download_does_not_confirm_or_remove_existing_content(){
    let root=tempfile::tempdir().unwrap();let catalog=sample();fs::write(root.path().join("keep"),b"keep").unwrap();
    assert!(official::install_on(root.path(),&catalog.skills[0],&catalog.skills[0].versions[0],b"bad zip").is_err());
    assert!(official::confirmed_on(root.path()).unwrap().versions.is_empty());assert_eq!(fs::read(root.path().join("keep")).unwrap(),b"keep");
}
#[test] fn removed_remote_versions_are_not_candidates_or_usable_and_game_metadata_refreshes(){
    let root=tempfile::tempdir().unwrap();let mut catalog=sample();
    official::confirm_games_on(root.path(),catalog.games.clone()).unwrap();
    official::install_on(root.path(),&catalog.skills[0],&catalog.skills[0].versions[0],include_bytes!("fixtures/mod-workflow.zip")).unwrap();
    catalog.games[0].name="新名称".into();catalog.skills[0].versions.clear();
    assert!(check_on(root.path(),&catalog).unwrap().versions.is_empty());
    assert!(official::available_on(root.path()).unwrap().is_empty());assert_eq!(official::confirmed_on(root.path()).unwrap().games[0].name,"新名称");
    assert!(root.path().join("mod-workflow/2.1.0/SKILL.md").exists());
}

#[test] fn confirmation_downloads_by_file_id_only_after_user_selection(){
    use std::{net::TcpListener,io::{Read,Write},thread};
    let root=tempfile::tempdir().unwrap();let config=tempfile::tempdir().unwrap();let catalog=sample();check_on(root.path(),&catalog).unwrap();
    let listener=TcpListener::bind("127.0.0.1:0").unwrap();let url=format!("http://{}",listener.local_addr().unwrap());
    let worker=thread::spawn(move||{let(mut socket,_)=listener.accept().unwrap();let mut request=vec![];let mut byte=[0;1];while !request.ends_with(b"\r\n\r\n"){socket.read_exact(&mut byte).unwrap();request.push(byte[0]);}let request=String::from_utf8(request).unwrap();assert!(request.starts_with("GET /files/file-id "));assert!(request.to_lowercase().contains("authorization: bearer access"));let bytes=include_bytes!("fixtures/mod-workflow.zip");write!(socket,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).unwrap();socket.write_all(bytes).unwrap();});
    let client=super::auth::AuthClient::new(config.path().to_owned(),url).unwrap();
    client.save(&serde_json::from_str(r#"{"accessToken":"access","refreshToken":"refresh","email":"a@b.c","role":"user"}"#).unwrap()).unwrap();
    let packages=tauri::async_runtime::block_on(downloads(&catalog,&["file-id".into()],&client)).unwrap();apply_on(root.path(),&catalog,&[],packages).unwrap();worker.join().unwrap();
    assert_eq!(official::confirmed_on(root.path()).unwrap().versions.len(),1);
}
#[test] fn valid_zip_with_wrong_hash_is_not_registered(){
    let root=tempfile::tempdir().unwrap();let mut catalog=sample();catalog.skills[0].versions[0].content_hash="0".repeat(64);
    assert!(official::install_on(root.path(),&catalog.skills[0],&catalog.skills[0].versions[0],include_bytes!("fixtures/mod-workflow.zip")).unwrap_err().contains("哈希"));
    assert!(official::confirmed_on(root.path()).unwrap().versions.is_empty());
}

#[test] fn checking_catalog_waits_for_task_start_storage_lock(){
    use std::sync::{Arc,Mutex,mpsc};
    let root=tempfile::tempdir().unwrap();let lock=Arc::new(Mutex::new(()));let held=lock.lock().unwrap();
    let (started_tx,started_rx)=mpsc::channel();let(done_tx,done_rx)=mpsc::channel();let path=root.path().to_owned();let worker_lock=lock.clone();
    let worker=std::thread::spawn(move||{started_tx.send(()).unwrap();let result=check_locked_on(&worker_lock,&path,&sample());done_tx.send(result.is_ok()).unwrap();});
    started_rx.recv().unwrap();assert!(done_rx.recv_timeout(std::time::Duration::from_millis(100)).is_err());assert!(!root.path().join("catalog.json").exists());
    drop(held);assert!(done_rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap());worker.join().unwrap();
}
