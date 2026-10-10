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
    app.manage(Database(path));
    super::migration::import(app)?;
    super::repository::interrupt_previous(app)?;
    super::repository::reconcile_files(app)
}
