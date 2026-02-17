use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use tauri::AppHandle;
use crate::helpers::paths;
use crate::models::following_article::FollowingArticle;
use crate::helpers::dotbit::{self, DWebRecordType};
use crate::helpers::ens;
use crate::helpers::feed;
use crate::ipfs::daemon::IpfsDaemon;

// ============================================================
// PlanetType 枚举
// ============================================================

/// 关注来源类型
///
/// 对标 Swift PlanetType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PlanetType {
    #[serde(rename = "planet")]
    Planet = 0,     // IPNS 原生 Planet
    #[serde(rename = "ens")]
    Ens = 1,        // ENS 域名
    #[serde(rename = "dnslink")]
    DnsLink = 2,    // DNSLink
    #[serde(rename = "dns")]
    Dns = 3,        // HTTP RSS/Atom Feed
    #[serde(rename = "dotbit")]
    DotBit = 4,     // .bit 域名
}

// 为了兼容原项目的 Int 编码
impl From<u8> for PlanetType {
    fn from(v: u8) -> Self {
        match v {
            0 => PlanetType::Planet,
            1 => PlanetType::Ens,
            2 => PlanetType::DnsLink,
            3 => PlanetType::Dns,
            4 => PlanetType::DotBit,
            _ => PlanetType::Planet,
        }
    }
}

// ============================================================
// FollowingPlanet 数据模型
// ============================================================

/// 关注的 Planet 数据模型
///
/// 对标 Swift FollowingPlanetModel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowingPlanet {
    pub id: Uuid,
    pub planet_type: PlanetType,
    pub name: String,
    pub about: String,
    pub link: String,
    pub cid: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub last_retrieved: DateTime<Utc>,

    #[serde(default)]
    pub archived: Option<bool>,
    pub archived_at: Option<DateTime<Utc>>,

    pub wallet_address: Option<String>,
    pub wallet_address_resolved_at: Option<DateTime<Utc>>,

    // social usernames
    pub twitter_username: Option<String>,
    pub github_username: Option<String>,
    pub telegram_username: Option<String>,
    pub mastodon_username: Option<String>,

    // juicebox
    #[serde(default)]
    pub juicebox_enabled: Option<bool>,
    pub juicebox_project_id: Option<i64>,
    pub juicebox_project_id_goerli: Option<i64>,

    // ---- 运行时字段 (不序列化) ----
    #[serde(skip)]
    pub is_updating: bool,
    #[serde(skip)]
    pub articles: Vec<FollowingArticle>,
}

impl FollowingPlanet {
    // ============================================================
    // 路径工具
    // ============================================================

    /// Following 数据根目录
    pub fn following_planets_path(app: &AppHandle) -> PathBuf {
        let path = paths::get_data_path(app).join("Following");
        let _ = fs::create_dir_all(&path);
        path
    }

    /// 本 Planet 的基础目录
    pub fn base_path(&self, app: &AppHandle) -> PathBuf {
        Self::following_planets_path(app).join(self.id.to_string())
    }

