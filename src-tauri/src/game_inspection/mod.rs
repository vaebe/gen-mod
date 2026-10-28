pub mod models;
pub mod repository;
pub mod scanner;
#[cfg(test)]
mod tests;
use crate::storage::{database, repository as storage};
use tauri::Manager;
#[tauri::command]
pub async fn inspect_supported_game(
    app: tauri::AppHandle,
    mod_id: String,
    kind: String,
    directory: String,
) -> Result<models::InspectionReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let project = storage::read(&app, &mod_id)?;
        let views = crate::skills::registry::resolve_for_game(&app, &project.game_id)?;
        let mut reports=vec![];
        for view in views.iter().filter(|view|view.definition.category=="game") {
            let Some(file)=&view.definition.inspection_file else {continue};
            let path=crate::artifacts::registry::checked_path(std::path::Path::new(&view.definition.path),file)?;
            let rules:models::InspectionRule=serde_json::from_slice(&std::fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            scanner::validate_rules(&rules)?;
            if let Some(profile)=rules.profiles.iter().find(|profile|profile.kind==kind){reports.push(scanner::scan_directory(std::path::Path::new(&directory),profile)?);}
        }
        let mut report=reports.into_iter().reduce(|mut combined,next|{combined.findings.extend(next.findings);combined.unknowns.extend(next.unknowns);combined.errors.extend(next.errors);combined.truncated|=next.truncated;combined}).ok_or("此游戏的已加载 Skill 没有对应调查规则")?;
        report.findings.sort_by(|a,b|(&a.relative_path,&a.fact,&a.sha256).cmp(&(&b.relative_path,&b.fact,&b.sha256)));
        report.findings.dedup_by(|a,b|a.relative_path==b.relative_path&&a.fact==b.fact&&a.sha256==b.sha256);
        report.unknowns.sort();report.unknowns.dedup();
        report.report_id = uuid::Uuid::new_v4().to_string();
        report.mod_id = mod_id;
        report.game_id = project.game_id;
        report.inspected_at = crate::storage::mods::now()?;
        let state=app.state::<crate::storage::mods::StorageState>();
        let _lock=state.0.lock().map_err(|_|"项目存储锁异常")?;
        crate::project_lifecycle::repository::ensure_writable(&app,&report.mod_id)?;
        repository::save_on(&database::connect(&app)?, &report, &directory)?;
        Ok(report)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn list_inspections(
    app: tauri::AppHandle,
    mod_id: String,
) -> Result<Vec<models::InspectionReport>, String> {
    storage::read(&app, &mod_id)?;
    repository::list_on(&database::connect(&app)?, &mod_id)
}
#[tauri::command]
pub fn apply_inspection(
    app: tauri::AppHandle,
    mod_id: String,
    report_id: String,
    expected_context_revision: i64,
) -> Result<crate::project_context::models::ProjectContext, String> {
    let state=app.state::<crate::storage::mods::StorageState>();
    let _lock=state.0.lock().map_err(|_|"项目存储锁异常")?;
    crate::project_lifecycle::repository::ensure_writable(&app,&mod_id)?;
    storage::read(&app, &mod_id)?;
    let db = database::connect(&app)?;
    let (report, directory) = repository::read_on(&db, &mod_id, &report_id)?;
    let mut context = crate::project_context::repository::read_on(&db, &mod_id)?;
    if context.revision != expected_context_revision {
        return Err("环境已变化，请重载后采纳".into());
    }
    let mut sets = std::collections::BTreeMap::<
        String,
        Vec<crate::project_context::models::ReferenceFile>,
    >::new();
    for f in report
        .findings
        .iter()
        .filter(|f| f.fact == "referenceAssembly")
    {
        if let Some(hash) = &f.sha256 {
            let path = std::path::Path::new(&f.relative_path);
            let parent = path.parent().ok_or("参考文件路径无效")?;
            let root = std::path::Path::new(&directory)
                .join(parent)
                .to_string_lossy()
                .into_owned();
            sets.entry(root)
                .or_default()
                .push(crate::project_context::models::ReferenceFile {
                    path: path
                        .file_name()
                        .ok_or("参考文件名无效")?
                        .to_string_lossy()
                        .into_owned(),
                    sha256: hash.clone(),
                });
        }
    }
    if sets.is_empty() {
        return Err("报告没有可采纳的参考程序集；游戏版本仍未知".into());
    }
    for (root, files) in sets {
        context
            .reference_sets
            .retain(|s| s.directory.to_lowercase() != root.to_lowercase());
        context
            .reference_sets
            .push(crate::project_context::models::ReferenceSet {
                id: uuid::Uuid::new_v4().to_string(),
                directory: root,
                files,
                source_report_id: Some(report_id.clone()),
            });
    }
    let original = context.reference_sets.clone();
    crate::project_context::validate(&context)?;
    crate::project_context::fingerprint_references(&mut context)?;
    if context.reference_sets != original {
        return Err("调查后参考文件发生变化，请重新检查目录".into());
    }
    crate::project_context::repository::save_on(&db, &context, expected_context_revision)
}
