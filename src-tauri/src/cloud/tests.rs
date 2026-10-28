use super::auth::*;
use std::{fs, io::{Read, Write}, net::TcpListener, thread};
pub(super) fn server(replies: Vec<(&'static str,u16,&'static str)>) -> String {
    let listener=TcpListener::bind("127.0.0.1:0").unwrap();let url=format!("http://{}",listener.local_addr().unwrap());
    thread::spawn(move || {for (path,status,body) in replies {
        let (mut socket,_)=listener.accept().unwrap();let mut request=vec![];let mut byte=[0;1];
        while !request.ends_with(b"\r\n\r\n") {socket.read_exact(&mut byte).unwrap();request.push(byte[0]);}
        let header=String::from_utf8_lossy(&request);assert!(header.lines().next().unwrap().contains(path));
        let length=header.lines().find_map(|line|line.to_lowercase().strip_prefix("content-length:").map(|value|value.trim().parse::<usize>().unwrap())).unwrap_or(0);
        let mut data=vec![0;length];socket.read_exact(&mut data).unwrap();
        write!(socket,"HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
    }});
    url
}
const TOKENS:&str=r#"{"accessToken":"access","refreshToken":"refresh","email":"a@b.c","role":"user"}"#;
#[test] fn missing_credentials_do_not_touch_projects() {
    let root=tempfile::tempdir().unwrap();fs::write(root.path().join("project.keep"),"keep").unwrap();
    let client=AuthClient::new(root.path().to_owned(),"http://127.0.0.1:1".into()).unwrap();
    assert!(tauri::async_runtime::block_on(client.get("/auth/me")).unwrap_err().starts_with(INVALID));
    assert_eq!(fs::read_to_string(root.path().join("project.keep")).unwrap(),"keep");
}
#[test] fn login_persists_tokens_in_config_directory() {
    let root=tempfile::tempdir().unwrap();let url=server(vec![("/auth/login",200,TOKENS)]);
    let client=AuthClient::new(root.path().to_owned(),url).unwrap();
    tauri::async_runtime::block_on(client.login("a@b.c","password",false)).unwrap();
    let saved:Tokens=serde_json::from_slice(&fs::read(root.path().join("cloud-auth.json")).unwrap()).unwrap();
    assert_eq!(saved.refresh_token,"refresh");assert!(!root.path().join("skills").exists());
}
#[test] fn expired_access_refreshes_and_retries() {
    let root=tempfile::tempdir().unwrap();let url=server(vec![("/auth/me",401,"{}"),("/auth/refresh",200,TOKENS),("/auth/me",200,r#"{"email":"a@b.c","role":"user"}"#)]);
    let client=AuthClient::new(root.path().to_owned(),url).unwrap();client.save(&serde_json::from_str(TOKENS).unwrap()).unwrap();
    assert!(tauri::async_runtime::block_on(client.get("/auth/me")).is_ok());
}
#[test] fn only_explicit_unauthorized_refresh_clears_credentials() {
    for status in [401,500] {
        let root=tempfile::tempdir().unwrap();let url=server(vec![("/auth/me",401,"{}"),("/auth/refresh",status,r#"{"error":"unavailable"}"#)]);
        let client=AuthClient::new(root.path().to_owned(),url).unwrap();client.save(&serde_json::from_str(TOKENS).unwrap()).unwrap();
        let error=tauri::async_runtime::block_on(client.get("/auth/me")).unwrap_err();
        assert_eq!(error.starts_with(INVALID),status==401);assert_eq!(root.path().join("cloud-auth.json").exists(),status!=401);
    }
    let root=tempfile::tempdir().unwrap();let client=AuthClient::new(root.path().to_owned(),"http://127.0.0.1:1".into()).unwrap();client.save(&serde_json::from_str(TOKENS).unwrap()).unwrap();
    assert!(!tauri::async_runtime::block_on(client.get("/auth/me")).unwrap_err().starts_with(INVALID));assert!(root.path().join("cloud-auth.json").exists());
}
