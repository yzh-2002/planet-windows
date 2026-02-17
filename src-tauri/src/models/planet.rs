use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::{anyhow, Context, Result};
use tracing::{debug, error, info, warn};
use tauri::{AppHandle, Emitter};

use crate::helpers::paths;
use crate::helpers::markdown::render_markdown_html;
use crate::template::Template;
use crate::ipfs::daemon::IpfsDaemon;
use crate::keystore::Keystore;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipns: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filebase_request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filebase_pin_cid: Option<String>,

    // Pinnable 集成
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinnable_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinnable_api_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinnable_pin_cid: Option<String>,

    // 其他配置（Phase 3+ 实现）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
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

    /// Planet 的 public 根目录
    /// 
    /// **注意**：实际实现需要传入 `app: &AppHandle` 参数。
    pub fn public_base_path(&self, app: &AppHandle) -> PathBuf {
        let public_path = paths::get_data_path(app).join("Public");
        std::fs::create_dir_all(&public_path).ok();
        public_path.join(self.id.to_string())  // id 是 Uuid，需要 to_string()
    }

    /// 检查是否有 avatar
    /// 
    /// **注意**：实际实现需要传入 `app: &AppHandle` 参数。
    pub fn has_avatar(&self, app: &AppHandle) -> bool {
        self.avatar_path(app).exists()
    }

    /// 生成公开 Planet 数据（包含文章列表）
    /// 
    /// **注意**：实际代码中已有 `From<&MyPlanet> for PublicPlanet` 实现，
    /// 但这个方法用于生成包含文章列表的版本（用于写入 planet.json）。
    /// 在模板渲染时，articles 会单独传入 context。
    pub fn to_public(&self, _articles: &[crate::models::article::PublicArticle]) -> PublicPlanet {
        // 注意：PublicPlanet 结构体中不包含 articles 字段，
        // articles 在模板渲染时会单独传入 context
        PublicPlanet::from(self)
    }

    /// 生成 serde_json::Value 版本 (用于模板渲染 context)
    pub fn to_public_value(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id.to_string(),
            "name": self.name,
            "about": self.about,
            "ipns": self.ipns.as_ref(),
            "created": self.created.to_rfc3339(),
            "updated": self.updated.to_rfc3339(),
        })
    }

    // ============================================================
    // CRUD 操作
    // ============================================================

    /// 创建新的 Planet
    pub fn create(name: String, about: String, template_name: String, app: &AppHandle) -> Result<Self> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let planet = Self {
            id,
            name,
            about,
            domain: None,
            author_name: None,
            created: now,
            ipns: None,  // IPNS 地址将在首次发布时生成
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
            filebase_request_id: None,
            filebase_pin_cid: None,
            pinnable_enabled: None,
            pinnable_api_endpoint: None,
            pinnable_pin_cid: None,
            tags: None,
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

    // ============================================================
    // IPNS 地址管理
    // ============================================================

    /// 获取或生成 IPNS 地址
    ///
    /// 如果 IPNS 地址已存在，直接返回
    /// 如果不存在，检查 IPFS key 是否存在：
    ///   - 如果 key 存在，从 key 获取 IPNS 地址
    ///   - 如果 key 不存在，尝试从 Keystore 恢复，然后获取 IPNS
    ///   - 如果 Keystore 也没有，生成新 key（返回 IPNS 地址）并保存
    pub fn get_or_create_ipns(
        &mut self,
        daemon: &crate::ipfs::daemon::IpfsDaemon,
        app: &AppHandle,
    ) -> Result<String> {
        // 如果已有 IPNS 地址，直接返回
        if let Some(ref ipns) = self.ipns {
            return Ok(ipns.clone());
        }

        let key_name = self.id.to_string();
        
        // 检查 key 是否存在
        let key_exists = daemon.check_key_exists(&key_name)
            .map_err(|e| anyhow::anyhow!("检查 IPFS key 失败: {}", e))?;

        let ipns_address = if key_exists {
            // Key 存在，尝试获取 IPNS 地址
            if let Some(ipns) = daemon.get_key_ipns(&key_name)
                .map_err(|e| anyhow::anyhow!("获取 key IPNS 地址失败: {}", e))?
            {
                ipns
            } else {
                // 无法获取 IPNS 地址，返回错误
                return Err(anyhow::anyhow!(
                    "Key 已存在，但无法获取 IPNS 地址。请先发布一次以生成 IPNS 地址。"
                ));
            }
        } else {
            // Key 不存在，尝试从 Keystore 恢复
            if crate::keystore::Keystore::check(&key_name) {
                // 从 Keystore 恢复 key
                crate::keystore::Keystore::import_key_from_keystore(daemon, &key_name, app)
                    .map_err(|e| anyhow::anyhow!("从 Keystore 恢复密钥失败: {}", e))?;
                
                // 恢复后，获取 IPNS 地址
                daemon.get_key_ipns(&key_name)
                    .map_err(|e| anyhow::anyhow!("获取 key IPNS 地址失败: {}", e))?
                    .ok_or_else(|| anyhow::anyhow!(
                        "Key 已从 Keystore 恢复，但无法获取 IPNS 地址。请先发布一次以生成 IPNS 地址。"
                    ))?
            } else {
                // 生成新 key，返回 IPNS 地址
                let ipns = daemon.generate_key(&key_name)
                    .map_err(|e| anyhow::anyhow!("生成 IPFS key 失败: {}", e))?;
                
                // 保存到 Keystore
                crate::keystore::Keystore::export_key_to_keystore(daemon, &key_name, app)
                    .map_err(|e| anyhow::anyhow!("保存密钥到 Keystore 失败: {}", e))?;
                
                ipns
            }
        };

        // 保存 IPNS 地址
        self.ipns = Some(ipns_address.clone());
        self.save(app)?;
        
        Ok(ipns_address)
    }

    /// 获取 IPNS 地址（如果不存在则返回 None）
    pub fn get_ipns(&self) -> Option<&String> {
        self.ipns.as_ref()
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
// PublishState 结构体（发布状态，传给前端）
// Phase 3 发布功能中使用
// ============================================================

/// 发布状态（传给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishState {
    pub planet_id: String,
    pub is_publishing: bool,
    pub step: String,               // "idle" | "saving" | "uploading" | "publishing" | "pinning" | "done" | "error"
    pub cid: Option<String>,
    pub error: Option<String>,
    pub started_at: Option<String>,
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

/// RSS 模板 (内置，对标 Planet/Templates/RSS.xml)
const RSS_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"
    xmlns:content="http://purl.org/rss/1.0/modules/content/"
    xmlns:atom="http://www.w3.org/2005/Atom"
    xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd"
    >
<channel>
    <title>{{ planet_name }}</title>
    <atom:link href="{{ root_prefix }}/rss.xml" rel="self" type="application/rss+xml" />
    <link>{{ root_prefix }}/</link>
    <description>{{ planet_about }}</description>
    {% for article in articles %}
    <item>
        <title>{{ article.title }}</title>
        <link>{{ root_prefix }}/{{ article.id }}/</link>
        <guid>{{ root_prefix }}/{{ article.id }}/</guid>
        <pubDate>{{ article.created }}</pubDate>
        <description><![CDATA[{{ article.content_rendered | default(value="") }}]]></description>
    </item>
    {% endfor %}
</channel>
</rss>"#;

impl MyPlanet {
    /// 渲染 RSS feed
    ///
    /// 对标 Swift MyPlanetModel.renderRSS(podcastOnly:)
    pub fn render_rss(&self, articles: &[super::article::PublicArticle]) -> Result<String> {
        let root_prefix = if let Some(ref domain) = self.domain {
            if !domain.is_empty() {
                format!("https://{}", domain)
            } else {
                self.ipns.as_ref()
                    .map(|ipns| format!("https://eth.sucks/ipns/{}", ipns))
                    .unwrap_or_else(|| "https://eth.sucks".to_string())
            }
        } else {
            self.ipns.as_ref()
                .map(|ipns| format!("https://eth.sucks/ipns/{}", ipns))
                .unwrap_or_else(|| "https://eth.sucks".to_string())
        };

        let mut tera = tera::Tera::default();
        crate::template::register_filters(&mut tera);  // 需要确保 register_filters 已导出
        tera.add_raw_template("rss.xml", RSS_TEMPLATE)?;

        let mut context = tera::Context::new();
        context.insert("planet_name", &self.name);
        context.insert("planet_about", &self.about);
        context.insert("root_prefix", &root_prefix);
        context.insert("articles", articles);
        
        // 注意：RSS 模板中可能需要日期格式化为 RFC 822 格式
        // 可以使用 rfc822 filter：{{ article.created | rfc822 }}

        let rss_xml = tera.render("rss.xml", &context)?;
        Ok(rss_xml)
    }

    /// 生成静态站点到 public 目录
    ///
    /// 对标 Swift MyPlanetModel.savePublic()
    /// 
    /// **注意**：实际实现需要传入 `app: &AppHandle` 参数。
    pub fn save_public(&mut self, template: &Template, app: &AppHandle) -> Result<()> {
        info!("开始生成静态站点: {}", self.name);
        let public_base = self.public_base_path(app);
        let public_assets = public_base.join("assets");

        // 确保 public 目录存在
        fs::create_dir_all(&public_base)?;
        info!("Public 目录已创建: {:?}", public_base);

        // ============================================================
        // 1. 对每篇文章执行 save_public
        // ============================================================
        // 注意：需要先加载所有文章
        info!("正在加载文章...");
        let articles = crate::models::article::MyArticle::load_all(self, app)?;
        info!("加载了 {} 篇文章", articles.len());
        
        for (i, mut article) in articles.into_iter().enumerate() {
            if let Err(e) = article.save_public(self, template, app) {
                error!("文章 '{}' 静态化失败: {}", article.title, e);
            }
            if (i + 1) % 10 == 0 {
                debug!("已处理 {} 篇文章", i + 1);
            }
        }
        info!("文章静态化完成");

        // ============================================================
        // 2. 构建公开文章列表
        // ============================================================
        let articles = crate::models::article::MyArticle::load_all(self, app)?;
        let public_articles: Vec<super::article::PublicArticle> =
            articles.iter().map(|a| super::article::PublicArticle::from(a)).collect();

        let public_planet = self.to_public(&public_articles);

        let about_html = render_markdown_html(&self.about);
        let css_hash = template.style_css_hash().unwrap_or_default();

        // ============================================================
        // 3. 渲染 RSS
        // ============================================================
        info!("正在渲染 RSS...");
        match self.render_rss(&public_articles) {
            Ok(rss_xml) => {
                let rss_path = public_base.join("rss.xml");
                fs::write(&rss_path, rss_xml.as_bytes())?;
                debug!("RSS 生成完成: {:?}", rss_path);
            }
            Err(e) => error!("RSS 渲染失败: {}", e),
        }

        // ============================================================
        // 4. 渲染 index.html (首页 + 分页)
        // ============================================================
        info!("正在渲染首页...");
        let items_per_page = template.info.ideal_items_per_page;
        let generate_pagination = template.info.generate_index_pagination;

        if generate_pagination && public_articles.len() > items_per_page {
            let total_pages = (public_articles.len() as f64 / items_per_page as f64).ceil() as usize;
            debug!("渲染 {} 页", total_pages);

            for page in 1..=total_pages {
                let start = (page - 1) * items_per_page;
                let end = std::cmp::min(page * items_per_page, public_articles.len());
                let page_articles = &public_articles[start..end];

                let mut context = tera::Context::new();
                context.insert("planet", &public_planet);
                context.insert("planet_ipns", &self.ipns.as_ref());
                context.insert("has_avatar", &self.has_avatar(app));
                context.insert("page_title", &self.name);
                context.insert("page_description", &self.about);
                context.insert("page_description_html", &about_html);
                context.insert("articles", &page_articles);
                context.insert("assets_prefix", "./");
                context.insert("style_css_sha256", &css_hash);
                context.insert("build_timestamp", &chrono::Utc::now().timestamp());
                context.insert("current_page", &page);
                context.insert("total_pages", &total_pages);

                // next/previous page
                if page < total_pages {
                    context.insert("next_page", &format!("page{}.html", page + 1));
                }
                if page > 1 {
                    context.insert("previous_page", &format!("page{}.html", page - 1));
                }

                let page_html = template.render_index(&context)
                    .with_context(|| format!("渲染第 {} 页失败", page))?;
                let page_path = public_base.join(format!("page{}.html", page));
                fs::write(&page_path, page_html.as_bytes())
                    .with_context(|| format!("写入第 {} 页失败: {:?}", page, page_path))?;

                // 第一页同时写入 index.html
                if page == 1 {
                    let index_path = public_base.join("index.html");
                    fs::write(&index_path, page_html.as_bytes())
                        .with_context(|| format!("写入 index.html 失败: {:?}", index_path))?;
                }
            }
        } else {
            // 不分页，直接渲染单个 index.html
            let mut context = tera::Context::new();
            context.insert("planet", &public_planet);
            context.insert("planet_ipns", &self.ipns);
            context.insert("has_avatar", &self.has_avatar(app));
            context.insert("page_title", &self.name);
            context.insert("page_description", &self.about);
            context.insert("page_description_html", &about_html);
            context.insert("articles", &public_articles);
            context.insert("assets_prefix", "./");
            context.insert("style_css_sha256", &css_hash);
            context.insert("build_timestamp", &chrono::Utc::now().timestamp());

            let index_html = template.render_index(&context)
                .with_context(|| "渲染 index.html 失败")?;
            let index_path = public_base.join("index.html");
            fs::write(&index_path, index_html.as_bytes())
                .with_context(|| format!("写入 index.html 失败: {:?}", index_path))?;

            let page1_path = public_base.join("page1.html");
            fs::write(&page1_path, index_html.as_bytes())
                .with_context(|| format!("写入 page1.html 失败: {:?}", page1_path))?;
        }

        // ============================================================
        // 5. 渲染 tags 页面 (可选)
        // ============================================================
        if template.info.generate_tag_pages && template.has_tags_html() {
            let mut tag_articles: HashMap<String, Vec<&super::article::PublicArticle>> = HashMap::new();

            for article in &public_articles {
                // 注意：PublicArticle 的 tags 是 HashMap<String, String>，不是 Option
                for key in article.tags.keys() {
                    tag_articles
                        .entry(key.clone())
                        .or_insert_with(Vec::new)
                        .push(article);
                }
            }

            // 渲染每个标签页面
            for (tag_key, articles) in &tag_articles {
                let tag_value = self.tags.as_ref()
                    .and_then(|t| t.get(tag_key))
                    .cloned()
                    .unwrap_or_else(|| tag_key.clone());

                let mut context = tera::Context::new();
                context.insert("planet", &public_planet);
                context.insert("planet_ipns", &self.ipns.as_ref());
                context.insert("has_avatar", &self.has_avatar(app));
                context.insert("tag_key", tag_key);
                context.insert("tag_value", &tag_value);
                context.insert("current_item_type", "tags");
                context.insert("articles", articles);
                context.insert("page_title", &format!("{} - {}", self.name, tag_value));
                context.insert("assets_prefix", "./");
                context.insert("style_css_sha256", &css_hash);
                context.insert("build_timestamp", &chrono::Utc::now().timestamp());

                if let Ok(tag_html) = template.render_index(&context) {
                    let tag_path = public_base.join(format!("{}.html", tag_key));
                    let _ = fs::write(&tag_path, tag_html.as_bytes());
                }
            }

            // 渲染 tags 汇总页 (tags.html)
            let mut tags_context = tera::Context::new();
            tags_context.insert("planet", &public_planet);
            tags_context.insert("planet_ipns", &self.ipns);
            tags_context.insert("has_avatar", &self.has_avatar(app));
            tags_context.insert("tags", &self.tags);
            tags_context.insert("assets_prefix", "./");
            tags_context.insert("style_css_sha256", &css_hash);

            if let Ok(tags_html) = template.render_tags(&tags_context) {
                let tags_path = public_base.join("tags.html");
                let _ = fs::write(&tags_path, tags_html.as_bytes());
            }

            debug!("Tags 页面渲染完成");
        }

        // ============================================================
        // 6. 渲染 archive 页面 (可选)
        // ============================================================
        if template.info.generate_archive && template.has_archive_html() {
            let mut archive: HashMap<String, Vec<&super::article::PublicArticle>> = HashMap::new();
            let mut archive_sections: Vec<String> = Vec::new();

            for article in &public_articles {
                // 按月份分组 (格式: "January 2025")
                let month_year = article.created.format("%B %Y").to_string();

                if !archive.contains_key(&month_year) {
                    archive_sections.push(month_year.clone());
                }
                archive.entry(month_year).or_insert_with(Vec::new).push(article);
            }

            archive_sections.sort();

            let mut context = tera::Context::new();
            context.insert("planet", &public_planet);
            context.insert("planet_ipns", &self.ipns);
            context.insert("has_avatar", &self.has_avatar(app));
            context.insert("articles", &public_articles);
            context.insert("archive", &archive);
            context.insert("archive_sections", &archive_sections);
            context.insert("assets_prefix", "./");
            context.insert("style_css_sha256", &css_hash);

            if let Ok(archive_html) = template.render_archive(&context) {
                let archive_path = public_base.join("archive.html");
                let _ = fs::write(&archive_path, archive_html.as_bytes());
            }

            debug!("Archive 页面渲染完成");
        }

        // ============================================================
        // 7. 写入 planet.json
        // ============================================================
        let planet_json = serde_json::to_string_pretty(&public_planet)?;
        let info_path = public_base.join("planet.json");
        fs::write(&info_path, planet_json.as_bytes())
            .with_context(|| format!("写入 planet.json 失败: {:?}", info_path))?;

        // ============================================================
        // 8. 复制模板 assets
        // ============================================================
        info!("正在复制模板 assets...");
        if public_assets.exists() {
            fs::remove_dir_all(&public_assets)?;
        }
        copy_dir_recursive(&template.assets_path(), &public_assets)
            .with_context(|| format!("复制模板 assets 失败: {:?} -> {:?}", template.assets_path(), public_assets))?;
        info!("模板 assets 复制完成");

        // ============================================================
        // 9. 复制 avatar (如果存在)
        // ============================================================
        let avatar_src = self.avatar_path(app);
        let avatar_dst = public_base.join("avatar.png");
        if avatar_src.exists() && !avatar_dst.exists() {
            let _ = fs::copy(&avatar_src, &avatar_dst);
        }

        // ============================================================
        // 10. 保存 robots.txt
        // ============================================================
        let robots_txt = if self.do_not_index.unwrap_or(false) {
            "User-agent: *\nDisallow: /"
        } else {
            ""
        };
        let robots_path = public_base.join("robots.txt");
        fs::write(&robots_path, robots_txt.as_bytes())
            .with_context(|| format!("写入 robots.txt 失败: {:?}", robots_path))?;

        info!("Planet '{}' 站点生成完成", self.name);
        Ok(())
    }

    /// 完整发布流程
    ///
    /// 对标 Swift MyPlanetModel.publish()
    /// 
    /// **注意**：实际实现需要传入 `app: &AppHandle` 参数用于内部调用。
    pub async fn publish(
        &mut self,
        daemon: &IpfsDaemon,
        template: &Template,
        app_handle: &AppHandle,
        app: &AppHandle,
    ) -> Result<()> {
        let publish_started_at = chrono::Utc::now();

        // 通知前端：发布开始
        self.emit_publish_state(app_handle, "saving", None, None, &publish_started_at);

        // ============================================================
        // 1. 检查 IPFS key 是否存在
        // ============================================================
        // 注意：check_key_exists 是同步方法，不是异步的
        let key_name = self.id.to_string();  // Uuid 转换为 String
        let key_exists = daemon.check_key_exists(&key_name)?;
        if !key_exists {
            info!("IPFS key '{}' 不存在，尝试从 Keystore 恢复", key_name);
            Keystore::import_key_from_keystore(daemon, &key_name, app)?;
        }

        // ============================================================
        // 2. save_public() — 生成静态站点
        // ============================================================
        self.save_public(template, app)?;

        self.emit_publish_state(app_handle, "uploading", None, None, &publish_started_at);

        // ============================================================
        // 3. add_directory → CID
        // ============================================================
        // 注意：add_directory 是同步方法，不是异步的
        let public_path = self.public_base_path(app);
        let cid = daemon.add_directory(public_path.to_str().unwrap())?;  // 需要转换为 &str

        if cid.is_empty() {
            self.emit_publish_state(app_handle, "error", None, Some("add_directory 返回空 CID"), &publish_started_at);
            anyhow::bail!("发布失败：add_directory 返回空 CID");
        }

        info!("Planet '{}' 上传完成，CID: {}", self.name, cid);
        self.emit_publish_state(app_handle, "publishing", Some(&cid), None, &publish_started_at);

        // ============================================================
        // 4. name/publish → IPNS
        // ============================================================
        // 注意：api 方法接受 Option<&HashMap<String, String>>
        let mut args = HashMap::new();
        args.insert("arg".to_string(), cid.clone());
        args.insert("allow-offline".to_string(), "1".to_string());
        args.insert("key".to_string(), self.id.to_string());  // id 是 Uuid，需要转换
        args.insert("quieter".to_string(), "1".to_string());
        args.insert("lifetime".to_string(), "7200h".to_string());
        
        let publish_result = daemon.api(
            "name/publish",
            Some(&args),
            Some(180),  // 超时 180 秒
        ).await;

        match publish_result {
            Ok(_) => {
                info!("Planet '{}' IPNS 发布成功: {}", self.name, cid);
            }
            Err(e) => {
                error!("IPNS 发布失败: {}", e);
                // IPNS 发布失败不阻塞整体流程，CID 已经可用
            }
        }

        // ============================================================
        // 5. 更新 Planet 状态
        // ============================================================
        if self.last_published_cid.as_deref() != Some(&cid) {
            self.last_published = Some(chrono::Utc::now());
            self.last_published_cid = Some(cid.clone());
            self.save(app)?;  // 使用已有的 save 方法
        }

        // ============================================================
        // 6. Filebase pin (可选, 异步)
        // ============================================================
        // TODO: Phase 3+ - 实现 Filebase 集成模块
        // #[cfg(feature = "integrations")]
        // if let Some(true) = self.filebase_enabled {
        //     if let (Some(ref pin_name), Some(ref api_token)) =
        //         (&self.filebase_pin_name, &self.filebase_api_token)
        //     {
        //         let should_pin = match &self.filebase_pin_cid {
        //             Some(existing_cid) => existing_cid.is_empty() || existing_cid != &cid,
        //             None => true,
        //         };
        //
        //         if should_pin {
        //             self.emit_publish_state(app_handle, "pinning", Some(&cid), None, &publish_started_at);
        //             let filebase = crate::integrations::filebase::Filebase::new(
        //                 pin_name.clone(),
        //                 api_token.clone(),
        //             );
        //             match filebase.pin(&cid).await {
        //                 Ok(Some(request_id)) => {
        //                     self.filebase_request_id = Some(request_id);
        //                     self.filebase_pin_cid = Some(cid.clone());
        //                     self.save(app)?;
        //                     info!("Filebase pin 成功");
        //                 }
        //                 Ok(None) => {
        //                     warn!("Filebase pin 未返回 request ID");
        //                 }
        //                 Err(e) => {
        //                     error!("Filebase pin 失败: {}", e);
        //                 }
        //             }
        //         }
        //     }
        // }

        // ============================================================
        // 7. Pinnable pin (可选, 异步)
        // ============================================================
        // TODO: Phase 3+ - 实现 Pinnable 集成模块
        // #[cfg(feature = "integrations")]
        // if let Some(true) = self.pinnable_enabled {
        //     if let Some(ref api_endpoint) = self.pinnable_api_endpoint {
        //         let pinnable = crate::integrations::pinnable::Pinnable::new(api_endpoint.clone());
        //         match pinnable.pin(&cid).await {
        //             Ok(_) => {
        //                 self.pinnable_pin_cid = Some(cid.clone());
        //                 self.save(app)?;
        //                 info!("Pinnable pin 成功");
        //             }
        //             Err(e) => {
        //                 error!("Pinnable pin 失败: {}", e);
        //             }
        //         }
        //     }
        // }

        // ============================================================
        // 8. 完成
        // ============================================================
        self.emit_publish_state(app_handle, "done", Some(&cid), None, &publish_started_at);

        info!("Planet '{}' 发布流程完成", self.name);
        Ok(())
    }

    /// 向前端发送发布状态更新
    pub fn emit_publish_state(
        &self,
        app_handle: &AppHandle,
        step: &str,
        cid: Option<&str>,
        error: Option<&str>,
        started_at: &DateTime<Utc>,
    ) {
        let is_publishing = step != "done" && step != "error";
        let state = PublishState {
            planet_id: self.id.to_string(),  // Uuid 转换为 String
            is_publishing,
            step: step.to_string(),
            cid: cid.map(|s| s.to_string()),
            error: error.map(|s| s.to_string()),
            started_at: Some(started_at.to_rfc3339()),  // DateTime 转换为 String
        };
        let _ = app_handle.emit("publish-state-changed", &state);
    }
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}