    /// planet.json 路径
    pub fn info_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("planet.json")
    }

    /// Articles 子目录
    pub fn articles_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("Articles")
    }

    /// 头像路径
    pub fn avatar_path(&self, app: &AppHandle) -> PathBuf {
        self.base_path(app).join("avatar.png")
    }

    // ============================================================
    // 构造函数
    // ============================================================

    pub fn new(
        planet_type: PlanetType,
        name: String,
        about: String,
        link: String,
        cid: Option<String>,
    ) -> Self {
        let now = Utc::now();
        FollowingPlanet {
            id: Uuid::new_v4(),
            planet_type,
            name,
            about,
            link,
            cid,
            created: now,
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
            juicebox_enabled: Some(false),
            juicebox_project_id: None,
            juicebox_project_id_goerli: None,
            is_updating: false,
            articles: Vec::new(),
        }
    }

    /// 从 PublicPlanetModel 数据构建
    pub fn from_public_planet(
        planet_type: PlanetType,
        link: &str,
        cid: &str,
        public: &PublicPlanetInfo,
    ) -> Self {
        let mut planet = Self::new(
            planet_type,
            public.name.clone(),
            public.about.clone(),
            link.to_string(),
            Some(cid.to_string()),
        );
        planet.created = public.created;
        planet.updated = public.updated;
        planet.twitter_username = public.twitter_username.clone();
        planet.github_username = public.github_username.clone();
        planet.telegram_username = public.telegram_username.clone();
        planet.mastodon_username = public.mastodon_username.clone();
        planet.juicebox_enabled = public.juicebox_enabled;
        planet.juicebox_project_id = public.juicebox_project_id;
        planet.juicebox_project_id_goerli = public.juicebox_project_id_goerli;
        planet
    }

    // ============================================================
    // 持久化
    // ============================================================

    /// 保存到磁盘
    ///
    /// 对标 Swift FollowingPlanetModel.save()
    pub fn save(&self, app: &AppHandle) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(self.info_path(app), data)?;
        Ok(())
    }

    /// 从目录加载
    ///
    /// 对标 Swift FollowingPlanetModel.load(from:)
    pub fn load(dir: &Path) -> Result<Self> {
        let dir_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("无效的目录名"))?;
        let planet_id = Uuid::parse_str(dir_name)
            .with_context(|| format!("目录名不是有效的 UUID: {}", dir_name))?;

        let planet_path = dir.join("planet.json");
        let data = fs::read_to_string(&planet_path)
            .with_context(|| format!("读取 planet.json 失败: {:?}", planet_path))?;
        let mut planet: FollowingPlanet = serde_json::from_str(&data)
            .with_context(|| format!("解析 planet.json 失败: {:?}", planet_path))?;

        if planet.id != planet_id {
            anyhow::bail!(
                "目录名 {} 与 planet.json 中的 id {} 不匹配",
                dir_name,
                planet.id
            );
        }

        // 加载文章
        let articles_dir = dir.join("Articles");
        if articles_dir.exists() {
            let mut articles = Vec::new();
            for entry in fs::read_dir(&articles_dir)?.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    match FollowingArticle::load(&path) {
                        Ok(article) => articles.push(article),
                        Err(e) => warn!("加载文章失败 {:?}: {}", path, e),
                    }
                }
            }
            articles.sort_by(|a, b| b.created.cmp(&a.created));
            planet.articles = articles;
        }

        Ok(planet)
    }

    /// 删除 Planet 及其所有数据
    pub fn delete(&self, app: &AppHandle) -> Result<()> {
        let path = self.base_path(app);
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        info!("已删除 Following Planet: {} ({})", self.name, self.id);
        Ok(())
    }

    /// 确保目录结构存在
    pub fn ensure_directories(&self, app: &AppHandle) -> Result<()> {
        fs::create_dir_all(self.base_path(app))?;
        fs::create_dir_all(self.articles_path(app))?;
        Ok(())
    }

    // ============================================================
    // 文章管理
    // ============================================================

    /// 增量更新文章列表
    ///
    /// 对标 Swift FollowingPlanetModel.updateArticles()
    ///
    /// - `delete`: 如果为 true，删除远端不存在的文章（原生 Planet 用）；
    ///             如果为 false，仅追加新文章（Feed 用，因为 Feed 会滚动掉旧条目）
    pub fn update_articles(
        &mut self,
        public_articles: &[PublicArticleInfo],
        delete: bool,
        app: &AppHandle,
    ) -> Result<Vec<FollowingArticle>> {
        // 预先计算 articles_path，避免后续借用冲突
        let articles_path = self.articles_path(app);
        let planet_type = self.planet_type;

        // 构建现有文章的 link → index 映射
        let mut existing_links: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for (i, article) in self.articles.iter().enumerate() {
            existing_links.insert(article.link.clone(), i);
        }

        let mut new_articles: Vec<FollowingArticle> = Vec::new();
        let mut seen_links: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for pub_article in public_articles {
            let link = sanitize_link(&pub_article.link);
            seen_links.insert(link.clone());

            if let Some(&idx) = existing_links.get(&link) {
                // 更新已有文章
                let article = &mut self.articles[idx];
                article.title = pub_article.title.clone();
                article.content = pub_article.content.clone();
                article.audio_filename = pub_article.audio_filename.clone();
                article.video_filename = pub_article.video_filename.clone();
                article.attachments = pub_article.attachments.clone();
                article.update_summary(planet_type);
                article.save(&articles_path)?;
            } else {
                // 新文章
                let article = FollowingArticle::from_public_article(pub_article, planet_type);
                article.save(&articles_path)?;
                new_articles.push(article.clone());
                self.articles.push(article);
            }
        }

        // 删除远端不存在的文章
        if delete {
            let mut to_remove = Vec::new();
            for (i, article) in self.articles.iter().enumerate() {
                if !seen_links.contains(&article.link) {
                    article.delete(&articles_path);
                    to_remove.push(i);
                }
            }
            // 从后往前删，避免索引移位
            for i in to_remove.into_iter().rev() {
                self.articles.remove(i);
            }
        }

        // 排序：最新在前
        self.articles.sort_by(|a, b| b.created.cmp(&a.created));

        info!(
            "更新文章完成: {} 篇新文章, 总共 {} 篇",
            new_articles.len(),
            self.articles.len()
        );

        Ok(new_articles)
    }

    // ============================================================
    // 头像管理
    // ============================================================

    /// 从 IPFS Gateway 下载头像
    pub async fn download_avatar(&self, gateway: &str, app: &AppHandle) -> Result<()> {
        if let Some(ref cid) = self.cid {
            let avatar_url = format!("{}/ipfs/{}/avatar.png", gateway, cid);
            let resp = reqwest::get(&avatar_url).await?;
            if resp.status().is_success() {
                let bytes = resp.bytes().await?;
                if !bytes.is_empty() {
                    fs::write(self.avatar_path(app), &bytes)?;
                    info!("已下载头像: {} ({})", self.name, self.id);
                }
            }
        }
        Ok(())
    }

    /// 头像是否存在
    pub fn has_avatar(&self, app: &AppHandle) -> bool {
        self.avatar_path(app).exists()
    }
}

// ============================================================
// 公共 Planet 信息 (从 planet.json 反序列化)
// ============================================================

/// 远端 planet.json 的结构
///
/// 对标 Swift PublicPlanetModel (用于反序列化远端数据)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicPlanetInfo {
    pub id: Uuid,
    pub name: String,
    pub about: String,
    pub ipns: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub articles: Vec<PublicArticleInfo>,

    pub twitter_username: Option<String>,
    pub github_username: Option<String>,
    pub telegram_username: Option<String>,
    pub mastodon_username: Option<String>,

    pub juicebox_enabled: Option<bool>,
    pub juicebox_project_id: Option<i64>,
    pub juicebox_project_id_goerli: Option<i64>,

    pub tags: Option<std::collections::HashMap<String, String>>,
}

