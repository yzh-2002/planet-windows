use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;
use serde::Deserialize;

use crate::store::{PlanetStoreHandle, PlanetStoreSnapshot};
use crate::models::planet::{MyPlanet, PublishState};
use crate::ipfs::state::IpfsStateHandle;
use crate::models::following_planet::{FollowingPlanet, FollowingPlanetSnapshot};
use crate::models::following_article::FollowingArticleSnapshot;

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

/// 触发发布
///
/// 对标 Swift MyPlanetModel.publish()
#[tauri::command]
pub async fn planet_publish(
    planet_id: String,
    app_handle: tauri::AppHandle,
    store: State<'_, PlanetStoreHandle>,
    ipfs_state: State<'_, IpfsStateHandle>,
) -> Result<(), String> {
    let planet_uuid = Uuid::parse_str(&planet_id)
        .map_err(|_| format!("无效的 Planet ID: {}", planet_id))?;

    let publish_started_at = chrono::Utc::now();

    // 发送发布开始事件
    {
        let store = store.lock().map_err(|e| e.to_string())?;
        let planet = store.my_planets
            .iter()
            .find(|p| p.id == planet_uuid)
            .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;
        planet.emit_publish_state(&app_handle, "saving", None, None, &publish_started_at);
    }

    // 第一步：获取模板名称（需要短暂持有 store lock）
    let template_name = {
        let store = store.lock().map_err(|e| e.to_string())?;
        let planet = store.my_planets
            .iter()
            .find(|p| p.id == planet_uuid)
            .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;
        planet.template_name.clone()
    };

    // 第二步：加载模板（不持有任何 lock）
    let templates_path = crate::helpers::paths::get_templates_path(&app_handle);
    tracing::info!("加载模板路径: {:?}", templates_path);
    
    let template_store = crate::template::TemplateStore::load(&templates_path)
        .map_err(|e| format!("加载模板失败: {}", e))?;

    let template = template_store.get(&template_name)
        .ok_or_else(|| format!("模板 '{}' 不存在", template_name))?;
    
    tracing::info!("使用模板: {} (路径: {:?})", template.info.name, template.path);

    // 第三步：执行发布流程
    // 注意：不能在持有 MutexGuard 的情况下跨越 await
    // 解决方案：先执行同步部分，释放 lock，然后执行异步部分，最后重新获取 lock 更新
    
    tracing::info!("开始 save_public...");
    // 先执行同步部分（save_public）
    {
        let mut store = store.lock().map_err(|e| e.to_string())?;
        let planet = store.my_planets
            .iter_mut()
            .find(|p| p.id == planet_uuid)
            .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;
        
        // 执行同步的 save_public
        if let Err(e) = planet.save_public(template, &app_handle) {
            let error_msg = format!("生成静态站点失败: {}", e);
            tracing::error!("{}", error_msg);
            planet.emit_publish_state(&app_handle, "error", None, Some(&error_msg), &publish_started_at);
            return Err(error_msg);
        }
    }
    tracing::info!("save_public 完成");
    
    // 现在执行异步部分（add_directory, name/publish 等）
    // 获取 public_path（需要短暂持有 store lock）
    let public_path = {
        let store = store.lock().map_err(|e| e.to_string())?;
        let planet = store.my_planets
            .iter()
            .find(|p| p.id == planet_uuid)
            .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;
        planet.public_base_path(&app_handle)
    };
    
    // 检查并确保 IPFS key 存在
    let key_name = planet_uuid.to_string();
    tracing::info!("检查 IPFS key: {}", key_name);
    {
        let ipfs = ipfs_state.lock().await;
        let daemon = &ipfs.daemon;
        
        // 检查 key 是否存在
        let key_exists = daemon.check_key_exists(&key_name)
            .map_err(|e| format!("检查 IPFS key 失败: {}", e))?;
        
        if !key_exists {
            tracing::info!("IPFS key 不存在，尝试恢复或生成...");
            // 尝试从 Keystore 恢复
            if crate::keystore::Keystore::check(&key_name) {
                tracing::info!("IPFS key '{}' 不存在，尝试从 Keystore 恢复", key_name);
                crate::keystore::Keystore::import_key_from_keystore(daemon, &key_name, &app_handle)
                    .map_err(|e| format!("从 Keystore 恢复密钥失败: {}", e))?;
            } else {
                // Keystore 中也没有，生成新的 key
                tracing::info!("IPFS key '{}' 不存在，生成新密钥", key_name);
                
                // 生成新 key
                daemon.generate_key(&key_name)
                    .map_err(|e| format!("生成 IPFS key 失败: {}", e))?;
                
                // 保存到 Keystore
                crate::keystore::Keystore::export_key_to_keystore(daemon, &key_name, &app_handle)
                    .map_err(|e| format!("保存密钥到 Keystore 失败: {}", e))?;
            }
        }
    }
    tracing::info!("IPFS key 检查完成");
    
    // 发送上传开始事件
    {
        let store = store.lock().map_err(|e| e.to_string())?;
        let planet = store.my_planets
            .iter()
            .find(|p| p.id == planet_uuid)
            .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;
        planet.emit_publish_state(&app_handle, "uploading", None, None, &publish_started_at);
    }

    // 执行 add_directory（需要持有 ipfs lock，但 add_directory 是同步的）
    let cid = match {
        let ipfs = ipfs_state.lock().await;
        let daemon = &ipfs.daemon;
        daemon.add_directory(public_path.to_str().unwrap())
    } {
        Ok(cid) => cid,
        Err(e) => {
            // 发送错误事件
            let store = store.lock().map_err(|e| e.to_string())?;
            if let Some(planet) = store.my_planets.iter().find(|p| p.id == planet_uuid) {
                let error_msg = format!("上传目录失败: {}", e);
                planet.emit_publish_state(&app_handle, "error", None, Some(&error_msg), &publish_started_at);
            }
            return Err(format!("上传目录失败: {}", e));
        }
    };
    
    if cid.is_empty() {
        // 发送错误事件
        let store = store.lock().map_err(|e| e.to_string())?;
        if let Some(planet) = store.my_planets.iter().find(|p| p.id == planet_uuid) {
            planet.emit_publish_state(&app_handle, "error", None, Some("上传目录返回空 CID"), &publish_started_at);
        }
        return Err("上传目录返回空 CID".to_string());
    }
    
    // 发送发布到 IPNS 开始事件
    {
        let store = store.lock().map_err(|e| e.to_string())?;
        let planet = store.my_planets
            .iter()
            .find(|p| p.id == planet_uuid)
            .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;
        planet.emit_publish_state(&app_handle, "publishing", Some(&cid), None, &publish_started_at);
    }

    // 执行 name/publish（异步，需要持有 ipfs lock）
    let ipns_address = {
        let mut args = std::collections::HashMap::new();
        args.insert("arg".to_string(), cid.clone());
        args.insert("allow-offline".to_string(), "1".to_string());
        args.insert("key".to_string(), key_name.clone());
        args.insert("quieter".to_string(), "1".to_string());
        args.insert("lifetime".to_string(), "7200h".to_string());
        
        let ipfs = ipfs_state.lock().await;
        let daemon = &ipfs.daemon;
        
        // 获取 name/publish 的 JSON 响应
        match daemon.api_json::<crate::ipfs::models::IpfsPublished>("name/publish", Some(&args), Some(180)).await {
            Ok(response) => {
                // 从响应中提取 IPNS 地址（格式：/ipns/k51...）
                // 需要去掉 /ipns/ 前缀
                response.name.strip_prefix("/ipns/")
                    .unwrap_or(&response.name)
                    .to_string()
            }
            Err(e) => {
                // 发送错误事件
                let store = store.lock().map_err(|e| e.to_string())?;
                if let Some(planet) = store.my_planets.iter().find(|p| p.id == planet_uuid) {
                    let error_msg = format!("IPNS 发布失败: {}", e);
                    planet.emit_publish_state(&app_handle, "error", Some(&cid), Some(&error_msg), &publish_started_at);
                }
                return Err(format!("IPNS 发布失败: {}", e));
            }
        }
    };
    
    // 更新 store 状态
    {
        let mut store = store.lock().map_err(|e| e.to_string())?;
        let planet = store.my_planets
            .iter_mut()
            .find(|p| p.id == planet_uuid)
            .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;
        
        planet.last_published = Some(chrono::Utc::now());
        planet.last_published_cid = Some(cid.clone());
        planet.ipns = Some(ipns_address.clone());  // 更新 IPNS 地址
        planet.save(&app_handle)
            .map_err(|e| format!("保存 Planet 失败: {}", e))?;
        
        // 发送完成事件
        planet.emit_publish_state(&app_handle, "done", Some(&cid), None, &publish_started_at);
        
        store.emit_state_changed(&app_handle);
    }
    
    Ok(())
}

