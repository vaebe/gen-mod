use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, io::Read, path::{Path, PathBuf}};
use crate::storage::paths;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SupportStatus { Supported, Planned }

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub steam_app_id: u32,
    pub name: String,
    pub description: String,
    pub cover: String,
    pub icon: String,
    pub sort_order: u32,
    pub support_status: SupportStatus,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog { schema_version: u32, games: Vec<Game> }

pub fn read_on(root: &Path) -> Result<Vec<Game>, String> {
    paths::reject_redirect(root)?;
    let file = root.join("catalog.json");
    paths::reject_redirect(&file)?;
    let catalog: Catalog = serde_json::from_slice(&fs::read(file).map_err(|error| format!("无法读取本地游戏清单：{error}"))?)
        .map_err(|error| format!("本地游戏清单无效：{error}"))?;
    if catalog.schema_version != 1 { return Err("不支持的本地游戏清单版本".into()); }
    let mut ids = BTreeSet::new();
    let mut apps = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for game in &catalog.games {
        if !crate::skills::registry::valid_name(&game.id) || game.steam_app_id == 0 || game.name.trim().is_empty()
            || !ids.insert(&game.id) || !apps.insert(game.steam_app_id) || !orders.insert(game.sort_order)
            || game.cover != format!("images/{}/cover.jpg", game.id)
            || game.icon != format!("images/{}/icon.jpg", game.id) {
            return Err("本地游戏清单包含无效或重复的游戏、排序或图片路径".into());
        }
    }
    let mut games = catalog.games;
    games.sort_by_key(|game| game.sort_order);
    Ok(games)
}

fn root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::local_resources::root(app)?.join("resources/games"))
}

pub fn list(app: &tauri::AppHandle) -> Result<Vec<Game>, String> { read_on(&root(app)?) }

pub fn ensure_supported(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let game = list(app)?.into_iter().find(|game| game.id == id).ok_or("游戏不在本地清单中")?;
    if !matches!(game.support_status, SupportStatus::Supported) { return Err("该游戏待适配，暂不支持创建项目".into()); }
    Ok(())
}

#[tauri::command]
pub fn list_games(app: tauri::AppHandle) -> Result<Vec<Game>, String> { list(&app) }

/// 只接收清单中的游戏 ID，不接受来自前端的文件路径。
#[tauri::command]
pub fn read_game_cover(app: tauri::AppHandle, game_id: String) -> Result<tauri::ipc::Response, String> {
    read_game_image(&app, &game_id, false)
}

#[tauri::command]
pub fn read_game_icon(app: tauri::AppHandle, game_id: String) -> Result<tauri::ipc::Response, String> {
    read_game_image(&app, &game_id, true)
}

fn read_game_image(app: &tauri::AppHandle, game_id: &str, icon: bool) -> Result<tauri::ipc::Response, String> {
    let root = root(app)?;
    let game = read_on(&root)?.into_iter().find(|game| game.id == game_id).ok_or("游戏不在本地清单中")?;
    let relative = if icon { &game.icon } else { &game.cover };
    let mut path = root.clone();
    for part in crate::changes::relative_path(relative)?.components() {
        path.push(part);
        paths::reject_redirect(&path)?;
    }
    let actual = path.canonicalize().map_err(|error| format!("游戏图片缺失：{error}"))?;
    if !actual.starts_with(root.canonicalize().map_err(|error| error.to_string())?) { return Err("游戏图片路径越界".into()); }
    let file = fs::File::open(actual).map_err(|error| error.to_string())?;
    if !file.metadata().map_err(|error| error.to_string())?.is_file() { return Err("游戏图片不是普通文件".into()); }
    let mut bytes = Vec::new();
    file.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    if bytes.len() > 2 * 1024 * 1024 || !bytes.starts_with(b"\xff\xd8\xff") || !bytes.ends_with(b"\xff\xd9") {
        return Err("游戏图片不是有效的 JPEG 文件或超过 2 MiB".into());
    }
    Ok(tauri::ipc::Response::new(bytes))
}