/// 远端文章信息
///
/// 对标 Swift PublicArticleModel (用于反序列化远端数据)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicArticleInfo {
    pub id: Uuid,
    pub link: String,
    pub title: String,
    pub content: String,
    pub content_rendered: Option<String>,
    pub created: DateTime<Utc>,
    pub has_video: Option<bool>,
    pub video_filename: Option<String>,
    pub has_audio: Option<bool>,
    pub audio_filename: Option<String>,
    pub audio_duration: Option<i64>,
    pub audio_byte_length: Option<i64>,
    pub attachments: Option<Vec<String>>,
    pub hero_image: Option<String>,
    pub tags: Option<std::collections::HashMap<String, String>>,
}

// ============================================================
// 工具函数
// ============================================================

/// 清理文章链接
///
/// 对标 Swift 中多处的 link 清理逻辑
fn sanitize_link(link: &str) -> String {
    let mut s = link.to_string();
    // 移除内部网关前缀
    if s.starts_with("http://127.0.0.1:") && s.len() > 22 {
        s = s[22..].to_string();
    }
    // 移除 /ipfs/Qm... 前缀
    if s.starts_with("/ipfs/Qm") && s.len() > 52 {
        s = s[52..].to_string();
    }
    s
}

/// 去重文章列表
///
/// 对标 Swift FollowingPlanetModel.deduplicate()
pub fn deduplicate_articles(articles: Vec<PublicArticleInfo>) -> Vec<PublicArticleInfo> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for article in articles {
        let link = sanitize_link(&article.link);
        if seen.insert(link) {
            result.push(article);
        }
    }
    result
}

/// 发送给前端的 FollowingPlanet 快照
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowingPlanetSnapshot {
    pub id: String,
    pub planet_type: u8,
    pub name: String,
    pub about: String,
    pub link: String,
    pub cid: Option<String>,
    pub created: String,
    pub updated: String,
    pub last_retrieved: String,
    pub is_updating: bool,
    pub article_count: usize,
    pub unread_count: usize,
    pub has_avatar: bool,
}

impl FollowingPlanet {
    pub fn snapshot(&self, app: &AppHandle) -> FollowingPlanetSnapshot {
        FollowingPlanetSnapshot {
            id: self.id.to_string(),
            planet_type: self.planet_type as u8,
            name: self.name.clone(),
            about: self.about.clone(),
            link: self.link.clone(),
            cid: self.cid.clone(),
            created: self.created.to_rfc3339(),
            updated: self.updated.to_rfc3339(),
            last_retrieved: self.last_retrieved.to_rfc3339(),
            is_updating: self.is_updating,
            article_count: self.articles.len(),
            unread_count: self.articles.iter().filter(|a| a.read.is_none()).count(),
            has_avatar: self.has_avatar(app),
        }
    }
}

impl FollowingPlanet {
    // ============================================================
    // 关注入口路由
    // ============================================================

    /// 关注入口 — 根据链接类型路由到不同的 follow 方法
    ///
    /// 对标 Swift FollowingPlanetModel.follow(link:)
    pub async fn follow(
        raw_link: &str,
        existing_links: &[String],
        daemon: &IpfsDaemon,
        app: &AppHandle,
    ) -> Result<Self> {
        let link = raw_link.trim().to_string();
        let link = if let Some(stripped) = link.strip_prefix("planet://") {
            stripped.to_string()
        } else {
            link
        };

        // 检查是否已关注
        if existing_links.iter().any(|l| l == &link) {
            anyhow::bail!("已关注该地址: {}", link);
        }

        if link.ends_with(".eth") {
            Self::follow_ens(&link, daemon, app).await
        } else if link.ends_with(".bit") {
            Self::follow_dotbit(&link, daemon, app).await
        } else if link.to_lowercase().starts_with("http://")
            || link.to_lowercase().starts_with("https://")
        {
            Self::follow_http(&link, app).await
        } else {
            Self::follow_ipns_or_dnslink(&link, daemon, app).await
        }
    }

    // ============================================================
    // 从 CID 获取 PublicPlanet
    // ============================================================

    /// 从 IPFS Gateway 获取 planet.json
    ///
    /// 对标 Swift getPublicPlanet(from cid:)
    async fn get_public_planet(
        cid: &str,
        gateway: &str,
    ) -> Result<Option<PublicPlanetInfo>> {
        let url = format!("{}/ipfs/{}/planet.json", gateway, cid);
        debug!("获取 planet.json: {}", url);

        let resp = match reqwest::get(&url).await {
            Ok(r) => r,
            Err(e) => {
                warn!("获取 planet.json 失败: {}", e);
                return Ok(None);
            }
        };

        if !resp.status().is_success() {
            debug!("planet.json 不存在 (HTTP {})", resp.status());
            return Ok(None);
        }

        let body = resp.text().await?;
        match serde_json::from_str::<PublicPlanetInfo>(&body) {
            Ok(planet) => {
                info!("成功解析 planet.json: {}", planet.name);
                Ok(Some(planet))
            }
            Err(e) => {
                debug!("解析 planet.json 失败: {}", e);
                Ok(None)
            }
        }
    }

    // ============================================================
    // followENS
    // ============================================================