/// 查询发布状态
#[tauri::command]
pub fn planet_get_publish_state(
    planet_id: String,
    store: State<'_, PlanetStoreHandle>,
) -> Result<PublishState, String> {
    let store = store.lock().map_err(|e| e.to_string())?;

    let planet_uuid = Uuid::parse_str(&planet_id)
        .map_err(|_| format!("无效的 Planet ID: {}", planet_id))?;
    
    let planet = store.my_planets
        .iter()
        .find(|p| p.id == planet_uuid)
        .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;

    // 注意：MyPlanet 结构体中没有 is_publishing 和 publish_started_at 字段
    // 这些状态通过事件系统实时更新，这里返回基本状态
    Ok(PublishState {
        planet_id: planet.id.to_string(),
        is_publishing: false,  // 实际状态通过事件系统获取
        step: "idle".to_string(),
        cid: planet.last_published_cid.clone(),
        error: None,
        started_at: planet.last_published.map(|dt| dt.to_rfc3339()),
    })
}

/// 更新 Filebase 设置
#[tauri::command]
pub fn planet_update_filebase(
    app: tauri::AppHandle,
    planet_id: String,
    enabled: bool,
    pin_name: Option<String>,
    api_token: Option<String>,
    store: State<'_, PlanetStoreHandle>,
) -> Result<(), String> {
    let planet_uuid = Uuid::parse_str(&planet_id)
        .map_err(|_| format!("无效的 Planet ID: {}", planet_id))?;
    
    let mut store = store.lock().map_err(|e| e.to_string())?;
    let planet = store.my_planets
        .iter_mut()
        .find(|p| p.id == planet_uuid)
        .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;

    planet.filebase_enabled = Some(enabled);
    planet.filebase_pin_name = pin_name;
    planet.filebase_api_token = api_token;

    // 使用已有的 save 方法
    planet.save(&app).map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);

    Ok(())
}

