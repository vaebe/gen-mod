use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};

pub(super) fn register(app: &tauri::AppHandle, mod_id: &str, session_id: &str, request_id: &str) -> Result<(), String> {
    crate::storage::paths::validate_id(request_id)?;
    let db = crate::storage::database::connect(app)?;
    let inserted = db.execute("INSERT OR IGNORE INTO submission_requests(request_id,mod_id,session_id,status) VALUES(?1,?2,?3,'pending')", params![request_id,mod_id,session_id]).map_err(|e| e.to_string())?;
    if inserted != 1 { return Err("该提交已经登记，请核对原提交结果".into()); }
    Ok(())
}
pub(super) fn bind_on(db: &rusqlite::Connection, request_id: &str, task_id: &str) -> Result<(), String> {
    db.execute("UPDATE submission_requests SET task_id=?2,status='accepted' WHERE request_id=?1",params![request_id,task_id]).map_err(|e|e.to_string())?;
    Ok(())
}
pub(super) fn reject(app: &tauri::AppHandle, request_id: &str, error: &str) -> Result<(), String> {
    crate::storage::database::connect(app)?.execute("UPDATE submission_requests SET status=CASE WHEN task_id IS NULL THEN 'rejected' ELSE status END,error=?2 WHERE request_id=?1",params![request_id,error]).map_err(|e|e.to_string())?;
    Ok(())
}

// 保存正式用户消息的身份，而不是用提交时间或正文猜测所属请求。
pub(super) fn confirm_message(app: &tauri::AppHandle, request_id: &str, message: &Value) -> Result<(), String> {
    let timestamp = message["timestamp"].as_i64().ok_or("正式用户消息缺少时间戳")?;
    let fingerprint = super::bridge::input_fingerprint(message, None)?;
    crate::storage::database::connect(app)?.execute(
        "UPDATE submission_requests SET message_timestamp=?2,message_fingerprint=?3 WHERE request_id=?1 AND message_timestamp IS NULL",
        params![request_id, timestamp, fingerprint],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn annotate_history(db: &rusqlite::Connection, mod_id: &str, session_id: &str, history: &mut Value) -> Result<(), String> {
    let mut query = db.prepare("SELECT request_id,task_id,message_timestamp,message_fingerprint FROM submission_requests WHERE mod_id=?1 AND session_id=?2 AND message_timestamp IS NOT NULL").map_err(|e| e.to_string())?;
    let rows = query.query_map(params![mod_id, session_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, String>(3)?)))
        .map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    for message in history["messages"].as_array_mut().into_iter().flatten() {
        if message["role"] != "user" { continue; }
        let fingerprint = super::bridge::input_fingerprint(message, None)?;
        let mut matched = rows.iter().filter(|(_, _, timestamp, hash)| message["timestamp"].as_i64() == Some(*timestamp) && *hash == fingerprint);
        if let Some((request_id, task_id, _, _)) = matched.next() {
            if matched.next().is_none() {
                message["appInputId"] = request_id.clone().into();
                message["appTaskId"] = task_id.clone().into();
            }
        }
    }
    Ok(())
}
#[tauri::command]
pub fn read_submission_result(app: tauri::AppHandle, mod_id: String, session_id: String, request_id: String) -> Result<Value,String> {
    let db=crate::storage::database::connect(&app)?;
    let row: Option<(String,Option<String>,Option<String>)> = db.query_row("SELECT status,task_id,error FROM submission_requests WHERE request_id=?1 AND mod_id=?2 AND session_id=?3",params![request_id,mod_id,session_id],|r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
    let Some((status,task_id,error))=row else {return Ok(json!({"status":"unknown"}));};
    let task:Option<String>=match task_id {Some(id)=>db.query_row("SELECT payload FROM tasks WHERE task_id=?1",[id],|r|r.get(0)).optional().map_err(|e|e.to_string())?,None=>None};
    Ok(json!({"status":status,"error":error,"task":task.map(|v|serde_json::from_str::<Value>(&v)).transpose().map_err(|e|e.to_string())?}))
}

#[cfg(test)]
mod tests {
    #[test]
    fn history_recovers_only_the_confirmed_message_in_its_session() {
        use serde_json::json;
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db.execute_batch(include_str!("../storage/migrations/001_initial.sql")).unwrap();
        db.execute_batch(include_str!("../storage/migrations/011_submission_requests.sql")).unwrap();
        db.execute("INSERT INTO projects VALUES('p','{}',0)", []).unwrap();
        db.execute("INSERT INTO sessions(mod_id,session_id,payload) VALUES('p','s','{}')", []).unwrap();
        db.execute("INSERT INTO submission_requests VALUES('r','p','s','accepted','task',NULL)", []).unwrap();
        db.execute_batch(include_str!("../storage/migrations/012_submission_messages.sql")).unwrap();
        let message = json!({"role":"user","timestamp":100,"content":"继续"});
        let fingerprint = crate::pi::bridge::input_fingerprint(&message, None).unwrap();
        db.execute("UPDATE submission_requests SET message_timestamp=100,message_fingerprint=?1", [&fingerprint]).unwrap();
        let mut history = json!({"messages":[message,{"role":"user","timestamp":101,"content":"继续"},{"role":"user","timestamp":100,"content":"其他内容"}]});
        super::annotate_history(&db, "p", "other-session", &mut history).unwrap();
        assert!(history["messages"][0]["appInputId"].is_null());
        super::annotate_history(&db, "p", "s", &mut history).unwrap();
        assert_eq!(history["messages"][0]["appInputId"], "r");
        assert_eq!(history["messages"][0]["appTaskId"], "task");
        assert!(history["messages"][1]["appInputId"].is_null());
        assert!(history["messages"][2]["appInputId"].is_null());
    }

    #[test]
    fn removing_session_cascades_accepted_and_rejected_submissions() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db.execute_batch(include_str!("../storage/migrations/001_initial.sql")).unwrap();
        db.execute_batch(include_str!("../storage/migrations/011_submission_requests.sql")).unwrap();
        db.execute("INSERT INTO projects VALUES('p','{}',0)", []).unwrap();
        db.execute("INSERT INTO sessions(mod_id,session_id,payload) VALUES('p','s','{}')", []).unwrap();
        db.execute("INSERT INTO submission_requests VALUES('a','p','s','accepted','t',NULL),('b','p','s','rejected',NULL,'busy')", []).unwrap();
        db.execute("DELETE FROM sessions WHERE mod_id='p'", []).unwrap();
        let count: i64 = db.query_row("SELECT COUNT(*) FROM submission_requests", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);
    }
}