    /// 关注 ENS 域名
    ///
    /// 对标 Swift FollowingPlanetModel.followENS()
    async fn follow_ens(ens_name: &str, daemon: &IpfsDaemon, app: &AppHandle) -> Result<Self> {
        info!("关注 ENS: {}", ens_name);

        // 1. 解析 ENS
        let resolution = ens::resolve_ens(ens_name)
            .await
            .with_context(|| format!("ENS 解析失败: {}", ens_name))?;

        let content_hash = resolution
            .content_hash
            .ok_or_else(|| anyhow::anyhow!("ENS 没有设置 contenthash: {}", ens_name))?;

        // 2. contenthash → CID
        let cid = ens::resolve_contenthash_to_cid(&content_hash, daemon).await?;
        info!("ENS {} → CID: {}", ens_name, cid);

        let gateway = daemon
            .get_gateway()
            .ok_or_else(|| anyhow::anyhow!("IPFS Gateway 未就绪"))?;

        // 3. 尝试获取原生 planet.json
        if let Some(public_planet) = Self::get_public_planet(&cid, &gateway).await? {
            info!("ENS {}: 发现原生 Planet: {}", ens_name, public_planet.name);

            let mut planet = Self::from_public_planet(
                PlanetType::Ens,
                ens_name,
                &cid,
                &public_planet,
            );

            if let Some(addr) = &resolution.address {
                planet.wallet_address = Some(addr.clone());
                planet.wallet_address_resolved_at = Some(Utc::now());
            }

            planet.ensure_directories(app)?;

            // 创建文章
            let articles: Vec<FollowingArticle> = public_planet
                .articles
                .iter()
                .map(|a| FollowingArticle::from_public_article(a, PlanetType::Ens))
                .collect();
            for article in &articles {
                article.save(&planet.articles_path(app))?;
            }
            planet.articles = articles;
            planet.articles.sort_by(|a, b| b.created.cmp(&a.created));

            planet.save(app)?;

            // 异步下载头像
            let _ = planet.download_avatar(&gateway, app).await;

            return Ok(planet);
        }

        // 4. 尝试获取 Feed
        let feed_url = format!("{}/ipfs/{}/", gateway, cid);
        let (feed_data, html_content) = feed::find_feed(&feed_url).await?;

        let now = Utc::now();
        let mut planet;
        let mut feed_avatar_data: Option<Vec<u8>> = None;

        if let Some(data) = feed_data {
            info!("ENS {}: 发现 Feed", ens_name);
            let parsed = feed::parse_feed(&data, &feed_url).await?;
            feed_avatar_data = parsed.avatar;

            planet = Self::new(
                PlanetType::Ens,
                parsed.name.unwrap_or_else(|| ens_name.to_string()),
                parsed.about.unwrap_or_default(),
                ens_name.to_string(),
                Some(cid.clone()),
            );

            if let Some(addr) = &resolution.address {
                planet.wallet_address = Some(addr.clone());
                planet.wallet_address_resolved_at = Some(now);
            }

            if let Some(pub_articles) = &parsed.articles {
                let deduped = deduplicate_articles(pub_articles.clone());
                planet.articles = deduped
                    .iter()
                    .map(|a| FollowingArticle::from_public_article(a, PlanetType::Ens))
                    .collect();
                planet.articles.sort_by(|a, b| b.created.cmp(&a.created));
            }
        } else if let Some(html) = html_content {
            info!("ENS {}: 无 Feed，使用首页作为唯一文章", ens_name);
            planet = Self::new(
                PlanetType::Ens,
                ens_name.to_string(),
                String::new(),
                ens_name.to_string(),
                Some(cid.clone()),
            );

            // 提取 HTML title
            let title = extract_html_title(&html).unwrap_or_else(|| "Homepage".to_string());
            let homepage = PublicArticleInfo {
                id: Uuid::new_v4(),
                link: "/".to_string(),
                title,
                content: String::new(),
                content_rendered: None,
                created: now,
                has_video: Some(false),
                video_filename: None,
                has_audio: Some(false),
                audio_filename: None,
                audio_duration: None,
                audio_byte_length: None,
                attachments: None,
                hero_image: None,
                tags: None,
            };
            planet.articles = vec![
                FollowingArticle::from_public_article(&homepage, PlanetType::Ens)
            ];
        } else {
            anyhow::bail!("ENS {}: 未找到 planet.json 或 Feed", ens_name);
        }

        planet.ensure_directories(app)?;
        for article in &planet.articles {
            article.save(&planet.articles_path(app))?;
        }

        // 处理头像：ENS avatar > Feed avatar
        if let Some(avatar_url) = &resolution.avatar {
            if let Ok(resp) = reqwest::get(avatar_url).await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        let _ = std::fs::write(planet.avatar_path(app), &bytes);
                    }
                }
            }
        } else if let Some(avatar_data) = feed_avatar_data {
            let _ = std::fs::write(planet.avatar_path(app), &avatar_data);
        }

        planet.save(app)?;
        Ok(planet)
    }

    // ============================================================
    // followDotBit
    // ============================================================

    /// 关注 .bit 域名
    ///
    /// 对标 Swift FollowingPlanetModel.followDotBit()
    async fn follow_dotbit(dotbit_name: &str, daemon: &IpfsDaemon, app: &AppHandle) -> Result<Self> {
        info!("关注 .bit: {}", dotbit_name);

        let dweb = dotbit::resolve_dotbit(dotbit_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!(".bit 没有 DWeb 记录: {}", dotbit_name))?;

        let cid = match dweb.record_type {
            DWebRecordType::Ipfs => dweb.value.clone(),
            DWebRecordType::Ipns => {
                daemon
                    .resolve_ipns(&dweb.value)
                    .await
                    .with_context(|| format!(".bit IPNS 解析失败: {}", dweb.value))?
            }
        };

        info!(".bit {} → CID: {}", dotbit_name, cid);
        let gateway = daemon
            .get_gateway()
            .ok_or_else(|| anyhow::anyhow!("IPFS Gateway 未就绪"))?;

        // Pin CID
        let _ = daemon.pin(&cid).await;

        // 尝试获取原生 planet.json
        if let Some(public_planet) = Self::get_public_planet(&cid, &gateway).await? {
            info!(".bit {}: 发现原生 Planet: {}", dotbit_name, public_planet.name);

            let mut planet = Self::from_public_planet(
                PlanetType::DotBit,
                dotbit_name,
                &cid,
                &public_planet,
            );
            planet.ensure_directories(app)?;

            let articles: Vec<FollowingArticle> = public_planet
                .articles
                .iter()
                .map(|a| FollowingArticle::from_public_article(a, PlanetType::DotBit))
                .collect();
            for article in &articles {
                article.save(&planet.articles_path(app))?;
            }
            planet.articles = articles;
            planet.articles.sort_by(|a, b| b.created.cmp(&a.created));

            let _ = planet.download_avatar(&gateway, app).await;
            planet.save(app)?;
            return Ok(planet);
        }

        // 尝试 Feed
        let feed_url = format!("{}/ipfs/{}/", gateway, cid);
        let (feed_data, html_content) = feed::find_feed(&feed_url).await?;
        let now = Utc::now();
        let mut planet;
        let mut feed_avatar_data: Option<Vec<u8>> = None;

        if let Some(data) = feed_data {
            info!(".bit {}: 发现 Feed", dotbit_name);
            let parsed = feed::parse_feed(&data, &feed_url).await?;
            feed_avatar_data = parsed.avatar;

            planet = Self::new(
                PlanetType::DotBit,
                parsed.name.unwrap_or_else(|| dotbit_name.to_string()),
                parsed.about.unwrap_or_default(),
                dotbit_name.to_string(),
                Some(cid.clone()),
            );

            if let Some(pub_articles) = &parsed.articles {
                let deduped = deduplicate_articles(pub_articles.clone());
                planet.articles = deduped
                    .iter()
                    .map(|a| FollowingArticle::from_public_article(a, PlanetType::DotBit))
                    .collect();
                planet.articles.sort_by(|a, b| b.created.cmp(&a.created));
            }
        } else if let Some(html) = html_content {
            info!(".bit {}: 无 Feed，使用首页", dotbit_name);
            planet = Self::new(
                PlanetType::DotBit,
                dotbit_name.to_string(),
                String::new(),
                dotbit_name.to_string(),
                Some(cid.clone()),
            );
            let title = extract_html_title(&html).unwrap_or_else(|| "Homepage".to_string());
            let homepage = PublicArticleInfo {
                id: Uuid::new_v4(),
                link: "/".to_string(),
                title,
                content: String::new(),
                content_rendered: None,
                created: now,
                has_video: Some(false),
                video_filename: None,
                has_audio: Some(false),
                audio_filename: None,
                audio_duration: None,
                audio_byte_length: None,
                attachments: None,
                hero_image: None,
                tags: None,
            };
            planet.articles = vec![
                FollowingArticle::from_public_article(&homepage, PlanetType::DotBit)
            ];
        } else {
            anyhow::bail!(".bit {}: 未找到内容", dotbit_name);
        }

        planet.ensure_directories(app)?;
        for article in &planet.articles {
            article.save(&planet.articles_path(app))?;
        }
        if let Some(avatar_data) = feed_avatar_data {
            let _ = std::fs::write(planet.avatar_path(app), &avatar_data);
        }
        planet.save(app)?;
        Ok(planet)
    }

    // ============================================================
    // followHTTP
    // ============================================================

    /// 关注 HTTP RSS/Atom Feed
    ///
    /// 对标 Swift FollowingPlanetModel.followHTTP()
    async fn follow_http(link: &str, app: &AppHandle) -> Result<Self> {
        info!("关注 HTTP Feed: {}", link);

        let (feed_data, _) = feed::find_feed(link).await?;
        let feed_data =
            feed_data.ok_or_else(|| anyhow::anyhow!("未找到有效的 Feed: {}", link))?;

        let parsed = feed::parse_feed(&feed_data, link).await?;

        let mut planet = Self::new(
            PlanetType::Dns,
            parsed.name.unwrap_or_else(|| link.to_string()),
            parsed.about.unwrap_or_default(),
            link.to_string(),
            None, // HTTP Feed 没有 CID
        );

        planet.ensure_directories(app)?;

        if let Some(pub_articles) = &parsed.articles {
            let deduped = deduplicate_articles(pub_articles.clone());
            planet.articles = deduped
                .iter()
                .map(|a| FollowingArticle::from_public_article(a, PlanetType::Dns))
                .collect();
            planet.articles.sort_by(|a, b| b.created.cmp(&a.created));
        }

        for article in &planet.articles {
            article.save(&planet.articles_path(app))?;
        }

        // 下载 Feed 头像
        if let Some(avatar_data) = parsed.avatar {
            let _ = std::fs::write(planet.avatar_path(app), &avatar_data);
        }

        planet.save(app)?;
        Ok(planet)
    }

    // ============================================================
    // followIPNSorDNSLink
    // ============================================================

    /// 关注 IPNS 或 DNSLink 地址
    ///
    /// 对标 Swift FollowingPlanetModel.followIPNSorDNSLink()
    async fn follow_ipns_or_dnslink(name: &str, daemon: &IpfsDaemon, app: &AppHandle) -> Result<Self> {
        let planet_type = if ens::is_ipns(name) {
            PlanetType::Planet
        } else {
            PlanetType::DnsLink
        };

        info!("关注 {:?}: {}", planet_type, name);

        // 1. 解析 IPNS/DNSLink → CID
        let cid = daemon
            .resolve_ipns(name)
            .await
            .with_context(|| format!("IPNS/DNSLink 解析失败: {}", name))?;

        info!("{} → CID: {}", name, cid);
        let gateway = daemon
            .get_gateway()
            .ok_or_else(|| anyhow::anyhow!("IPFS Gateway 未就绪"))?;

        // 后台 Pin
        let _ = daemon.pin(&cid).await;

        // 2. 尝试获取 planet.json
        if let Some(public_planet) = Self::get_public_planet(&cid, &gateway).await? {
            info!("{}: 发现原生 Planet: {}", name, public_planet.name);

            let mut planet = Self::from_public_planet(
                planet_type,
                name,
                &cid,
                &public_planet,
            );
            planet.ensure_directories(app)?;

            let articles: Vec<FollowingArticle> = public_planet
                .articles
                .iter()
                .map(|a| FollowingArticle::from_public_article(a, planet_type))
                .collect();
            for article in &articles {
                article.save(&planet.articles_path(app))?;
            }
            planet.articles = articles;
            planet.articles.sort_by(|a, b| b.created.cmp(&a.created));

            let _ = planet.download_avatar(&gateway, app).await;
            planet.save(app)?;
            return Ok(planet);
        }

        // 3. 尝试 Feed
        let feed_url = format!("{}/ipfs/{}/", gateway, cid);
        let (feed_data, html_content) = feed::find_feed(&feed_url).await?;
        let now = Utc::now();
        let mut planet;
        let mut feed_avatar_data: Option<Vec<u8>> = None;

        if let Some(data) = feed_data {
            info!("{}: 发现 Feed", name);
            let parsed = feed::parse_feed(&data, &feed_url).await?;
            feed_avatar_data = parsed.avatar;

            planet = Self::new(
                planet_type,
                parsed.name.unwrap_or_else(|| name.to_string()),
                parsed.about.unwrap_or_default(),
                name.to_string(),
                Some(cid.clone()),
            );

            if let Some(pub_articles) = &parsed.articles {
                let deduped = deduplicate_articles(pub_articles.clone());
                planet.articles = deduped
                    .iter()
                    .map(|a| FollowingArticle::from_public_article(a, planet_type))
                    .collect();
                planet.articles.sort_by(|a, b| b.created.cmp(&a.created));
            }
        } else if let Some(html) = html_content {
            info!("{}: 无 Feed，使用首页", name);
            planet = Self::new(
                planet_type,
                name.to_string(),
                String::new(),
                name.to_string(),
                Some(cid.clone()),
            );
            let title = extract_html_title(&html).unwrap_or_else(|| "Homepage".to_string());
            let homepage = PublicArticleInfo {
                id: Uuid::new_v4(),
                link: "/".to_string(),
                title,
                content: String::new(),
                content_rendered: None,
                created: now,
                has_video: Some(false),
                video_filename: None,
                has_audio: Some(false),
                audio_filename: None,
                audio_duration: None,
                audio_byte_length: None,
                attachments: None,
                hero_image: None,
                tags: None,
            };
            planet.articles = vec![
                FollowingArticle::from_public_article(&homepage, planet_type)
            ];
        } else {
            anyhow::bail!("{}: 未找到内容", name);
        }

        planet.ensure_directories(app)?;
        for article in &planet.articles {
            article.save(&planet.articles_path(app))?;
        }
        if let Some(avatar_data) = feed_avatar_data {
            let _ = std::fs::write(planet.avatar_path(app), &avatar_data);
        }
        planet.save(app)?;
        Ok(planet)
    }
}

