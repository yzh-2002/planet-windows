use tauri::State;
use uuid::Uuid;
use serde::Deserialize;

use crate::store::PlanetStoreHandle;
use crate::models::article::MyArticle;
use crate::models::draft::Draft;

// ============================================================
// 请求类型
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreateArticleRequest {
    pub planet_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArticleRequest {
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDraftRequest {
    pub planet_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveDraftRequest {
    pub planet_id: String,
    pub draft_id: String,
    pub title: String,
    pub content: String,
}

// ============================================================
// Article Commands
// ============================================================

/// 列出 Planet 的所有文章
#[tauri::command]
pub fn article_list(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
) -> Result<Vec<MyArticle>, String> {
    let uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store.list_articles(uuid, &app).map_err(|e| e.to_string())
}

/// 创建文章
#[tauri::command]
pub fn article_create(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    request: CreateArticleRequest,
) -> Result<MyArticle, String> {
    let planet_id = Uuid::parse_str(&request.planet_id).map_err(|e| e.to_string())?;
    let mut store = store.lock().map_err(|e| e.to_string())?;
    let article = store
        .create_article(planet_id, request.title, request.content, &app)
        .map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);
    Ok(article)
}

/// 获取单篇文章
#[tauri::command]
pub fn article_get(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
    article_id: String,
) -> Result<MyArticle, String> {
    let planet_uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let article_uuid = Uuid::parse_str(&article_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    let planet = store.get_planet(planet_uuid)
        .ok_or_else(|| format!("Planet not found: {}", planet_id))?;
    MyArticle::load(planet, article_uuid, &app).map_err(|e| e.to_string())
}

/// 更新文章
#[tauri::command]
pub fn article_update(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
    article_id: String,
    request: UpdateArticleRequest,
) -> Result<MyArticle, String> {
    let planet_uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let article_uuid = Uuid::parse_str(&article_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store
        .update_article(planet_uuid, article_uuid, request.title, request.content, &app)
        .map_err(|e| e.to_string())
}

/// 删除文章
#[tauri::command]
pub fn article_delete(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
    article_id: String,
) -> Result<(), String> {
    let planet_uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let article_uuid = Uuid::parse_str(&article_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store.delete_article(planet_uuid, article_uuid, &app).map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);
    Ok(())
}

// ============================================================
// Draft Commands
// ============================================================

/// 列出 Planet 的所有草稿
#[tauri::command]
pub fn draft_list(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
) -> Result<Vec<Draft>, String> {
    let uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store.list_drafts(uuid, &app).map_err(|e| e.to_string())
}

/// 创建草稿
#[tauri::command]
pub fn draft_create(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    request: CreateDraftRequest,
) -> Result<Draft, String> {
    let planet_id = Uuid::parse_str(&request.planet_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store
        .create_draft(planet_id, request.title, request.content, &app)
        .map_err(|e| e.to_string())
}

/// 保存草稿（更新标题和内容）
#[tauri::command]
pub fn draft_save(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    request: SaveDraftRequest,
) -> Result<(), String> {
    let planet_id = Uuid::parse_str(&request.planet_id).map_err(|e| e.to_string())?;
    let draft_id = Uuid::parse_str(&request.draft_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    let planet = store.get_planet(planet_id)
        .ok_or_else(|| format!("Planet not found: {}", request.planet_id))?;

    let mut draft = Draft::load(planet, draft_id, &app).map_err(|e| e.to_string())?;
    draft.title = request.title;
    draft.content = request.content;
    draft.date = chrono::Utc::now();
    draft.save(planet, &app).map_err(|e| e.to_string())
}

/// 删除草稿
#[tauri::command]
pub fn draft_delete(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
    draft_id: String,
) -> Result<(), String> {
    let planet_uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let draft_uuid = Uuid::parse_str(&draft_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store.delete_draft(planet_uuid, draft_uuid, &app).map_err(|e| e.to_string())
}

/// 发布草稿为文章
#[tauri::command]
pub fn draft_publish(
    app: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
    draft_id: String,
) -> Result<MyArticle, String> {
    let planet_uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let draft_uuid = Uuid::parse_str(&draft_id).map_err(|e| e.to_string())?;
    let mut store = store.lock().map_err(|e| e.to_string())?;
    let article = store
        .publish_draft(planet_uuid, draft_uuid, &app)
        .map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);
    Ok(article)
}