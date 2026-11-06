use super::paths;
use rusqlite::Connection;
use std::path::PathBuf;
use tauri::Manager;

/// 只保存数据库位置；每次仓储操作使用独立短连接，不跨文件或进程操作持有事务。
pub struct Database(pub PathBuf);

pub fn connect(app: &tauri::AppHandle) -> Result<Connection, String> {
    open(&app.state::<Database>().0)
}
fn open(path: &std::path::Path) -> Result<Connection, String> {
    if path.exists() { paths::reject_redirect(path)?; }
    let connection = Connection::open(path).map_err(|e| e.to_string())?;
    connection.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| e.to_string())?;
    connection.pragma_update(None, "foreign_keys", true).map_err(|e| e.to_string())?;
    Ok(connection)
}

/// 在 IPC 开放前初始化；只在实际启动应用时访问 local data。
pub fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    let root = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    paths::reject_redirect(&root)?;
    let path = root.join("gen-mod.sqlite3");
    let mut connection = open(&path)?;
    if rusqlite::version_number() < 3_051_003 { return Err("内置 SQLite 缺少 WAL 修复，不能启用元数据写入".into()); }
    let mode: String = connection.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if mode.to_lowercase() != "wal" { return Err("SQLite 未能启用 WAL".into()); }
    let exists: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='schema_migrations')", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if !exists {
        let tx = connection.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(include_str!("migrations/001_initial.sql")).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO schema_migrations VALUES(1,?1)", [super::mods::now()?]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    let model_migration: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=2)", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if !model_migration {
        let tx = connection.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(include_str!("migrations/002_session_model.sql")).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO schema_migrations VALUES(2,?1)", [super::mods::now()?]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    for (version,sql) in [(3, include_str!("migrations/003_project_context.sql")),(4,include_str!("migrations/004_delivery_metadata.sql")),(5,include_str!("migrations/005_verification.sql")),(6,include_str!("migrations/006_knowledge.sql")),(7,include_str!("migrations/007_inspection.sql")),(8,include_str!("migrations/008_assets.sql")),(9,include_str!("migrations/009_memory.sql")),(10,include_str!("migrations/010_project_lifecycle.sql")),(11,include_str!("migrations/011_submission_requests.sql")),(12,include_str!("migrations/012_submission_messages.sql"))] {
        let applied:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=?1)",[version],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !applied {
            let tx=connection.transaction().map_err(|e|e.to_string())?;
            if version==9 { crate::knowledge::repository::migrate_memory_on(&tx)?; }
            else { tx.execute_batch(sql).map_err(|e|e.to_string())?; }
            tx.execute("INSERT INTO schema_migrations VALUES(?1,?2)",rusqlite::params![version,super::mods::now()?]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())?;
        }
    }
    app.manage(Database(path));
    crate::project_lifecycle::recovery::recover_pending(app)?;
    super::migration::import(app)?;
    crate::project_context::repository::backfill(app)?;
    crate::artifacts::delivery_metadata::backfill(app)?;
    super::repository::interrupt_previous(app)?;
    connect(app)?.execute("UPDATE submission_requests SET status='rejected',error='应用已重启，提交未启动' WHERE status='pending' AND task_id IS NULL",[]).map_err(|e|e.to_string())?;
    super::repository::reconcile_files(app)
}

#[cfg(test)]
mod tests {
    #[test]
    fn memory_migration_backfill_failure_restores_original_schema_and_rows() {
        let mut db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("migrations/001_initial.sql")).unwrap();
        db.execute_batch(include_str!("migrations/006_knowledge.sql")).unwrap();
        db.execute("INSERT INTO projects VALUES('p','{}',0)", []).unwrap();
        db.execute("INSERT INTO knowledge_notes VALUES('n','p','duckov',1,'active','{}')", []).unwrap();
        {
            let tx = db.transaction().unwrap();
            assert!(crate::knowledge::repository::migrate_memory_on(&tx).is_err());
        }
        let (owner, count): (String, i64) = db.query_row("SELECT mod_id,COUNT(*) FROM knowledge_notes", [], |row| Ok((row.get(0)?,row.get(1)?))).unwrap();
        assert_eq!(owner, "p");
        assert_eq!(count, 1);
        let extra_tables: i64 = db.query_row("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('knowledge_notes_next','memory_content_links','task_memory_uses')", [], |row| row.get(0)).unwrap();
        assert_eq!(extra_tables, 0);
        let applied: i64 = db.query_row("SELECT COUNT(*) FROM schema_migrations WHERE version=9", [], |row| row.get(0)).unwrap();
        assert_eq!(applied, 0);
    }
}