// ============================================================
// 辅助函数
// ============================================================

/// 从 HTML 中提取 <title>
fn extract_html_title(html: &str) -> Option<String> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("title").ok()?;
    document
        .select(&selector)
        .next()
        .map(|el| el.text().collect::<String>())
}

impl FollowingPlanet {
    // ============================================================
    // 更新入口
    // ============================================================

    /// 检查并拉取更新
    ///
    /// 对标 Swift FollowingPlanetModel.update()
    pub async fn update(&mut self, daemon: &IpfsDaemon, app: &AppHandle) -> Result<()> {
        info!("更新 Planet: {} ({})", self.name, self.link);
        self.is_updating = true;

        let result = match self.planet_type {
            PlanetType::Planet | PlanetType::DnsLink => {
                self.update_ipns(daemon, app).await
            }
            PlanetType::Ens => {
                self.update_ens(daemon, app).await
            }
            PlanetType::DotBit => {
                self.update_dotbit(daemon, app).await
            }
            PlanetType::Dns => {
                self.update_http(app).await
            }
        };

        self.is_updating = false;
        result
    }

    // ============================================================
    // IPNS / DNSLink 更新
    // ============================================================

    async fn update_ipns(&mut self, daemon: &IpfsDaemon, app: &AppHandle) -> Result<()> {
        let new_cid = daemon
            .resolve_ipns(&self.link)
            .await
            .with_context(|| format!("IPNS 解析失败: {}", self.link))?;

        if self.cid.as_deref() == Some(&new_cid) {
            info!("Planet {} 无更新 (CID 相同)", self.name);
            return Ok(());
        }

        info!("Planet {} 有更新: {} → {}", self.name,
            self.cid.as_deref().unwrap_or("none"), new_cid);

        // Pin 新 CID
        let _ = daemon.pin(&new_cid).await;

        let gateway = daemon
            .get_gateway()
            .ok_or_else(|| anyhow::anyhow!("IPFS Gateway 未就绪"))?;

        // 尝试获取 planet.json
        if let Some(public_planet) = Self::get_public_planet(&new_cid, &gateway).await? {
            if public_planet.updated <= self.updated {
                info!("Planet {} 无实际更新 (updated 时间未变)", self.name);
                return Ok(());
            }

            self.name = public_planet.name.clone();
            self.about = public_planet.about.clone();
            self.updated = public_planet.updated;
            self.twitter_username = public_planet.twitter_username.clone();
            self.github_username = public_planet.github_username.clone();
            self.telegram_username = public_planet.telegram_username.clone();
            self.mastodon_username = public_planet.mastodon_username.clone();
            self.juicebox_enabled = public_planet.juicebox_enabled;
            self.juicebox_project_id = public_planet.juicebox_project_id;
            self.juicebox_project_id_goerli = public_planet.juicebox_project_id_goerli;

            self.update_articles(&public_planet.articles, true, app)?;

            // 下载头像
            let _ = self.download_avatar(&gateway, app).await;

            self.cid = Some(new_cid);
            self.last_retrieved = Utc::now();
            self.save(app)?;
            return Ok(());
        }

        // 尝试 Feed
        let feed_url = format!("{}/ipfs/{}/", gateway, new_cid);
        let (feed_data, _) = feed::find_feed(&feed_url).await?;
        if let Some(data) = feed_data {
            let parsed = feed::parse_feed(&data, &feed_url).await?;
            let now = Utc::now();

            self.name = parsed.name.unwrap_or_else(|| self.link.clone());
            self.about = parsed.about.unwrap_or_default();
            self.updated = now;
            self.cid = Some(new_cid);
            self.last_retrieved = now;

            if let Some(pub_articles) = &parsed.articles {
                self.update_articles(pub_articles, false, app)?;
            }

            if let Some(avatar_data) = parsed.avatar {
                let _ = std::fs::write(self.avatar_path(app), &avatar_data);
            }

            self.save(app)?;
        }

        Ok(())
    }

