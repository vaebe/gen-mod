use super::content_check::ContentCheckReport;
use crate::storage::{
    models::ArtifactVersion,
    repository::{decode, encode},
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerificationCase {
    pub id: String,
    pub precondition: String,
    pub action: String,
    pub expected: String,
    pub evidence_hint: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryMetadata {
    pub mod_id: String,
    pub version_id: String,
    pub task_id: String,
    pub context_snapshot_id: Option<String>,
    pub created_at: String,
    pub artifact_deleted: bool,
    pub cases: Vec<VerificationCase>,
    pub check_report: Option<ContentCheckReport>,
}
pub fn read_on(db: &Connection, id: &str, version: &str) -> Result<DeliveryMetadata, String> {
    decode(
        db.query_row(
            "SELECT payload FROM delivery_metadata WHERE mod_id=?1 AND version_id=?2",
            params![id, version],
            |r| r.get(0),
        )
        .map_err(|_| "版本身份不存在或不属于项目".to_string())?,
    )
}
pub fn ensure_anchor_on(
    db: &Connection,
    id: &str,
    version: &ArtifactVersion,
) -> Result<(), String> {
    let value = DeliveryMetadata {
        mod_id: id.into(),
        version_id: version.version_id.clone(),
        task_id: version.task_id.clone(),
        context_snapshot_id: None,
        created_at: version.created_at.clone(),
        artifact_deleted: false,
        cases: vec![],
        check_report: None,
    };
    db.execute("INSERT INTO delivery_metadata VALUES(?1,?2,?3,?4) ON CONFLICT(mod_id,version_id) DO NOTHING",params![id,version.version_id,version.task_id,encode(&value)?]).map_err(|e|e.to_string())?;
    Ok(())
}
pub fn save_on(db: &Connection, value: &DeliveryMetadata) -> Result<(), String> {
    if db
        .execute(
            "UPDATE delivery_metadata SET payload=?3 WHERE mod_id=?1 AND version_id=?2",
            params![value.mod_id, value.version_id, encode(value)?],
        )
        .map_err(|e| e.to_string())?
        != 1
    {
        return Err("版本锚点不存在".into());
    }
    Ok(())
}
pub fn mark_deleted_on(
    db: &Connection,
    id: &str,
    version: &str,
    deleted: bool,
) -> Result<(), String> {
    let mut value = read_on(db, id, version)?;
    value.artifact_deleted = deleted;
    save_on(db, &value)
}
pub fn list_on(db: &Connection, id: &str) -> Result<Vec<DeliveryMetadata>, String> {
    let mut q = db
        .prepare("SELECT payload FROM delivery_metadata WHERE mod_id=?1 ORDER BY rowid DESC")
        .map_err(|e| e.to_string())?;
    let rows = q
        .query_map([id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.map(|r| decode(r.map_err(|e| e.to_string())?))
        .collect()
}
pub fn backfill(app: &tauri::AppHandle) -> Result<(), String> {
    let mut db = crate::storage::database::connect(app)?;
    let projects = crate::storage::repository::list(app)?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    for p in projects {
        for v in p.versions {
            ensure_anchor_on(&tx, &p.mod_id, &v)?;
        }
    }
    tx.commit().map_err(|e| e.to_string())
}
pub fn validate_cases(cases: &[VerificationCase]) -> Result<(), String> {
    if cases.len() > 100 {
        return Err("验收条目过多".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for c in cases {
        if c.id.trim().is_empty()
            || c.id.len() > 120
            || !ids.insert(&c.id)
            || c.expected.trim().is_empty()
            || c.action.trim().is_empty()
            || [&c.precondition, &c.action, &c.expected, &c.evidence_hint]
                .iter()
                .any(|s| s.len() > 4000)
        {
            return Err("验收步骤无效、重复或过长".into());
        }
    }
    Ok(())
}
#[tauri::command]
pub fn get_delivery_metadata(
    app: tauri::AppHandle,
    mod_id: String,
    version_id: Option<String>,
) -> Result<Vec<DeliveryMetadata>, String> {
    crate::storage::repository::read(&app, &mod_id)?;
    let db = crate::storage::database::connect(&app)?;
    match version_id {
        Some(v) => Ok(vec![read_on(&db, &mod_id, &v)?]),
        None => list_on(&db, &mod_id),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anchors_are_idempotent_and_survive_package_deletion() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE projects(mod_id TEXT PRIMARY KEY);INSERT INTO projects VALUES('a'),('b');").unwrap();
        db.execute_batch(include_str!(
            "../storage/migrations/004_delivery_metadata.sql"
        ))
        .unwrap();
        let version = ArtifactVersion {
            version_id: "v1".into(),
            task_id: "t1".into(),
            created_at: "now".into(),
            skills: vec![],
            files: vec![],
            zip_sha256: "hash".into(),
        };
        ensure_anchor_on(&db, "a", &version).unwrap();
        mark_deleted_on(&db, "a", "v1", true).unwrap();
        ensure_anchor_on(&db, "a", &version).unwrap();
        assert!(read_on(&db, "a", "v1").unwrap().artifact_deleted);
        assert!(read_on(&db, "a", "v1").unwrap().cases.is_empty());
        assert!(read_on(&db, "b", "v1").is_err());
    }
}
