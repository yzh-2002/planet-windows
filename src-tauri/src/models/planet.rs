use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::{anyhow, Result};
use tracing::{debug, error, info};
use tauri::AppHandle;

use crate::helpers::paths;

// ============================================================
// PlanetType 枚举
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanetType {
    Planet = 0,
    Ens = 1,
    DnsLink = 2,
    Dns = 3,
    DotBit = 4,
}

// ============================================================
// MyPlanet 结构体
// 对应原项目 MyPlanetModel.swift
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPlanet {
    pub id: Uuid,
    pub name: String,
    pub about: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    pub created: DateTime<Utc>,
    pub ipns: String,
    pub updated: DateTime<Utc>,
    pub template_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_published: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_published_cid: Option<String>,

    // 归档相关
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<DateTime<Utc>>,

    // 社交媒体
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mastodon_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord_link: Option<String>,

    // 第三方服务（Phase 3+ 实现）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filebase_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filebase_pin_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filebase_api_token: Option<String>,

    // 其他配置（Phase 3+ 实现）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_not_index: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prewarm_new_post: Option<bool>,
}

impl MyPlanet {
    // ============================================================
    // 路径方法
    // ============================================================

    /// My Planets 根目录
    pub fn my_planets_path(app: &AppHandle) -> PathBuf {
        let path = paths::get_data_path(app).join("My");
        fs::create_dir_all(&path).ok();
        path
    }

    /// 当前 Planet 的基础路径
    pub fn base_path(&self, app: &AppHandle) -> PathBuf {
        Self::my_planets_path(app).join(self.id.to_string())
    }

    /// planet.json 文件路径
    pub fn info_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("planet.json")
    }

    /// Articles 目录路径
    pub fn articles_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("Articles")
    }

    /// Drafts 目录路径
    pub fn drafts_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("Drafts")
    }

    /// Avatar 图片路径
    pub fn avatar_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("avatar.png")
    }

    /// Favicon 路径
    pub fn favicon_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("favicon.ico")
    }

    // ============================================================
    // CRUD 操作
    // ============================================================

    /// 创建新的 Planet
    pub fn create(name: String, about: String, template_name: String, app: &AppHandle) -> Result<Self> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        // 生成 IPNS key（Phase 1 中已实现）
        // 这里先使用占位符，Phase 3 发布时会生成真正的 IPNS key
        let ipns = format!("k51qzi5uqu5dibstm2yxidly22jx94embd7j3xjstfk65ulictn2ajnjvpiac7");

        let planet = Self {
            id,
            name,
            about,
            domain: None,
            author_name: None,
            created: now,
            ipns,
            updated: now,
            template_name,
            last_published: None,
            last_published_cid: None,
            archived: Some(false),
            archived_at: None,
            twitter_username: None,
            github_username: None,
            telegram_username: None,
            mastodon_username: None,
            discord_link: None,
            filebase_enabled: None,
            filebase_pin_name: None,
            filebase_api_token: None,
            do_not_index: Some(false),
            prewarm_new_post: Some(true),
        };

        // 创建目录结构
        fs::create_dir_all(planet.articles_path(app))?;
        fs::create_dir_all(planet.drafts_path(app))?;

        // 保存到磁盘
        planet.save(app)?;

        info!("Created new planet: {} ({})", planet.name, planet.id);
        Ok(planet)
    }

    /// 从磁盘加载 Planet
    pub fn load(planet_id: Uuid, app: &AppHandle) -> Result<Self> {
        let base_path = Self::my_planets_path(app).join(planet_id.to_string());
        let info_path = base_path.join("planet.json");

        if !info_path.exists() {
            return Err(anyhow!("Planet not found: {}", planet_id));
        }

        let content = fs::read_to_string(&info_path)?;
        let mut planet: Self = serde_json::from_str(&content)?;

        // 验证路径一致性
        if planet.id != planet_id {
            return Err(anyhow!("Planet ID mismatch"));
        }

        Ok(planet)
    }

    /// 从目录加载所有 My Planets
    pub fn load_all(app: &AppHandle) -> Result<Vec<Self>> {
        let my_planets_path = Self::my_planets_path(app);
        let mut planets = Vec::new();

        if !my_planets_path.exists() {
            return Ok(planets);
        }

        for entry in fs::read_dir(&my_planets_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(planet_id) = Uuid::parse_str(dir_name) {
                        match Self::load(planet_id, app) {
                            Ok(planet) => planets.push(planet),
                            Err(e) => {
                                error!("Failed to load planet {}: {}", planet_id, e);
                            }
                        }
                    }
                }
            }
        }

        planets.sort_by(|a, b| b.updated.cmp(&a.updated));
        Ok(planets)
    }

    /// 保存 Planet 到磁盘
    pub fn save(&self, app: &AppHandle) -> Result<()> {
        let info_path = self.info_path(app);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&info_path, content)?;
        debug!("Saved planet: {}", self.id);
        Ok(())
    }

    /// 更新 Planet
    pub fn update<F>(&mut self, f: F, app: &AppHandle) -> Result<()>
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated = Utc::now();
        self.save(app)?;
        Ok(())
    }

    /// 删除 Planet（包括所有文章和草稿）
    pub fn delete(&self, app: &AppHandle) -> Result<()> {
        let base_path = self.base_path(app);
        if base_path.exists() {
            fs::remove_dir_all(&base_path)?;
            info!("Deleted planet: {} ({})", self.name, self.id);
        }
        Ok(())
    }
}

