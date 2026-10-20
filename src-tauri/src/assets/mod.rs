// 仅兼容旧素材元数据与未完成操作；新导入走全部文件，不再解码或处理图片。
pub mod models;
pub mod repository;
#[cfg(test)]
mod tests;
use crate::storage::{database, paths, repository as storage};
use models::ProjectAsset;
use std::path::PathBuf;

pub fn asset_path(app: &tauri::AppHandle, asset: &ProjectAsset) -> Result<PathBuf, String> {
    asset_path_on(&paths::project_dir(app,&asset.mod_id)?,asset)
}
fn asset_path_on(root:&std::path::Path,asset:&ProjectAsset)->Result<PathBuf,String>{
    paths::validate_id(&asset.asset_id)?;
    let extension = if asset.media_type == "image/png" {
        "png"
    } else if asset.media_type == "image/jpeg" {
        "jpg"
    } else {
        return Err("素材类型无效".into());
    };
    let expected = format!(
        "assets/{}/{}.{}",
        if asset.input_asset_id.is_some() {
            "derived"
        } else {
            "originals"
        },
        asset.asset_id,
        extension
    );
    if asset.relative_path != expected {
        return Err("素材路径与身份不一致".into());
    }
    crate::artifacts::registry::checked_path(
        root,
        &asset.relative_path,
    )
}

#[tauri::command]
pub fn list_legacy_assets(app:tauri::AppHandle,mod_id:String)->Result<Vec<serde_json::Value>,String>{
    storage::read(&app,&mod_id)?;
    let source=paths::project_dir(&app,&mod_id)?.join("source");
    repository::list_on(&database::connect(&app)?,&mod_id)?.into_iter().map(|asset|{
        let relative=format!("assets/legacy/{}.{}",asset.asset_id,if asset.media_type=="image/png"{"png"}else{"jpg"});
        let restored=crate::artifacts::registry::checked_path(&source,&relative).and_then(|path|crate::changes::file_hash(&path)).is_ok_and(|hash|hash==asset.sha256);
        let mut value=serde_json::to_value(asset).map_err(|e|e.to_string())?;
        value["restored"]=restored.into();value["sourcePath"]=relative.into();Ok(value)
    }).collect()
}
#[tauri::command]
pub async fn restore_legacy_asset(app:tauri::AppHandle,mod_id:String,asset_id:String)->Result<serde_json::Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let asset=repository::read_on(&database::connect(&app)?,&mod_id,&asset_id)?;
        if asset.mod_id!=mod_id{return Err("旧素材归属不一致".into());}
        let original=asset_path(&app,&asset)?;
        let extension=if asset.media_type=="image/png"{"png"}else{"jpg"};
        let mut result=crate::source_files::restore_legacy(&app,&mod_id,&original,&format!("{asset_id}.{extension}"),&asset.sha256)?;
        result["assetId"]=asset_id.into();Ok(result)
    }).await.map_err(|e|e.to_string())?
}
pub fn reconcile(
    app: &tauri::AppHandle,
    id: &str,
    operation: &str,
    kind: &str,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let Some(raw) = payload.get("asset") else {
        return storage::file_operation(app, operation, id, kind, "interrupted", payload);
    };
    let asset: ProjectAsset = serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?;
    if asset.mod_id != id {
        return Err("素材恢复归属不一致".into());
    }
    let valid = asset_path(app, &asset).and_then(|path|crate::changes::file_hash(&path)).is_ok_and(|hash|hash==asset.sha256);
    let status=reconcile_metadata_on(&database::connect(app)?,id,&asset,payload["taskId"].as_str(),valid)?;
    storage::file_operation(app,operation,id,kind,status,payload)
}
pub(crate) fn reconcile_metadata_on(db:&rusqlite::Connection,id:&str,asset:&ProjectAsset,task:Option<&str>,valid:bool)->Result<&'static str,String>{
    if asset.mod_id!=id{return Err("旧素材归属不一致".into());}
    if task.is_some() {
        let existing:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM project_assets WHERE mod_id=?1 AND asset_id=?2)",rusqlite::params![id,asset.asset_id],|row|row.get(0)).map_err(|e|e.to_string())?;
        if !existing{return Ok("interrupted");}
    }
    if !valid{return Ok("requires_review");}
    repository::save_on(db,asset)?;
    Ok("reconciled")
}
