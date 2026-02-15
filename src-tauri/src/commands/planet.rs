use tauri::State;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::store::{PlanetStoreHandle, PlanetStoreSnapshot};
use crate::models::planet::MyPlanet;

// ============================================================
// 请求/响应类型
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreatePlanetRequest {
    pub name: String,
    pub about: String,
    #[serde(default = "default_template")]
    pub template_name: String,
}

fn default_template() -> String {
    "Plain".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlanetRequest {
    pub name: Option<String>,
    pub about: Option<String>,
    pub domain: Option<String>,
    pub author_name: Option<String>,
    pub twitter_username: Option<String>,
    pub github_username: Option<String>,
    pub telegram_username: Option<String>,
    pub mastodon_username: Option<String>,
    pub discord_link: Option<String>,
}

// ============================================================
// Tauri Commands
// ============================================================

/// 获取全局状态快照
#[tauri::command]
pub fn planet_get_state(
    store: State<'_, PlanetStoreHandle>,
) -> Result<PlanetStoreSnapshot, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    Ok(store.snapshot())
}

/// 列出所有 My Planets
#[tauri::command]
pub fn planet_list(
    store: State<'_, PlanetStoreHandle>,
) -> Result<Vec<MyPlanet>, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    Ok(store.my_planets.clone())
}

/// 创建 Planet
#[tauri::command]
pub fn planet_create(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    request: CreatePlanetRequest,
) -> Result<MyPlanet, String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;
    let planet = store
        .create_planet(request.name, request.about, request.template_name, &app)
        .map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);
    Ok(planet)
}

/// 获取单个 Planet 详情
#[tauri::command]
pub fn planet_get(
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
) -> Result<MyPlanet, String> {
    let uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store
        .get_planet(uuid)
        .cloned()
        .ok_or_else(|| format!("Planet not found: {}", planet_id))
}

/// 更新 Planet
#[tauri::command]
pub fn planet_update(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
    request: UpdatePlanetRequest,
) -> Result<MyPlanet, String> {
    let uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let mut store = store.lock().map_err(|e| e.to_string())?;

    store.update_planet(uuid, |planet| {
        if let Some(name) = request.name {
            planet.name = name;
        }
        if let Some(about) = request.about {
            planet.about = about;
        }
        if let Some(domain) = request.domain {
            planet.domain = Some(domain);
        }
        if let Some(author_name) = request.author_name {
            planet.author_name = Some(author_name);
        }
        if let Some(twitter) = request.twitter_username {
            planet.twitter_username = Some(twitter);
        }
        if let Some(github) = request.github_username {
            planet.github_username = Some(github);
        }
        if let Some(telegram) = request.telegram_username {
            planet.telegram_username = Some(telegram);
        }
        if let Some(mastodon) = request.mastodon_username {
            planet.mastodon_username = Some(mastodon);
        }
        if let Some(discord) = request.discord_link {
            planet.discord_link = Some(discord);
        }
    }, &app).map_err(|e| e.to_string())?;

    let planet = store.get_planet(uuid).cloned()
        .ok_or_else(|| "Planet disappeared after update".to_string())?;
    store.emit_state_changed(&app);
    Ok(planet)
}

/// 删除 Planet
#[tauri::command]
pub fn planet_delete(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let mut store = store.lock().map_err(|e| e.to_string())?;
    store.delete_planet(uuid, &app).map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);
    Ok(())
}