    // ============================================================
    // ENS 更新
    // ============================================================

    async fn update_ens(&mut self, daemon: &IpfsDaemon, app: &AppHandle) -> Result<()> {
        info!("更新 ENS Planet: {} ({})", self.name, self.link);

        // 1. 解析 ENS
        let resolution = ens::resolve_ens(&self.link).await?;

        // 更新钱包地址
        if let Some(addr) = &resolution.address {
            if self.wallet_address.as_deref() != Some(addr) {
                self.wallet_address = Some(addr.clone());
                self.wallet_address_resolved_at = Some(Utc::now());
                self.save(app)?;
            }
        }

        let content_hash = resolution
            .content_hash
            .ok_or_else(|| anyhow::anyhow!("ENS 没有 contenthash: {}", self.link))?;

        // 2. contenthash → CID
        let new_cid = ens::resolve_contenthash_to_cid(&content_hash, daemon).await?;

        if self.cid.as_deref() == Some(&new_cid) {
            info!("ENS Planet {} 无更新 (CID 相同)", self.name);
            return Ok(());
        }

        info!("ENS Planet {} 有更新: {} → {}", self.name,
            self.cid.as_deref().unwrap_or("none"), new_cid);

        let _ = daemon.pin(&new_cid).await;
        let gateway = daemon
            .get_gateway()
            .ok_or_else(|| anyhow::anyhow!("IPFS Gateway 未就绪"))?;

        // 3. 获取 planet.json
        if let Some(public_planet) = Self::get_public_planet(&new_cid, &gateway).await? {
            if public_planet.updated <= self.updated {
                info!("ENS Planet {} 无实际更新", self.name);
                return Ok(());
            }

            self.name = public_planet.name.clone();
            self.about = public_planet.about.clone();
            self.updated = public_planet.updated;
            self.twitter_username = public_planet.twitter_username.clone();
            self.github_username = public_planet.github_username.clone();
            self.telegram_username = public_planet.telegram_username.clone();
            self.mastodon_username = public_planet.mastodon_username.clone();
            self.juicebox_enabled = public_planet.juicebox_enabled;
            self.juicebox_project_id = public_planet.juicebox_project_id;
            self.juicebox_project_id_goerli = public_planet.juicebox_project_id_goerli;

            self.update_articles(&public_planet.articles, true, app)?;

            // 头像：优先 ENS avatar, 然后 IPFS avatar
            if let Some(avatar_url) = &resolution.avatar {
                if let Ok(resp) = reqwest::get(avatar_url).await {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            let _ = std::fs::write(self.avatar_path(app), &bytes);
                        }
                    }
                }
            } else {
                let _ = self.download_avatar(&gateway, app).await;
            }

