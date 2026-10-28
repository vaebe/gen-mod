use crate::skills::official;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Manager;
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub description: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub category: String,
    pub game_id: Option<String>,
    pub version: String,
    pub dependencies: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub required_command_scripts: Vec<String>,
    pub inspection_file: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub id: String,
    pub skill_id: String,
    pub version: String,
    pub content_hash: String,
    pub file_id: String,
    pub uploaded_at: String,
    pub manifest: Manifest,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub game_id: Option<String>,
    pub versions: Vec<Version>,
}
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Catalog {
    pub games: Vec<Game>,
    pub skills: Vec<Skill>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVersion {
    pub skill_id: String,
    pub name: String,
    pub version: String,
    pub file_id: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Updates {
    pub new_games: Vec<Game>,
    pub versions: Vec<UpdateVersion>,
}
pub fn check_on(root: &Path, catalog: &Catalog) -> Result<Updates, String> {
    let mut confirmed = official::confirmed_on(root)?;
    let new_games = catalog
        .games
        .iter()
        .filter(|game| !confirmed.games.iter().any(|old| old.id == game.id))
        .cloned()
        .collect();
    for known in &mut confirmed.games {
        if let Some(game) = catalog.games.iter().find(|game| game.id == known.id) {
            *known = game.clone();
        }
    }
    let versions = catalog
        .skills
        .iter()
        .flat_map(|skill| {
            skill
                .versions
                .iter()
                .filter(|version| {
                    !confirmed.versions.iter().any(|old| {
                        old.skill_id == skill.id
                            && old.version.id == version.id
                            && old.version.content_hash == version.content_hash
                            && old.version.file_id == version.file_id
                    })
                })
                .map(|version| UpdateVersion {
                    skill_id: skill.id.clone(),
                    name: skill.name.clone(),
                    version: version.version.clone(),
                    file_id: version.file_id.clone(),
                })
        })
        .collect();
    official::save_catalog_on(root, catalog)?;
    official::save_confirmed_on(root, &confirmed)?;
    Ok(Updates {
        new_games,
        versions,
    })
}
pub(super) fn check_locked_on(
    lock: &std::sync::Mutex<()>,
    root: &Path,
    catalog: &Catalog,
) -> Result<Updates, String> {
    let _guard = lock.lock().map_err(|_| "项目存储锁异常")?;
    check_on(root, catalog)
}
async fn fetch(client: &super::auth::AuthClient) -> Result<Catalog, String> {
    let games =
        serde_json::from_slice(&client.get("/games").await?).map_err(|_| "游戏目录响应无效")?;
    let skills =
        serde_json::from_slice(&client.get("/skills").await?).map_err(|_| "Skill 目录响应无效")?;
    Ok(Catalog { games, skills })
}
pub(super) async fn downloads(
    catalog: &Catalog,
    file_ids: &[String],
    client: &super::auth::AuthClient,
) -> Result<Vec<(Skill, Version, Vec<u8>)>, String> {
    let mut selected = vec![];
    for id in file_ids {
        let pair = catalog
            .skills
            .iter()
            .find_map(|skill| {
                skill
                    .versions
                    .iter()
                    .find(|version| &version.file_id == id)
                    .map(|version| (skill.clone(), version.clone()))
            })
            .ok_or("版本目录已变化，请重新检查")?;
        selected.push(pair);
    }
    let mut result = vec![];
    for (skill, version) in selected {
        let bytes = client.get(&format!("/files/{}", version.file_id)).await?;
        result.push((skill, version, bytes));
    }
    Ok(result)
}
pub(super) fn apply_on(
    root: &Path,
    catalog: &Catalog,
    game_ids: &[String],
    downloads: Vec<(Skill, Version, Vec<u8>)>,
) -> Result<(), String> {
    let games: Vec<Game> = game_ids
        .iter()
        .map(|id| {
            catalog
                .games
                .iter()
                .find(|game| &game.id == id)
                .cloned()
                .ok_or_else(|| "游戏目录已变化，请重新检查".to_string())
        })
        .collect::<Result<_, _>>()?;
    let mut confirmed = official::confirmed_on(root)?;
    // 文件可提前落盘；整批校验完成前不切换任何已确认版本。
    for (skill, version, bytes) in downloads {
        let entry = official::prepare_on(root, &skill, &version, &bytes)?;
        confirmed
            .versions
            .retain(|old| old.version.id != entry.version.id);
        confirmed.versions.push(entry);
    }
    for game in games {
        confirmed.games.retain(|old| old.id != game.id);
        confirmed.games.push(game);
    }
    official::save_confirmed_on(root, &confirmed)
}
#[tauri::command]
pub async fn cloud_check_updates(app: tauri::AppHandle) -> Result<Updates, String> {
    let state = app.state::<super::auth::CloudState>();
    let _guard = state.0.lock().await;
    let catalog = fetch(&super::auth::client(&app)?).await?;
    check_locked_on(
        &app.state::<crate::storage::mods::StorageState>().0,
        &official::root(&app)?,
        &catalog,
    )
}
#[tauri::command]
pub async fn cloud_confirm_updates(
    app: tauri::AppHandle,
    game_ids: Vec<String>,
    file_ids: Vec<String>,
) -> Result<(), String> {
    let state = app.state::<super::auth::CloudState>();
    let _guard = state.0.lock().await;
    let client = super::auth::client(&app)?;
    let catalog = fetch(&client).await?;
    check_locked_on(
        &app.state::<crate::storage::mods::StorageState>().0,
        &official::root(&app)?,
        &catalog,
    )?;
    let packages = downloads(&catalog, &file_ids, &client).await?;
    let worker_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let storage = worker_app.state::<crate::storage::mods::StorageState>();
        let _storage = storage.0.lock().map_err(|_| "项目存储锁异常")?;
        apply_on(&official::root(&worker_app)?, &catalog, &game_ids, packages)?;
        for project in crate::storage::repository::list(&worker_app)? {
            crate::storage::repository::update(&worker_app, &project.mod_id, |project| {
                crate::skills::registry::refresh_sessions(&worker_app, project)
            })?;
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
pub fn cloud_games(app: tauri::AppHandle) -> Result<Vec<Game>, String> {
    super::auth::require_local(&app)?;
    Ok(official::confirmed_on(&official::root(&app)?)?.games)
}
