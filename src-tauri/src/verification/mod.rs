pub mod evidence;
pub mod models;
pub mod repository;
#[cfg(test)]
mod tests;
use crate::storage::database;
use models::*;
use tauri::Manager;
#[tauri::command]
pub fn list_verification(
    app: tauri::AppHandle,
    mod_id: String,
    version_id: Option<String>,
) -> Result<VerificationList, String> {
    crate::storage::repository::read(&app, &mod_id)?;
    repository::list_on(&database::connect(&app)?, &mod_id, version_id.as_deref())
}
#[tauri::command]
pub fn edit_feedback_draft(
    app: tauri::AppHandle,
    mod_id: String,
    draft_id: String,
    expected_revision: i64,
    payload: FeedbackPayload,
) -> Result<FeedbackDraft, String> {
    let state=app.state::<crate::storage::mods::StorageState>();
    let _lock=state.0.lock().map_err(|_|"项目存储锁异常")?;
    crate::project_lifecycle::repository::ensure_writable(&app,&mod_id)?;
    let evidence = evidence::validate(&app, &mod_id, &payload)?;
    repository::edit_on(
        &database::connect(&app)?,
        &mod_id,
        &draft_id,
        expected_revision,
        payload,
        evidence,
    )
}
#[tauri::command]
pub fn confirm_feedback_draft(
    app: tauri::AppHandle,
    mod_id: String,
    draft_id: String,
    expected_revision: i64,
    supersedes: Option<String>,
) -> Result<VerificationRecord, String> {
    let state=app.state::<crate::storage::mods::StorageState>();
    let _lock=state.0.lock().map_err(|_|"项目存储锁异常")?;
    crate::project_lifecycle::repository::ensure_writable(&app,&mod_id)?;
    crate::storage::repository::read(&app, &mod_id)?;
    repository::confirm_on(
        &mut database::connect(&app)?,
        &mod_id,
        &draft_id,
        expected_revision,
        supersedes,
    )
}
#[tauri::command]
pub fn reject_feedback_draft(
    app: tauri::AppHandle,
    mod_id: String,
    draft_id: String,
    expected_revision: i64,
) -> Result<(), String> {
    let state=app.state::<crate::storage::mods::StorageState>();
    let _lock=state.0.lock().map_err(|_|"项目存储锁异常")?;
    crate::project_lifecycle::repository::ensure_writable(&app,&mod_id)?;
    crate::storage::repository::read(&app, &mod_id)?;
    repository::reject_on(
        &database::connect(&app)?,
        &mod_id,
        &draft_id,
        expected_revision,
    )
}
