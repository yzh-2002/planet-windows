use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};
use uuid::Uuid;
use anyhow::{anyhow, Result};
use tracing::{error, info, warn};
use tauri::{AppHandle, Emitter};

use crate::models::planet::MyPlanet;
use crate::models::article::MyArticle;
use crate::models::draft::Draft;
use crate::models::following_planet::FollowingPlanet;
use crate::models::following_article::FollowingArticle;
use crate::ipfs::state::IpfsStateHandle;

// ============================================================
// SelectedView 枚举
// 对应原项目 PlanetDetailViewType
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum SelectedView {
    Today,
    Unread,
    Starred,
    MyPlanet(Uuid),
    FollowingPlanet(Uuid),
}

// ============================================================
// PlanetStore 全局状态
// 对应原项目 PlanetStore.swift
// ============================================================

#[derive(Debug)]
pub struct PlanetStore {
    pub my_planets: Vec<MyPlanet>,
    pub following_planets: Vec<FollowingPlanet>,
    pub selected_view: Option<SelectedView>,
    pub selected_planet_articles: Vec<MyArticle>,
    pub selected_following_articles: Vec<FollowingArticle>,
    pub selected_article_id: Option<Uuid>,
}

/// 全局 PlanetStore 的类型别名
pub type PlanetStoreHandle = Arc<Mutex<PlanetStore>>;

impl PlanetStore {
    /// 创建新的 PlanetStore 实例
    pub fn new() -> Self {
        Self {
            my_planets: Vec::new(),
            following_planets: Vec::new(),
            selected_view: None,
            selected_planet_articles: Vec::new(),
            selected_following_articles: Vec::new(),
            selected_article_id: None,
        }
    }

    /// 从磁盘加载所有数据
    pub fn load(&mut self, app: &AppHandle) -> Result<()> {
        info!("Loading planets from disk...");

        // 加载 My Planets
        self.my_planets = MyPlanet::load_all(app)?;
        info!("Loaded {} my planets", self.my_planets.len());

        // 加载 Following Planets
        self.following_planets = self.load_following_planets(app);
        info!("Loaded {} following planets", self.following_planets.len());

        Ok(())
    }

    /// 加载所有 Following Planets
    ///
    /// 对标 Swift PlanetStore 中加载 Following 列表的逻辑
    pub fn load_following_planets(&self, app: &AppHandle) -> Vec<FollowingPlanet> {
        let base = FollowingPlanet::following_planets_path(app);
        if !base.exists() {
            return Vec::new();
        }

        let mut planets = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    match FollowingPlanet::load(&path) {
                        Ok(planet) => {
                            info!("加载 Following Planet: {} ({})", planet.name, planet.id);
                            planets.push(planet);
                        }
                        Err(e) => {
                            warn!("加载 Following Planet 失败 {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        planets.sort_by(|a, b| b.updated.cmp(&a.updated));
        planets
    }

    // ============================================================
    // My Planet CRUD
    // ============================================================

    /// 创建新 Planet
    pub fn create_planet(
        &mut self,
        name: String,
        about: String,
        template_name: String,
        app: &AppHandle,
    ) -> Result<MyPlanet> {
        let planet = MyPlanet::create(name, about, template_name, app)?;
        self.my_planets.insert(0, planet.clone());
        Ok(planet)
    }

    /// 获取 Planet（不可变引用）
    pub fn get_planet(&self, planet_id: Uuid) -> Option<&MyPlanet> {
        self.my_planets.iter().find(|p| p.id == planet_id)
    }

    /// 获取 Planet（可变引用）
    pub fn get_planet_mut(&mut self, planet_id: Uuid) -> Option<&mut MyPlanet> {
        self.my_planets.iter_mut().find(|p| p.id == planet_id)
    }

    /// 更新 Planet
    pub fn update_planet<F>(&mut self, planet_id: Uuid, f: F, app: &AppHandle) -> Result<()>
    where
        F: FnOnce(&mut MyPlanet),
    {
        let planet = self.my_planets.iter_mut().find(|p| p.id == planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        planet.update(f, app)?;
        Ok(())
    }

    /// 删除 Planet
    pub fn delete_planet(&mut self, planet_id: Uuid, app: &AppHandle) -> Result<()> {
        if let Some(idx) = self.my_planets.iter().position(|p| p.id == planet_id) {
            let planet = &self.my_planets[idx];
            planet.delete(app)?;
            self.my_planets.remove(idx);
            Ok(())
        } else {
            Err(anyhow!("Planet not found: {}", planet_id))
        }
    }

    // ============================================================
    // Article CRUD
    // ============================================================

    /// 获取 Planet 的所有文章
    pub fn list_articles(&self, planet_id: Uuid, app: &AppHandle) -> Result<Vec<MyArticle>> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        MyArticle::load_all(planet, app)
    }

    /// 创建新文章
    pub fn create_article(
        &mut self,
        planet_id: Uuid,
        title: String,
        content: String,
        app: &AppHandle,
    ) -> Result<MyArticle> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        let article = MyArticle::create(planet_id, title, content)?;
        article.save(planet, app)?;

        // 更新 Planet 时间戳
        if let Some(planet) = self.get_planet_mut(planet_id) {
            planet.updated = chrono::Utc::now();
            planet.save(app)?;
        }

        Ok(article)
    }

    /// 更新文章
    pub fn update_article(
        &self,
        planet_id: Uuid,
        article_id: Uuid,
        title: Option<String>,
        content: Option<String>,
        app: &AppHandle,
    ) -> Result<MyArticle> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        let mut article = MyArticle::load(planet, article_id, app)?;

        article.update(planet, |a| {
            if let Some(t) = title {
                a.title = t;
            }
            if let Some(c) = content {
                a.content = c;
            }
        }, app)?;

        Ok(article)
    }

    /// 删除文章
    pub fn delete_article(&self, planet_id: Uuid, article_id: Uuid, app: &AppHandle) -> Result<()> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        let article = MyArticle::load(planet, article_id, app)?;
        article.delete(planet, app)
    }