/// 更新 Pinnable 设置
#[tauri::command]
pub fn planet_update_pinnable(
    app: tauri::AppHandle,
    planet_id: String,
    enabled: bool,
    api_endpoint: Option<String>,
    store: State<'_, PlanetStoreHandle>,
) -> Result<(), String> {
    let planet_uuid = Uuid::parse_str(&planet_id)
        .map_err(|_| format!("无效的 Planet ID: {}", planet_id))?;
    
    let mut store = store.lock().map_err(|e| e.to_string())?;
    let planet = store.my_planets
        .iter_mut()
        .find(|p| p.id == planet_uuid)
        .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;

    planet.pinnable_enabled = Some(enabled);
    planet.pinnable_api_endpoint = api_endpoint;

    // 使用已有的 save 方法
    planet.save(&app).map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);

    Ok(())
}

/// 检查 Filebase pin 状态
#[tauri::command]
pub async fn planet_check_filebase_status(
    planet_id: String,
    store: State<'_, PlanetStoreHandle>,
) -> Result<Option<crate::integrations::filebase::FilebasePin>, String> {
    let planet_uuid = Uuid::parse_str(&planet_id)
        .map_err(|_| format!("无效的 Planet ID: {}", planet_id))?;
    
    // 在 await 之前克隆需要的数据，然后释放 lock
    let (api_token, pin_name, request_id) = {
        let store = store.lock().map_err(|e| e.to_string())?;
        let planet = store.my_planets
            .iter()
            .find(|p| p.id == planet_uuid)
            .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;

        // 检查是否有 Filebase 配置和 request_id，并克隆数据
        match (
            planet.filebase_api_token.clone(),
            planet.filebase_pin_name.clone(),
            planet.filebase_request_id.clone(),
        ) {
            (Some(api_token), Some(pin_name), Some(request_id)) => {
                (Some(api_token), Some(pin_name), Some(request_id))
            }
            _ => (None, None, None),
        }
    };

    // 现在 store lock 已释放，可以安全地 await
    if let (Some(api_token), Some(pin_name), Some(request_id)) = (api_token, pin_name, request_id) {
        let filebase = crate::integrations::filebase::Filebase::new(
            pin_name,
            api_token,
        );
        let result: Option<crate::integrations::filebase::FilebasePin> = filebase.check_pin_status(&request_id)
            .await
            .map_err(|e| format!("检查 Filebase pin 状态失败: {}", e))?;
        return Ok(result);
    }

    Ok(None)
}

// ============================================================
// 关注
// ============================================================

