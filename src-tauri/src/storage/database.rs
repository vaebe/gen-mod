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
    initialize_schema(&mut connection)?;
    let mode: String = connection.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if mode.to_lowercase() != "wal" { return Err("SQLite 未能启用 WAL".into()); }
    app.manage(Database(path));
    Ok(())
}

// 仅初始化当前结构；非空旧库不迁移，也不自动清空。
fn initialize_schema(connection: &mut Connection) -> Result<(), String> {
    let tx = connection.transaction().map_err(|e| e.to_string())?;
    let version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0)).map_err(|e| e.to_string())?;
    let legacy: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='schema_migrations')", [], |row| row.get(0)).map_err(|e| e.to_string())?;
    if legacy { return Err("不支持旧开发数据库，请使用独立数据目录；原数据未迁移或删除".into()); }
    match version {
        0 => {
            let nonempty: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name NOT GLOB 'sqlite_*')", [], |row| row.get(0)).map_err(|e| e.to_string())?;
            if nonempty { return Err("不支持旧开发数据库或未知结构；原数据未迁移或删除".into()); }
            tx.execute_batch(include_str!("schema.sql")).map_err(|e| e.to_string())?;
            tx.pragma_update(None, "user_version", 1).map_err(|e| e.to_string())?;
        }
        1 => {},
        _ => return Err(format!("不支持数据库结构版本：{version}")),
    }
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_schema_initializes_once_and_preserves_rows() {
        let mut db = Connection::open_in_memory().unwrap();
        initialize_schema(&mut db).unwrap();
        db.execute("INSERT INTO projects(mod_id,payload) VALUES('p','{}')", []).unwrap();
        initialize_schema(&mut db).unwrap();
        assert_eq!(db.query_row::<i64, _, _>("SELECT COUNT(*) FROM projects", [], |row| row.get(0)).unwrap(), 1);
        assert_eq!(db.pragma_query_value::<i64, _>(None, "user_version", |row| row.get(0)).unwrap(), 1);
        assert_eq!(db.query_row::<i64, _, _>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT GLOB 'sqlite_*'", [], |row| row.get(0)).unwrap(), 18);
        assert_eq!(db.query_row::<i64, _, _>("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('legacy_imports','project_assets','context_proposals','schema_migrations')", [], |row| row.get(0)).unwrap(), 0);
    }

    #[test]
    fn unsupported_databases_are_rejected_without_changes() {
        for (schema, version) in [
            ("CREATE TABLE schema_migrations(version INTEGER); INSERT INTO schema_migrations VALUES(12);", 0),
            ("CREATE TABLE unknown(value TEXT); INSERT INTO unknown VALUES('keep');", 0),
            ("CREATE TABLE unknown(value TEXT); INSERT INTO unknown VALUES('keep');", 2),
        ] {
            let mut db = Connection::open_in_memory().unwrap();
            db.execute_batch(schema).unwrap();
            db.pragma_update(None, "user_version", version).unwrap();
            let before: String = db.query_row("SELECT group_concat(sql) FROM sqlite_master", [], |row| row.get(0)).unwrap();
            let changes = db.total_changes();
            assert!(initialize_schema(&mut db).is_err());
            assert_eq!(db.query_row::<String, _, _>("SELECT group_concat(sql) FROM sqlite_master", [], |row| row.get(0)).unwrap(), before);
            assert_eq!(db.total_changes(), changes);
            assert_eq!(db.pragma_query_value::<i64, _>(None, "user_version", |row| row.get(0)).unwrap(), version);
        }
    }
}