    // ============================================================
    // Draft CRUD
    // ============================================================

    /// 获取 Planet 的所有草稿
    pub fn list_drafts(&self, planet_id: Uuid, app: &AppHandle) -> Result<Vec<Draft>> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        Draft::load_all(planet, app)
    }

    /// 创建新草稿
    pub fn create_draft(
        &self,
        planet_id: Uuid,
        title: String,
        content: String,
        app: &AppHandle,
    ) -> Result<Draft> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        let draft = Draft::create_new(planet_id, title, content);
        draft.save(planet, app)?;
        Ok(draft)
    }

    /// 保存草稿
    pub fn save_draft(&self, planet_id: Uuid, draft: &Draft, app: &AppHandle) -> Result<()> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        draft.save(planet, app)
    }

    /// 删除草稿
    pub fn delete_draft(&self, planet_id: Uuid, draft_id: Uuid, app: &AppHandle) -> Result<()> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        let draft = Draft::load(planet, draft_id, app)?;
        draft.delete(planet, app)
    }

    /// 发布草稿为文章
    pub fn publish_draft(
        &mut self,
        planet_id: Uuid,
        draft_id: Uuid,
        app: &AppHandle,
    ) -> Result<MyArticle> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?
            .clone();  // 需要 clone 因为 publish_to_article 需要 &mut MyPlanet

        let draft = Draft::load(&planet, draft_id, app)?;
        let mut planet_mut = planet;
        let article = draft.publish_to_article(&mut planet_mut, app)?;

        // 更新内存中的 Planet
        if let Some(idx) = self.my_planets.iter().position(|p| p.id == planet_id) {
            self.my_planets[idx] = planet_mut;
        }

        Ok(article)
    }

    // ============================================================
    // Following Planet CRUD
    // ============================================================

    /// 将已构建好的 FollowingPlanet 加入列表
    ///
    /// 实际的 follow 流程（IPNS 解析、ENS 解析等）在 FollowingPlanet::follow() 中异步完成，
    /// 完成后调用此方法将结果加入内存列表。
    pub fn add_following_planet(&mut self, planet: FollowingPlanet) {
        self.following_planets.insert(0, planet);
    }

    /// 取消关注 Planet
    pub fn unfollow_planet(&mut self, planet_id: Uuid, app: &AppHandle) -> Result<()> {
        if let Some(idx) = self.following_planets.iter().position(|p| p.id == planet_id) {
            let planet = &self.following_planets[idx];
            planet.delete(app)?;
            self.following_planets.remove(idx);
            Ok(())
        } else {
            Err(anyhow!("Following planet not found: {}", planet_id))
        }
    }

    /// 获取 Following Planet（不可变引用）
    pub fn get_following_planet(&self, planet_id: Uuid) -> Option<&FollowingPlanet> {
        self.following_planets.iter().find(|p| p.id == planet_id)
    }

    /// 获取 Following Planet（可变引用）
    pub fn get_following_planet_mut(&mut self, planet_id: Uuid) -> Option<&mut FollowingPlanet> {
        self.following_planets.iter_mut().find(|p| p.id == planet_id)
    }

    /// 获取 Following Planet 的所有文章
    ///
    /// 文章已在加载 Planet 时从磁盘读入内存 (planet.articles)
    pub fn list_following_articles(&self, planet_id: Uuid) -> Result<Vec<FollowingArticle>> {
        let planet = self.following_planets.iter().find(|p| p.id == planet_id)
            .ok_or_else(|| anyhow!("Following planet not found: {}", planet_id))?;
        Ok(planet.articles.clone())
    }

    /// 获取所有已关注链接（用于去重检查）
    pub fn following_links(&self) -> Vec<String> {
        self.following_planets.iter().map(|p| p.link.clone()).collect()
    }
}