/// 关注一个 Planet / ENS / Feed
///
/// 由于 FollowingPlanet::follow() 内部使用 scraper 等非 Send 类型，
/// 需要通过 spawn_blocking + Handle::block_on 执行异步操作。
#[tauri::command]
pub async fn planet_follow(
    link: String,
    app_handle: AppHandle,
    store: State<'_, PlanetStoreHandle>,
    ipfs_state: State<'_, IpfsStateHandle>,
) -> Result<FollowingPlanetSnapshot, String> {
    // 获取已关注链接（短暂持有 std::sync::Mutex）
    let existing_links: Vec<String> = {
        let store = store.lock().map_err(|e| e.to_string())?;
        store.following_links()
    };

    // 在 spawn_blocking 中执行异步 follow（避免非 Send future 问题）
    let ipfs_clone = ipfs_state.inner().clone();
    let app_clone = app_handle.clone();
    let result = tokio::task::spawn_blocking(move || {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(async move {
            let ipfs = ipfs_clone.lock().await;
            FollowingPlanet::follow(&link, &existing_links, &ipfs.daemon, &app_clone).await
        })
    })
    .await
    .map_err(|e| format!("关注任务失败: {}", e))?;

    let planet = result.map_err(|e| format!("关注失败: {}", e))?;
    let snapshot = planet.snapshot(&app_handle);

    // 加入 store
    {
        let mut store = store.lock().map_err(|e| e.to_string())?;
        store.add_following_planet(planet);
        store.emit_state_changed(&app_handle);
    }

    Ok(snapshot)
}

/// 取消关注
#[tauri::command]
pub fn planet_unfollow(
    id: String,
    app_handle: AppHandle,
    store: State<'_, PlanetStoreHandle>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut store = store.lock().map_err(|e| e.to_string())?;
    store.unfollow_planet(uuid, &app_handle).map_err(|e| e.to_string())?;
    store.emit_state_changed(&app_handle);
    Ok(())
}

// ============================================================
// 列表查询
// ============================================================

/// 获取所有关注的 Planet 列表
#[tauri::command]
pub fn following_list(
    app_handle: AppHandle,
    store: State<'_, PlanetStoreHandle>,
) -> Result<Vec<FollowingPlanetSnapshot>, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    Ok(store
        .following_planets
        .iter()
        .map(|p| p.snapshot(&app_handle))
        .collect())
}

/// 获取某个关注 Planet 的文章列表
#[tauri::command]
pub fn following_articles(
    id: String,
    store: State<'_, PlanetStoreHandle>,
) -> Result<Vec<FollowingArticleSnapshot>, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;

    if let Some(planet) = store.following_planets.iter().find(|p| p.id == uuid) {
        Ok(planet.articles.iter().map(|a| a.snapshot()).collect())
    } else {
        Err("未找到该 Planet".to_string())
    }
}

/// 获取单篇文章详情
#[tauri::command]
pub fn following_article_get(
    planet_id: String,
    article_id: String,
    app_handle: AppHandle,
    store: State<'_, PlanetStoreHandle>,
) -> Result<FollowingArticleSnapshot, String> {
    let p_uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let a_uuid = Uuid::parse_str(&article_id).map_err(|e| e.to_string())?;

    let mut store = store.lock().map_err(|e| e.to_string())?;
    if let Some(planet) = store.following_planets.iter_mut().find(|p| p.id == p_uuid) {
        let articles_dir = planet.articles_path(&app_handle);
        if let Some(article) = planet.articles.iter_mut().find(|a| a.id == a_uuid) {
            // 标记为已读
            article.mark_as_read();
            let _ = article.save(&articles_dir);
            Ok(article.snapshot())
        } else {
            Err("未找到该文章".to_string())
        }
    } else {
        Err("未找到该 Planet".to_string())
    }
}

// ============================================================
// 更新
// ============================================================

/// 更新单个 Following Planet
///
/// 采用 clone → spawn_blocking(update) → write-back 策略，
/// 解决 std::sync::Mutex 不能跨 .await 和 scraper 非 Send 的问题。
#[tauri::command]
pub async fn following_update(
    id: String,
    app_handle: AppHandle,
    store: State<'_, PlanetStoreHandle>,
    ipfs_state: State<'_, IpfsStateHandle>,
) -> Result<FollowingPlanetSnapshot, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    // 克隆 planet（短暂持有 std::sync::Mutex）
    let planet = {
        let store = store.lock().map_err(|e| e.to_string())?;
        store
            .following_planets
            .iter()
            .find(|p| p.id == uuid)
            .cloned()
            .ok_or_else(|| "未找到该 Planet".to_string())?
    };

    // 在 spawn_blocking 中执行异步更新
    let ipfs_clone = ipfs_state.inner().clone();
    let app_clone = app_handle.clone();
    let result = tokio::task::spawn_blocking(move || {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(async move {
            let mut planet = planet;
            let ipfs = ipfs_clone.lock().await;
            let result = planet.update(&ipfs.daemon, &app_clone).await;
            (planet, result)
        })
    })
    .await
    .map_err(|e| format!("更新任务失败: {}", e))?;

    let (updated_planet, update_result) = result;
    update_result.map_err(|e| format!("更新失败: {}", e))?;

    let snapshot = updated_planet.snapshot(&app_handle);

    // 写回 store
    {
        let mut store = store.lock().map_err(|e| e.to_string())?;
        if let Some(p) = store.following_planets.iter_mut().find(|p| p.id == uuid) {
            *p = updated_planet;
        }
    }

    let _ = app_handle.emit(
        "following-updated",
        serde_json::json!({ "id": uuid.to_string() }),
    );

    Ok(snapshot)
}

