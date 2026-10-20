pub mod repository;
pub mod recovery;
use crate::{storage::{self,models::ModProject,mods::StorageState},pi::workspace::{Workspaces,NativeOperation}};
use tauri::Manager;

#[tauri::command]
pub fn list_archived_mods(app: tauri::AppHandle) -> Result<Vec<ModProject>,String> {
    let db=storage::database::connect(&app)?;
    let mut statement=db.prepare("SELECT mod_id FROM projects WHERE archived=1 ORDER BY json_extract(payload,'$.updatedAt') DESC").map_err(|e|e.to_string())?;
    let rows=statement.query_map([],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;
    rows.map(|row|storage::repository::read_on(&db,&row.map_err(|e|e.to_string())?)).collect()
}
#[tauri::command]
pub async fn set_mod_archived(app: tauri::AppHandle,mod_id:String,expected_revision:i64,archived:bool) -> Result<ModProject,String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _reservation=app.state::<Workspaces>().try_reserve_native(&mod_id,NativeOperation::ProjectLifecycle)?;
        repository::ensure_idle_on(&storage::database::connect(&app)?,&mod_id)?;
        crate::pi::bridge::close_idle_project(&app,&mod_id)?;
        let state=app.state::<StorageState>();
        let _lock=state.0.try_lock().map_err(|_| "项目正在进行文件或版本操作，请完成后重试")?;
        repository::set_archived_on(&mut storage::database::connect(&app)?,&mod_id,expected_revision,archived)
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn delete_mod(app:tauri::AppHandle,mod_id:String,expected_revision:i64)->Result<recovery::DeleteProjectResult,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let reservation=app.state::<Workspaces>().try_reserve_native(&mod_id,NativeOperation::ProjectLifecycle)?;
        repository::ensure_idle_on(&storage::database::connect(&app)?,&mod_id)?;
        crate::pi::bridge::close_idle_project(&app,&mod_id)?;
        let root=storage::paths::mods_root(&app)?;
        let removal={
            let state=app.state::<StorageState>();
            let _lock=state.0.try_lock().map_err(|_|"项目正在进行文件或版本操作，请完成后重试")?;
            recovery::begin_delete(&mut storage::database::connect(&app)?,&root,&mod_id,expected_revision)?
        };
        let mut result=match storage::database::connect(&app){
            Ok(db)=>recovery::finish_delete(&db,&root,&removal),
            Err(error)=>recovery::DeleteProjectResult{mod_id:mod_id.clone(),removed:true,cleanup_pending:true,message:Some(format!("项目已移除，文件清理尚未开始：{error}"))},
        };
        if let Err(error)=crate::pi::bridge::forget_project(&app,&mod_id){result.message=Some(format!("{}；会话缓存清理失败：{error}",result.message.unwrap_or_else(||"项目已移除".into())));}
        // 预留保护到逻辑删除完成；丢弃缓存前释放令牌，迟到请求仍由数据库墓碑拒绝。
        drop(reservation);
        if let Err(error)=app.state::<Workspaces>().forget_project(&mod_id){result.message=Some(format!("{}；工作区缓存清理失败：{error}",result.message.unwrap_or_else(||"项目已移除".into())));}
        Ok(result)
    }).await.map_err(|e|e.to_string())?
}
#[cfg(test)]
mod tests;