// ============================================================
// PlanetStoreSnapshot（发送给前端的快照）
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanetStoreSnapshot {
    pub my_planets: Vec<MyPlanet>,
    pub following_planets: Vec<FollowingPlanet>,
    pub selected_view: Option<SelectedView>,
}

impl PlanetStore {
    pub fn snapshot(&self) -> PlanetStoreSnapshot {
        PlanetStoreSnapshot {
            my_planets: self.my_planets.clone(),
            following_planets: self.following_planets.clone(),
            selected_view: self.selected_view.clone(),
        }
    }

    /// 通过 Tauri 事件通知前端状态变化
    pub fn emit_state_changed(&self, app: &AppHandle) {
        let snapshot = self.snapshot();
        if let Err(e) = app.emit("planet:state-changed", &snapshot) {
            error!("Failed to emit planet state: {}", e);
        }
    }

    /// 启动后台更新定时器
    ///
    /// 对标 Swift PlanetStore 的定时更新逻辑：
    /// 定期遍历所有 Following Planet 并拉取更新。
    ///
    /// 由于 PlanetStore 使用 std::sync::Mutex，不能跨 .await 持有锁，
    /// 因此采用 clone → update → write-back 策略。
    ///
    /// 由于 planet.update() 内部使用 scraper 等非 Send 类型，
    /// 每个 planet 的更新通过 spawn_blocking + Handle::block_on 执行，
    /// 避免 tokio::spawn 的 Send 约束问题。
    pub fn start_background_updater(
        store_handle: PlanetStoreHandle,
        ipfs_state: IpfsStateHandle,
        app_handle: AppHandle,
    ) {
        tauri::async_runtime::spawn(async move {
            // 首次延迟 30 秒再开始，等待 IPFS daemon 完全就绪
            tokio::time::sleep(Duration::from_secs(30)).await;

            // 每 5 分钟更新一次（可配置）
            let mut timer = interval(Duration::from_secs(5 * 60));

            loop {
                timer.tick().await;
                info!("开始后台更新所有 Following Planets...");

                // 1. 获取所有 planet 的 id（短暂持有 std::sync::Mutex）
                let ids: Vec<Uuid> = {
                    let store = store_handle.lock().unwrap();
                    store.following_planets.iter().map(|p| p.id).collect()
                };

                for id in &ids {
                    // 2. 克隆出目标 planet（短暂持有 std::sync::Mutex）
                    let planet_opt = {
                        let store = store_handle.lock().unwrap();
                        store.following_planets.iter().find(|p| p.id == *id).cloned()
                    };

                    if let Some(planet) = planet_opt {
                        // 3. 在 spawn_blocking 中执行异步更新
                        //    planet.update() 使用了 scraper 等非 Send 类型，
                        //    不能直接在 tokio::spawn 的 async block 中 .await，
                        //    但 spawn_blocking 内可以通过 Handle::block_on 安全地运行。
                        let ipfs_clone = ipfs_state.clone();
                        let app_clone = app_handle.clone();
                        let id_copy = *id;

                        let result = tokio::task::spawn_blocking(move || {
                            let handle = tokio::runtime::Handle::current();
                            handle.block_on(async move {
                                let mut planet = planet;
                                let ipfs = ipfs_clone.lock().await;
                                let result = planet.update(&ipfs.daemon, &app_clone).await;
                                (planet, result)
                            })
                        })
                        .await;

                        match result {
                            Ok((updated_planet, Ok(_))) => {
                                info!("更新完成: {}", updated_planet.name);
                                // 4. 将更新后的 planet 写回 store
                                {
                                    let mut store = store_handle.lock().unwrap();
                                    if let Some(p) =
                                        store.following_planets.iter_mut().find(|p| p.id == id_copy)
                                    {
                                        *p = updated_planet;
                                    }
                                }
                                // 5. 通知前端
                                let _ = app_handle.emit(
                                    "following-updated",
                                    serde_json::json!({
                                        "id": id_copy.to_string(),
                                    }),
                                );
                            }
                            Ok((planet, Err(e))) => {
                                warn!("更新失败 {}: {}", planet.name, e);
                            }
                            Err(e) => {
                                error!("后台更新任务异常: {}", e);
                            }
                        }
                    }
                }

                info!("后台更新完成");
            }
        });
    }
}