// ============================================================
// FollowingPlanet 结构体
// 对应原项目 FollowingPlanetModel.swift
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowingPlanet {
    pub id: Uuid,
    pub name: String,
    pub about: String,
    pub created: DateTime<Utc>,
    pub planet_type: PlanetType,
    pub link: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
    pub updated: DateTime<Utc>,
    pub last_retrieved: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address_resolved_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mastodon_username: Option<String>,
}

impl FollowingPlanet {
    /// Following Planets 根目录
    pub fn following_planets_path(app: &AppHandle) -> PathBuf {
        let path = paths::get_data_path(app).join("Following");
        fs::create_dir_all(&path).ok();
        path
    }

    /// 当前 Planet 的基础路径
    pub fn base_path(&self, app: &AppHandle) -> PathBuf {
        Self::following_planets_path(app).join(self.id.to_string())
    }

    /// planet.json 文件路径
    pub fn info_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("planet.json")
    }

    /// Articles 目录路径
    pub fn articles_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("Articles")
    }

    /// Avatar 图片路径
    pub fn avatar_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("avatar.png")
    }

    /// 创建新的 Following Planet
    pub fn create(
        name: String,
        about: String,
        planet_type: PlanetType,
        link: String,
        app: &AppHandle,
    ) -> Result<Self> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let planet = Self {
            id,
            name,
            about,
            created: now,
            planet_type,
            link,
            cid: None,
            updated: now,
            last_retrieved: now,
            archived: Some(false),
            archived_at: None,
            wallet_address: None,
            wallet_address_resolved_at: None,
            twitter_username: None,
            github_username: None,
            telegram_username: None,
            mastodon_username: None,
        };

        // 创建目录结构
        fs::create_dir_all(planet.articles_path(app))?;

        // 保存到磁盘
        planet.save(app)?;

        info!("Created new following planet: {} ({})", planet.name, planet.id);
        Ok(planet)
    }

    /// 从磁盘加载 Following Planet
    pub fn load(planet_id: Uuid, app: &AppHandle) -> Result<Self> {
        let base_path = Self::following_planets_path(app).join(planet_id.to_string());
        let info_path = base_path.join("planet.json");

        if !info_path.exists() {
            return Err(anyhow!("Following planet not found: {}", planet_id));
        }

        let content = fs::read_to_string(&info_path)?;
        let mut planet: Self = serde_json::from_str(&content)?;

        if planet.id != planet_id {
            return Err(anyhow!("Planet ID mismatch"));
        }

        Ok(planet)
    }

    /// 从目录加载所有 Following Planets
    pub fn load_all(app: &AppHandle) -> Result<Vec<Self>> {
        let following_planets_path = Self::following_planets_path(app);
        let mut planets = Vec::new();

        if !following_planets_path.exists() {
            return Ok(planets);
        }

        for entry in fs::read_dir(&following_planets_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(planet_id) = Uuid::parse_str(dir_name) {
                        match Self::load(planet_id, app) {
                            Ok(planet) => planets.push(planet),
                            Err(e) => {
                                error!("Failed to load following planet {}: {}", planet_id, e);
                            }
                        }
                    }
                }
            }
        }

        planets.sort_by(|a, b| b.updated.cmp(&a.updated));
        Ok(planets)
    }

    /// 保存 Following Planet 到磁盘
    pub fn save(&self, app: &AppHandle) -> Result<()> {
        let info_path = self.info_path(app);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&info_path, content)?;
        debug!("Saved following planet: {}", self.id);
        Ok(())
    }

    /// 更新 Following Planet
    pub fn update<F>(&mut self, f: F, app: &AppHandle) -> Result<()>
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated = Utc::now();
        self.save(app)?;
        Ok(())
    }

    /// 删除 Following Planet
    pub fn delete(&self, app: &AppHandle) -> Result<()> {
        let base_path = self.base_path(app);
        if base_path.exists() {
            fs::remove_dir_all(&base_path)?;
            info!("Deleted following planet: {} ({})", self.name, self.id);
        }
        Ok(())
    }
}

// ============================================================
// PublicPlanet 结构体（用于模板渲染）
// Phase 3 发布功能中使用
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicPlanet {
    pub id: Uuid,
    pub name: String,
    pub about: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_username: Option<String>,
    // ... 其他公开字段
}

impl From<&MyPlanet> for PublicPlanet {
    fn from(planet: &MyPlanet) -> Self {
        Self {
            id: planet.id,
            name: planet.name.clone(),
            about: planet.about.clone(),
            author_name: planet.author_name.clone(),
            created: planet.created,
            updated: planet.updated,
            twitter_username: planet.twitter_username.clone(),
            github_username: planet.github_username.clone(),
        }
    }
}