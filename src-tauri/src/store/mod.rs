use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use anyhow::{anyhow, Result};
use tracing::{debug, error, info};
use tauri::{AppHandle, Emitter};

use crate::models::planet::{MyPlanet, FollowingPlanet};
use crate::models::article::{MyArticle, FollowingArticle};
use crate::models::draft::Draft;

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
        self.following_planets = FollowingPlanet::load_all(app)?;
        info!("Loaded {} following planets", self.following_planets.len());

        Ok(())
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

    /// 创建新的 Following Planet（Phase 4 完善 IPNS 解析逻辑）
    pub fn follow_planet(
        &mut self,
        name: String,
        about: String,
        planet_type: crate::models::planet::PlanetType,
        link: String,
        app: &AppHandle,
    ) -> Result<FollowingPlanet> {
        let planet = FollowingPlanet::create(name, about, planet_type, link, app)?;
        self.following_planets.insert(0, planet.clone());
        Ok(planet)
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

    /// 获取 Following Planet 的所有文章
    pub fn list_following_articles(&self, planet_id: Uuid, app: &AppHandle) -> Result<Vec<FollowingArticle>> {
        let planet = self.following_planets.iter().find(|p| p.id == planet_id)
            .ok_or_else(|| anyhow!("Following planet not found: {}", planet_id))?;
        FollowingArticle::load_all(planet, app)
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
}