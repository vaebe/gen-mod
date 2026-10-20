use super::models::ProjectAsset;
use crate::storage::repository::{decode, encode};
use rusqlite::{params, Connection};
pub fn save_on(db: &Connection, value: &ProjectAsset) -> Result<(), String> {
    db.execute(
        "INSERT INTO project_assets VALUES(?1,?2,?3) ON CONFLICT(asset_id) DO NOTHING",
        params![value.asset_id, value.mod_id, encode(value)?],
    )
    .map_err(|e| e.to_string())?;
    let existing = read_on(db, &value.mod_id, &value.asset_id)?;
    if existing.sha256 != value.sha256 || existing.relative_path != value.relative_path {
        return Err("素材 ID 已对应其他内容".into());
    }
    Ok(())
}
pub fn read_on(db: &Connection, id: &str, asset: &str) -> Result<ProjectAsset, String> {
    crate::storage::paths::validate_id(asset)?;
    decode(
        db.query_row(
            "SELECT payload FROM project_assets WHERE mod_id=?1 AND asset_id=?2",
            params![id, asset],
            |r| r.get(0),
        )
        .map_err(|_| "素材不存在或不属于项目".to_string())?,
    )
}
pub fn list_on(db: &Connection, id: &str) -> Result<Vec<ProjectAsset>, String> {
    let mut q = db
        .prepare("SELECT payload FROM project_assets WHERE mod_id=?1 ORDER BY rowid DESC")
        .map_err(|e| e.to_string())?;
    let rows = q
        .query_map([id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.map(|r| decode(r.map_err(|e| e.to_string())?))
        .collect()
}
