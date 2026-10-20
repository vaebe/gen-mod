use super::models::InspectionReport;
use crate::storage::repository::{decode, encode};
use rusqlite::{params, Connection};
pub fn save_on(db: &Connection, report: &InspectionReport, directory: &str) -> Result<(), String> {
    db.execute(
        "INSERT INTO inspection_reports VALUES(?1,?2,?3,?4)",
        params![report.report_id, report.mod_id, directory, encode(report)?],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn read_on(
    db: &Connection,
    id: &str,
    report_id: &str,
) -> Result<(InspectionReport, String), String> {
    let (payload, directory): (String, String) = db
        .query_row(
            "SELECT payload,directory FROM inspection_reports WHERE mod_id=?1 AND report_id=?2",
            params![id, report_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "调查报告不存在或不属于项目".to_string())?;
    Ok((decode(payload)?, directory))
}
pub fn list_on(db: &Connection, id: &str) -> Result<Vec<InspectionReport>, String> {
    let mut q = db
        .prepare(
            "SELECT payload FROM inspection_reports WHERE mod_id=?1 ORDER BY rowid DESC LIMIT 20",
        )
        .map_err(|e| e.to_string())?;
    let rows = q
        .query_map([id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.map(|r| decode(r.map_err(|e| e.to_string())?))
        .collect()
}
