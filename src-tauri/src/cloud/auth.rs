use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}, time::Duration};
use tauri::Manager;
pub const INVALID:&str="CLOUD_AUTH_INVALID:";
#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Tokens { pub access_token:String,pub refresh_token:String,pub email:String,pub role:String }
#[derive(Clone,Deserialize,Serialize)]
pub struct User {pub email:String,pub role:String}
#[derive(Default)] pub struct CloudState(pub tauri::async_runtime::Mutex<()>);
pub struct AuthClient {pub root:PathBuf,pub base_url:String,http:reqwest::Client}
pub fn load_tokens(root:&Path)->Result<Tokens,String> {
    match fs::read(root.join("cloud-auth.json")) {
        Ok(bytes)=>serde_json::from_slice(&bytes).map_err(|_|"账号配置无法解析，请重新登录".into()),
        Err(error) if error.kind()==std::io::ErrorKind::NotFound=>Err(format!("{INVALID}请先登录")),
        Err(error)=>Err(error.to_string()),
    }
}
impl AuthClient {
    pub fn new(root:PathBuf,base_url:String)->Result<Self,String> {
        let url=reqwest::Url::parse(&base_url).map_err(|_|"API 地址无效")?;
        if !matches!(url.scheme(),"http"|"https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some(){return Err("API 地址应为不含凭据的 HTTP(S) 地址".into());}
        Ok(Self{root,base_url:base_url.trim_end_matches('/').into(),http:reqwest::Client::builder().timeout(Duration::from_secs(60)).redirect(reqwest::redirect::Policy::none()).build().map_err(|e|e.to_string())?})
    }
    pub fn save(&self,tokens:&Tokens)->Result<(),String> {fs::create_dir_all(&self.root).map_err(|e|e.to_string())?;crate::storage::mods::write_json(&self.root.join("cloud-auth.json"),tokens)}
    async fn decode<T:serde::de::DeserializeOwned>(response:reqwest::Response)->Result<T,String>{
        let status=response.status();let bytes=response.bytes().await.map_err(|e|e.to_string())?;
        if !status.is_success(){let error=serde_json::from_slice::<serde_json::Value>(&bytes).ok().and_then(|value|value.get("error").and_then(|v|v.as_str()).map(str::to_owned)).unwrap_or_else(||format!("服务请求失败（{status}）"));return Err(error);}
        serde_json::from_slice(&bytes).map_err(|_|"服务响应格式无效".into())
    }
    pub async fn login(&self,email:&str,password:&str,register:bool)->Result<User,String>{
        let body=serde_json::json!({"email":email,"password":password});
        if register {Self::decode::<serde_json::Value>(self.http.post(format!("{}/auth/register",self.base_url)).json(&body).send().await.map_err(|e|e.to_string())?).await?;}
        let tokens:Tokens=Self::decode(self.http.post(format!("{}/auth/login",self.base_url)).json(&body).send().await.map_err(|e|e.to_string())?).await?;
        if tokens.access_token.is_empty() || tokens.refresh_token.is_empty(){return Err("服务未返回有效令牌".into());}
        self.save(&tokens)?;Ok(User{email:tokens.email,role:tokens.role})
    }
    pub async fn get(&self,path:&str)->Result<Vec<u8>,String>{
        let mut tokens=load_tokens(&self.root)?;
        let mut response=self.http.get(format!("{}{path}",self.base_url)).bearer_auth(&tokens.access_token).send().await.map_err(|e|e.to_string())?;
        if response.status()==reqwest::StatusCode::UNAUTHORIZED {
            let refreshed=self.http.post(format!("{}/auth/refresh",self.base_url)).json(&serde_json::json!({"refreshToken":tokens.refresh_token})).send().await.map_err(|e|e.to_string())?;
            if refreshed.status()==reqwest::StatusCode::UNAUTHORIZED {self.clear()?;return Err(format!("{INVALID}登录凭证无效，请重新登录"));}
            tokens=Self::decode(refreshed).await?;self.save(&tokens)?;
            response=self.http.get(format!("{}{path}",self.base_url)).bearer_auth(&tokens.access_token).send().await.map_err(|e|e.to_string())?;
            if response.status()==reqwest::StatusCode::UNAUTHORIZED {self.clear()?;return Err(format!("{INVALID}登录凭证无效，请重新登录"));}
        }
        if !response.status().is_success(){return Err(format!("服务请求失败（{}）",response.status()));}
        Ok(response.bytes().await.map_err(|e|e.to_string())?.to_vec())
    }
    pub fn clear(&self)->Result<(),String>{match fs::remove_file(self.root.join("cloud-auth.json")){Ok(())=>Ok(()),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),Err(e)=>Err(e.to_string())}}
}
pub fn client(app:&tauri::AppHandle)->Result<AuthClient,String>{AuthClient::new(crate::config::directory(app)?,crate::config::cloud_api_url(app)?)}
pub fn require_local(app:&tauri::AppHandle)->Result<(),String>{load_tokens(&crate::config::directory(app)?).map(|_|())}
#[tauri::command]
pub async fn cloud_initialize(app: tauri::AppHandle, api_url: String) -> Result<(), String> {
    let state = app.state::<CloudState>();
    let _guard = state.0.lock().await;
    let client = AuthClient::new(crate::config::directory(&app)?, api_url.trim().into())?;
    if crate::config::cloud_api_url(&app)?.trim_end_matches('/') != client.base_url {
        // 切换 env 对应的服务时，旧令牌不能发送到新的 API。
        client.clear()?;
        crate::config::save_cloud_api_url(&app, &client.base_url)?;
    }
    Ok(())
}
#[tauri::command]
pub async fn cloud_login(app:tauri::AppHandle,email:String,password:String)->Result<User,String>{
    let state=app.state::<CloudState>();let _guard=state.0.lock().await;
    client(&app)?.login(&email,&password,false).await
}
#[tauri::command]
pub async fn cloud_register(app:tauri::AppHandle,email:String,password:String)->Result<User,String>{
    let state=app.state::<CloudState>();let _guard=state.0.lock().await;
    client(&app)?.login(&email,&password,true).await
}
#[tauri::command]
pub async fn cloud_me(app:tauri::AppHandle)->Result<User,String>{
    let state=app.state::<CloudState>();let _guard=state.0.lock().await;
    serde_json::from_slice(&client(&app)?.get("/auth/me").await?).map_err(|_|"账号响应无效".into())
}
#[tauri::command]
pub async fn cloud_logout(app:tauri::AppHandle)->Result<(),String>{
    let state=app.state::<CloudState>();let _guard=state.0.lock().await;
    // 退出只清除此端凭证；不调用会影响浏览器 Cookie 的接口。
    client(&app)?.clear()
}