            self.cid = Some(new_cid);
            self.last_retrieved = Utc::now();
            self.save(app)?;
            return Ok(());
        }

        // Feed 回退
        let feed_url = format!("{}/ipfs/{}/", gateway, new_cid);
        let (feed_data, _) = feed::find_feed(&feed_url).await?;
        if let Some(data) = feed_data {
            let parsed = feed::parse_feed(&data, &feed_url).await?;
            let now = Utc::now();

            self.cid = Some(new_cid);
            self.name = parsed.name.unwrap_or_else(|| self.link.clone());
            self.about = parsed.about.unwrap_or_default();
            self.updated = now;
            self.last_retrieved = now;

            if let Some(pub_articles) = &parsed.articles {
                self.update_articles(pub_articles, false, app)?;
            }

            // 头像
            if let Some(avatar_url) = &resolution.avatar {
                if let Ok(resp) = reqwest::get(avatar_url).await {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            let _ = std::fs::write(self.avatar_path(app), &bytes);
                        }
                    }
                }
            } else if let Some(avatar_data) = parsed.avatar {
                let _ = std::fs::write(self.avatar_path(app), &avatar_data);
            }

            self.save(app)?;
        }

        Ok(())
    }

    // ============================================================
    // .bit 更新
    // ============================================================

    async fn update_dotbit(&mut self, daemon: &IpfsDaemon, app: &AppHandle) -> Result<()> {
        info!("更新 .bit Planet: {} ({})", self.name, self.link);

        let dweb = dotbit::resolve_dotbit(&self.link)
            .await?
            .ok_or_else(|| anyhow::anyhow!(".bit 无 DWeb 记录: {}", self.link))?;

        let new_cid = match dweb.record_type {
            DWebRecordType::Ipfs => dweb.value.clone(),
            DWebRecordType::Ipns => {
                daemon.resolve_ipns(&dweb.value).await?
            }
        };

        if self.cid.as_deref() == Some(&new_cid) {
            info!(".bit Planet {} 无更新", self.name);
            return Ok(());
        }

        let _ = daemon.pin(&new_cid).await;
        let gateway = daemon
            .get_gateway()
            .ok_or_else(|| anyhow::anyhow!("IPFS Gateway 未就绪"))?;

        // 与 update_ipns 逻辑基本一致
        if let Some(public_planet) = Self::get_public_planet(&new_cid, &gateway).await? {
            if public_planet.updated <= self.updated {
                return Ok(());
            }

            self.name = public_planet.name.clone();
            self.about = public_planet.about.clone();
            self.updated = public_planet.updated;
            self.twitter_username = public_planet.twitter_username.clone();
            self.github_username = public_planet.github_username.clone();
            self.telegram_username = public_planet.telegram_username.clone();
            self.mastodon_username = public_planet.mastodon_username.clone();

            self.update_articles(&public_planet.articles, true, app)?;
            let _ = self.download_avatar(&gateway, app).await;

            self.cid = Some(new_cid);
            self.last_retrieved = Utc::now();
            self.save(app)?;
            return Ok(());
        }

        // Feed 回退
        let feed_url = format!("{}/ipfs/{}/", gateway, new_cid);
        let (feed_data, _) = feed::find_feed(&feed_url).await?;
        if let Some(data) = feed_data {
            let parsed = feed::parse_feed(&data, &feed_url).await?;
            let now = Utc::now();

            self.cid = Some(new_cid);
            self.name = parsed.name.unwrap_or_else(|| self.link.clone());
            self.about = parsed.about.unwrap_or_default();
            self.updated = now;
            self.last_retrieved = now;

            if let Some(pub_articles) = &parsed.articles {
                self.update_articles(pub_articles, false, app)?;
            }
            if let Some(avatar_data) = parsed.avatar {
                let _ = std::fs::write(self.avatar_path(app), &avatar_data);
            }
            self.save(app)?;
        }

        Ok(())
    }

    // ============================================================
    // HTTP Feed 更新
    // ============================================================

    async fn update_http(&mut self, app: &AppHandle) -> Result<()> {
        info!("更新 HTTP Feed: {} ({})", self.name, self.link);

        let (feed_data, _) = feed::find_feed(&self.link).await?;
        let feed_data =
            feed_data.ok_or_else(|| anyhow::anyhow!("获取 Feed 失败: {}", self.link))?;

        let parsed = feed::parse_feed(&feed_data, &self.link).await?;
        let now = Utc::now();

        self.name = parsed.name.unwrap_or_else(|| self.link.clone());
        self.about = parsed.about.unwrap_or_default();
        self.updated = now;
        self.last_retrieved = now;

        if let Some(pub_articles) = &parsed.articles {
            self.update_articles(pub_articles, false, app)?;
        }

        // 更新头像
        if let Some(avatar_data) = parsed.avatar {
            let _ = std::fs::write(self.avatar_path(app), &avatar_data);
        } else {
            // 尝试从 HTML 页面获取头像
            if let Ok((_, Some(html))) = feed::find_feed(&self.link).await {
                if let Some(avatar_data) =
                    feed::find_avatar_from_html(&html, &self.link).await
                {
                    let _ = std::fs::write(self.avatar_path(app), &avatar_data);
                }
            }
        }

        self.save(app)?;
        Ok(())
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_link() {
        // 移除内部网关前缀（http://127.0.0.1:xxxxx/ 共 22 字符）
        let link = "http://127.0.0.1:18181/ipfs/Qm.../path";
        let result = sanitize_link(link);
        assert!(
            !result.starts_with("http://127.0.0.1:"),
            "应移除网关前缀, got: {}",
            result
        );

        // 普通路径保持不变
        assert_eq!(sanitize_link("/some/path"), "/some/path");
    }

    #[test]
    fn test_deduplicate_articles() {
        let articles = vec![
            PublicArticleInfo {
                id: Uuid::new_v4(),
                link: "/post1".to_string(),
                title: "Post 1".to_string(),
                content: "".to_string(),
                content_rendered: None,
                created: Utc::now(),
                has_video: None,
                video_filename: None,
                has_audio: None,
                audio_filename: None,
                audio_duration: None,
                audio_byte_length: None,
                attachments: None,
                hero_image: None,
                tags: None,
            },
            PublicArticleInfo {
                id: Uuid::new_v4(),
                link: "/post1".to_string(), // 重复
                title: "Post 1 Dup".to_string(),
                content: "".to_string(),
                content_rendered: None,
                created: Utc::now(),
                has_video: None,
                video_filename: None,
                has_audio: None,
                audio_filename: None,
                audio_duration: None,
                audio_byte_length: None,
                attachments: None,
                hero_image: None,
                tags: None,
            },
        ];

        let deduped = deduplicate_articles(articles);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].title, "Post 1");
    }

    #[test]
    fn test_following_planet_new() {
        let planet = FollowingPlanet::new(
            PlanetType::Ens,
            "Test".to_string(),
            "About".to_string(),
            "test.eth".to_string(),
            Some("QmTest".to_string()),
        );
        assert_eq!(planet.name, "Test");
        assert_eq!(planet.planet_type, PlanetType::Ens);
        assert!(planet.articles.is_empty());
    }
}
