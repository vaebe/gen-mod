use crate::storage::{database, repository::encode};
use rusqlite::params;

/// 文件操作日志与业务提交分开持久，部署复制不能被数据库回滚冒充已回退。
pub fn file_operation(app: &tauri::AppHandle, id: &str, mod_id: &str, kind: &str, status: &str, payload: &serde_json::Value) -> Result<(),String> {
    database::connect(app)?.execute("INSERT INTO file_operations VALUES(?1,?2,?3,?4,?5) ON CONFLICT(operation_id) DO UPDATE SET status=excluded.status,payload=excluded.payload",params![id,mod_id,kind,status,encode(payload)?]).map_err(|e|e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn list_recovery_issues(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>,String> {
    let db=database::connect(&app)?;
    let mut query=db.prepare("SELECT mod_id,kind,payload FROM file_operations WHERE status='requires_review'").map_err(|e|e.to_string())?;
    let rows=query.query_map([],|r|Ok(serde_json::json!({"modId":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"detail":r.get::<_,String>(2)?}))).map_err(|e|e.to_string())?;
    rows.map(|r|r.map_err(|e|e.to_string())).collect()
}