/// 更新所有 Following Planets
#[tauri::command]
pub async fn following_update_all(
    app_handle: AppHandle,
    store: State<'_, PlanetStoreHandle>,
    ipfs_state: State<'_, IpfsStateHandle>,
) -> Result<(), String> {
    // 获取所有 ID
    let ids: Vec<Uuid> = {
        let store = store.lock().map_err(|e| e.to_string())?;
        store.following_planets.iter().map(|p| p.id).collect()
    };

    for id in ids {
        // 克隆 planet
        let planet_opt = {
            let store = store.lock().map_err(|e| e.to_string())?;
            store.following_planets.iter().find(|p| p.id == id).cloned()
        };

        if let Some(planet) = planet_opt {
            let ipfs_clone = ipfs_state.inner().clone();
            let app_clone = app_handle.clone();

            let result = tokio::task::spawn_blocking(move || {
                let handle = tokio::runtime::Handle::current();
                handle.block_on(async move {
                    let mut planet = planet;
                    let ipfs = ipfs_clone.lock().await;
                    let result = planet.update(&ipfs.daemon, &app_clone).await;
                    (planet, result)
                })
            })
            .await
            .map_err(|e| format!("更新任务失败: {}", e))?;

            match result {
                (updated_planet, Ok(_)) => {
                    // 写回 store
                    {
                        let mut store = store.lock().map_err(|e| e.to_string())?;
                        if let Some(p) =
                            store.following_planets.iter_mut().find(|p| p.id == id)
                        {
                            *p = updated_planet;
                        }
                    }
                    let _ = app_handle.emit(
                        "following-updated",
                        serde_json::json!({
                            "id": id.to_string(),
                        }),
                    );
                }
                (planet, Err(e)) => {
                    tracing::warn!("更新失败 {}: {}", planet.name, e);
                }
            }
        }
    }

    Ok(())
}

// ============================================================
// 文章操作
// ============================================================

/// 标记文章为已读
#[tauri::command]
pub fn following_article_mark_read(
    planet_id: String,
    article_id: String,
    app_handle: AppHandle,
    store: State<'_, PlanetStoreHandle>,
) -> Result<(), String> {
    let p_uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let a_uuid = Uuid::parse_str(&article_id).map_err(|e| e.to_string())?;

    let mut store = store.lock().map_err(|e| e.to_string())?;
    if let Some(planet) = store.following_planets.iter_mut().find(|p| p.id == p_uuid) {
        let articles_dir = planet.articles_path(&app_handle);
        if let Some(article) = planet.articles.iter_mut().find(|a| a.id == a_uuid) {
            article.mark_as_read();
            article.save(&articles_dir).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("未找到该文章".to_string())
        }
    } else {
        Err("未找到该 Planet".to_string())
    }
}

/// 标记文章为未读
#[tauri::command]
pub fn following_article_mark_unread(
    planet_id: String,
    article_id: String,
    app_handle: AppHandle,
    store: State<'_, PlanetStoreHandle>,
) -> Result<(), String> {
    let p_uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let a_uuid = Uuid::parse_str(&article_id).map_err(|e| e.to_string())?;

    let mut store = store.lock().map_err(|e| e.to_string())?;
    if let Some(planet) = store.following_planets.iter_mut().find(|p| p.id == p_uuid) {
        let articles_dir = planet.articles_path(&app_handle);
        if let Some(article) = planet.articles.iter_mut().find(|a| a.id == a_uuid) {
            article.mark_as_unread();
            article.save(&articles_dir).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("未找到该文章".to_string())
        }
    } else {
        Err("未找到该 Planet".to_string())
    }
}
