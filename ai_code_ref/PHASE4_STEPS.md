# Phase 4：关注与内容获取 — 详细开发文档

> **目标**：实现关注其他 Planet 并获取内容的功能，包含 IPNS/DNSLink 解析、ENS 解析、.bit 域名解析、RSS/Atom Feed 解析、内容获取与更新、定时后台更新。对应原项目 `FollowingPlanetModel.swift`、`FollowingArticleModel.swift`、`FeedUtils.swift`、`ENSUtils.swift`、`DotBitKit.swift`。
>
> **预计工期**：2 周
>
> **验收标准**：输入一个已知的 IPNS 地址（如 `k51...`）或 ENS 域名（如 `vitalik.eth`），能成功关注并拉取到 planet.json 和文章列表。文章内容可在 ArticleDetail 中正常展示。手动点击 Update 能检查并拉取新内容。关闭重开后 Following 列表和已拉取的文章不丢失。

---

## 目录

- [1. 整体架构](#1-整体架构)
- [2. 新增依赖](#2-新增依赖)
- [3. Step 1：实现 FollowingPlanet 数据模型 (`models/following_planet.rs`)](#3-step-1实现-followingplanet-数据模型-modelsfollowing_planetrs)
- [4. Step 2：实现 FollowingArticle 数据模型 (`models/following_article.rs`)](#4-step-2实现-followingarticle-数据模型-modelsfollowing_articlers)
- [5. Step 3：实现 Feed 解析器 (`helpers/feed.rs`)](#5-step-3实现-feed-解析器-helpersfeedrs)
- [6. Step 4：实现 ENS 解析 (`helpers/ens.rs`)](#6-step-4实现-ens-解析-helpersensrs)
- [7. Step 5：实现 .bit 域名解析 (`helpers/dotbit.rs`)](#7-step-5实现-bit-域名解析-helpersdotbitrs)
- [8. Step 6：实现关注流程 — follow (`models/following_planet.rs` 扩展)](#8-step-6实现关注流程--follow-modelsfollowing_planetrs-扩展)
- [9. Step 7：实现更新流程 — update (`models/following_planet.rs` 扩展)](#9-step-7实现更新流程--update-modelsfollowing_planetrs-扩展)
- [10. Step 8：实现定时后台更新 (`store/mod.rs` 扩展)](#10-step-8实现定时后台更新-storemodrs-扩展)
- [11. Step 9：注册 Tauri Commands (`commands/planet.rs` 扩展)](#11-step-9注册-tauri-commands-commandsplanetrs-扩展)
- [12. Step 10：前端实现](#12-step-10前端实现)
- [13. Step 11：测试与调试](#13-step-11测试与调试)
- [14. 文件清单](#14-文件清单)
- [15. Swift → Rust 对照表](#15-swift--rust-对照表)
- [16. 执行顺序总结](#16-执行顺序总结)

---

## 1. 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      前端 (React)                           │
│                                                             │
│  FollowPlanetDialog  ──→  invoke("planet_follow")           │
│  FollowingList       ←──  invoke("following_list")          │
│  ArticleDetail       ←──  invoke("following_article_get")   │
│  UpdateButton        ──→  invoke("following_update")        │
│  UpdateAllButton     ──→  invoke("following_update_all")    │
│  UnfollowButton      ──→  invoke("planet_unfollow")         │
│                                                             │
│  listen("following-updated")  ← 后台更新通知                │
│                                                             │
└────────────────────────┬────────────────────────────────────┘
                         │ IPC (Tauri invoke / events)
┌────────────────────────┴────────────────────────────────────┐
│                    Rust 后端                                 │
│                                                             │
│  commands/planet.rs                                          │
│    ├── planet_follow(link)     → 关注 Planet/ENS/RSS        │
│    ├── planet_unfollow(id)     → 取消关注                    │
│    ├── following_list()        → 列出所有关注                │
│    ├── following_update(id)    → 更新单个                    │
│    ├── following_update_all()  → 更新所有                    │
│    ├── following_articles(id)  → 获取文章列表                │
│    └── following_article_get() → 获取单篇文章                │
│                                                             │
│  models/following_planet.rs                                  │
│    ├── follow(link)            → 入口路由                    │
│    │     ├── follow_ens()      → ENS 解析 → 获取内容        │
│    │     ├── follow_dotbit()   → .bit 解析 → 获取内容       │
│    │     ├── follow_http()     → RSS/Atom Feed 解析         │
│    │     └── follow_ipns()     → IPNS/DNSLink 解析          │
│    ├── update()                → 检查更新并拉取新内容        │
│    ├── update_articles()       → 增量更新文章列表            │
│    ├── save() / load() / delete()                           │
│    └── refresh_icon()          → 下载头像                    │
│                                                             │
│  models/following_article.rs                                 │
│    ├── from_public_article()   → PublicArticleModel → 本地   │
│    ├── save() / load() / delete()                           │
│    └── extract_summary()       → 提取摘要                   │
│                                                             │
│  helpers/feed.rs                                             │
│    ├── find_feed(url)          → 发现 RSS/Atom Feed         │
│    ├── parse_feed(data)        → 解析 Feed → 文章列表       │
│    └── find_avatar_from_html() → 从 HTML 中提取头像         │
│                                                             │
│  helpers/ens.rs                                              │
│    ├── is_ipns(str)            → 判断是否为 IPNS 地址       │
│    └── resolve_ens(name)       → ENS → contenthash → CID   │
│                                                             │
│  helpers/dotbit.rs                                           │
│    └── resolve_dotbit(account) → .bit → dweb record → CID  │
│                                                             │
│  ipfs/daemon.rs  (Phase 1 已实现)                            │
│    ├── resolve_ipns_or_dnslink()                             │
│    ├── pin() / unpin()                                       │
│    └── get_gateway()                                         │
└─────────────────────────────────────────────────────────────┘
```

### 关注流程概览（对应 Swift `follow()`)

```
用户输入 link
  │
  ├─ link.ends_with(".eth")  → follow_ens()
  │    ├─ ENS resolve → contenthash
  │    ├─ contenthash → CID (via IPFS resolve)
  │    ├─ 尝试获取 planet.json → 如果存在 → 原生 Planet
  │    └─ 否则 → 尝试发现 Feed → RSS/Atom 解析
  │
  ├─ link.ends_with(".bit")  → follow_dotbit()
  │    ├─ DotBit indexer API → dweb record
  │    ├─ ipfs/ipns → CID
  │    └─ 同上：尝试 planet.json 或 Feed
  │
  ├─ link.starts_with("http") → follow_http()
  │    ├─ 发现 Feed (RSS/Atom/JSON)
  │    └─ 解析 Feed → 文章列表
  │
  └─ 其他 (IPNS / DNSLink) → follow_ipns_or_dnslink()
       ├─ IPFS name/resolve → CID
       ├─ 尝试获取 planet.json
       └─ 否则 → 尝试 Feed
```

### 更新流程概览（对应 Swift `update()`)

```
FollowingPlanet.update()
  │
  ├─ planetType == planet/dnslink:
  │    ├─ resolve IPNS → newCID
  │    ├─ if cid == newCID → 无更新，返回
  │    ├─ pin newCID
  │    ├─ 获取 planet.json → update_articles()
  │    └─ 刷新头像
  │
  ├─ planetType == ens:
  │    ├─ ENS resolve → contenthash → newCID
  │    ├─ if cid == newCID → 无更新，返回
  │    ├─ pin newCID
  │    ├─ 获取 planet.json → update_articles()
  │    └─ 刷新头像
  │
  ├─ planetType == dotbit: (类似 ens)
  │
  └─ planetType == dns (HTTP Feed):
       ├─ 重新获取 Feed
       └─ update_articles()
```

---

## 2. 新增依赖

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中添加：

```toml
# RSS/Atom Feed 解析
feed-rs = "1"

# HTML 解析 (发现 Feed、提取摘要)
scraper = "0.18"

# ENS 解析 (Ethereum RPC 调用)
alloy = { version = "0.3", features = ["provider-http", "contract"] }
# 或者使用更轻量的方案:
# ethers = { version = "2", features = ["abigen"] }

# URL 处理
url = "2"
```

> **说明**：
> - `feed-rs`：Rust 最流行的 RSS/Atom/JSON Feed 解析库，对标 Swift 的 `FeedKit`
> - `scraper`：基于 CSS 选择器的 HTML 解析器，对标 Swift 的 `SwiftSoup`
> - `alloy`：新一代 Ethereum 库（ethers-rs 的继任者），用于 ENS 解析
> - `url`：标准 URL 解析库

### 已有依赖（Phase 1/2/3 已添加）

- `tokio`、`reqwest`、`anyhow`、`thiserror`、`tracing`、`serde`、`serde_json`、`uuid`、`chrono`、`pulldown-cmark`、`tera`、`keyring`、`regex`

### 关于 ENS 解析的简化方案

ENS 解析需要连接 Ethereum RPC 节点，`alloy` 是功能完整但比较重的方案。如果想先简化实现，可以用 HTTP API：

```toml
# 简化方案：使用 ENS HTTP API 而非直接调用合约
# 不需要 alloy，仅用 reqwest 即可
```

本文档将提供两种方案，推荐先用 HTTP API 方案快速实现。

---

## 3. Step 1：实现 FollowingPlanet 数据模型 (`models/following_planet.rs`)

### 3.1 对应 Swift 代码

```swift
// Planet/Entities/FollowingPlanetModel.swift
enum PlanetType: Int, Codable {
    case planet = 0  // IPNS
    case ens = 1
    case dnslink = 2
    case dns = 3     // HTTP Feed
    case dotbit = 4
}

class FollowingPlanetModel {
    let id: UUID
    var name: String
    var about: String
    let created: Date
    let planetType: PlanetType
    let link: String
    var cid: String?
    var updated: Date
    var lastRetrieved: Date
    var isUpdating: Bool = false
    var articles: [FollowingArticleModel]
    // ...
}
```

### 3.2 Rust 实现

创建文件 `src-tauri/src/models/following_planet.rs`：

```rust
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::helpers::paths;
use crate::models::following_article::FollowingArticle;

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
    pub fn following_planets_path() -> PathBuf {
        let path = paths::repo_path().join("Following");
        let _ = fs::create_dir_all(&path);
        path
    }

    /// 本 Planet 的基础目录
    pub fn base_path(&self) -> PathBuf {
        Self::following_planets_path().join(self.id.to_string())
    }

    /// planet.json 路径
    pub fn info_path(&self) -> PathBuf {
        self.base_path().join("planet.json")
    }

    /// Articles 子目录
    pub fn articles_path(&self) -> PathBuf {
        self.base_path().join("Articles")
    }

    /// 头像路径
    pub fn avatar_path(&self) -> PathBuf {
        self.base_path().join("avatar.png")
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
    pub fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(self.info_path(), data)?;
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
    pub fn delete(&self) -> Result<()> {
        let path = self.base_path();
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        info!("已删除 Following Planet: {} ({})", self.name, self.id);
        Ok(())
    }

    /// 确保目录结构存在
    pub fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(self.base_path())?;
        fs::create_dir_all(self.articles_path())?;
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
    ) -> Result<Vec<FollowingArticle>> {
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
                article.update_summary(self.planet_type);
                article.save(&self.articles_path())?;
            } else {
                // 新文章
                let article = FollowingArticle::from_public_article(pub_article, self.planet_type);
                article.save(&self.articles_path())?;
                new_articles.push(article.clone());
                self.articles.push(article);
            }
        }

        // 删除远端不存在的文章
        if delete {
            let mut to_remove = Vec::new();
            for (i, article) in self.articles.iter().enumerate() {
                if !seen_links.contains(&article.link) {
                    article.delete(&self.articles_path());
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
    pub async fn download_avatar(&self, gateway: &str) -> Result<()> {
        if let Some(ref cid) = self.cid {
            let avatar_url = format!("{}/ipfs/{}/avatar.png", gateway, cid);
            let resp = reqwest::get(&avatar_url).await?;
            if resp.status().is_success() {
                let bytes = resp.bytes().await?;
                if !bytes.is_empty() {
                    fs::write(self.avatar_path(), &bytes)?;
                    info!("已下载头像: {} ({})", self.name, self.id);
                }
            }
        }
        Ok(())
    }

    /// 头像是否存在
    pub fn has_avatar(&self) -> bool {
        self.avatar_path().exists()
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
```

### 3.3 前端用到的序列化模型

在 `FollowingPlanet` 上添加一个供前端消费的精简版结构：

```rust
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
    pub fn snapshot(&self) -> FollowingPlanetSnapshot {
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
            has_avatar: self.has_avatar(),
        }
    }
}
```

### 3.4 更新 `models/mod.rs`

```rust
pub mod planet;
pub mod article;
pub mod draft;
pub mod following_planet;   // 新增
pub mod following_article;  // 新增
```

---

## 4. Step 2：实现 FollowingArticle 数据模型 (`models/following_article.rs`)

### 4.1 对应 Swift 代码

```swift
// Planet/Entities/FollowingArticleModel.swift
class FollowingArticleModel: ArticleModel, Codable {
    var link: String
    var read: Date? = nil
    var summary: String? = nil
    // ...
}
```

### 4.2 Rust 实现

创建文件 `src-tauri/src/models/following_article.rs`：

```rust
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::helpers::markdown::render_markdown_html;
use crate::models::following_planet::{PlanetType, PublicArticleInfo};

/// 关注的文章数据模型
///
/// 对标 Swift FollowingArticleModel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowingArticle {
    pub id: Uuid,
    pub link: String,
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
    pub created: DateTime<Utc>,
    pub read: Option<DateTime<Utc>>,
    pub starred: Option<DateTime<Utc>>,
    pub star_type: Option<String>,
    pub video_filename: Option<String>,
    pub audio_filename: Option<String>,
    pub attachments: Option<Vec<String>>,
}

impl FollowingArticle {
    // ============================================================
    // 构造
    // ============================================================

    /// 从 PublicArticleInfo 创建
    ///
    /// 对标 Swift FollowingArticleModel.from(publicArticle:planet:)
    pub fn from_public_article(
        pub_article: &PublicArticleInfo,
        planet_type: PlanetType,
    ) -> Self {
        let mut article = FollowingArticle {
            id: Uuid::new_v4(),  // 生成新 UUID，不复用远端 ID
            link: sanitize_article_link(&pub_article.link),
            title: pub_article.title.clone(),
            content: pub_article.content.clone(),
            summary: None,
            created: pub_article.created,
            read: None,
            starred: None,
            star_type: Some("star".to_string()),
            video_filename: pub_article.video_filename.clone(),
            audio_filename: pub_article.audio_filename.clone(),
            attachments: pub_article.attachments.clone(),
        };
        article.update_summary(planet_type);
        article
    }

    // ============================================================
    // 摘要提取
    // ============================================================

    /// 提取文章摘要
    ///
    /// 对标 Swift FollowingArticleModel.extractSummary()
    pub fn extract_summary(content: &str, planet_type: PlanetType) -> Option<String> {
        if content.is_empty() {
            return None;
        }

        let html = match planet_type {
            PlanetType::Planet | PlanetType::Ens | PlanetType::DotBit => {
                // 原生 Planet 的 content 是 Markdown
                render_markdown_html(content)
            }
            PlanetType::DnsLink | PlanetType::Dns => {
                // Feed 的 content 已经是 HTML
                content.to_string()
            }
        };

        // 从 HTML 中提取纯文本
        let text = strip_html_tags(&html);
        let trimmed = text.trim();

        if trimmed.is_empty() {
            return None;
        }

        if trimmed.len() > 280 {
            Some(format!("{}...", &trimmed[..280]))
        } else {
            Some(trimmed.to_string())
        }
    }

    /// 更新摘要
    pub fn update_summary(&mut self, planet_type: PlanetType) {
        self.summary = Self::extract_summary(&self.content, planet_type);
    }

    // ============================================================
    // 持久化
    // ============================================================

    /// 保存到磁盘
    pub fn save(&self, articles_dir: &Path) -> Result<()> {
        let path = articles_dir.join(format!("{}.json", self.id));
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&path, data)?;
        Ok(())
    }

    /// 从文件加载
    pub fn load(file_path: &Path) -> Result<Self> {
        let filename = file_path
            .file_stem()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("无效的文件名"))?;
        let article_id = Uuid::parse_str(filename)
            .with_context(|| format!("文件名不是有效的 UUID: {}", filename))?;

        let data = fs::read_to_string(file_path)
            .with_context(|| format!("读取文章文件失败: {:?}", file_path))?;
        let article: FollowingArticle = serde_json::from_str(&data)
            .with_context(|| format!("解析文章文件失败: {:?}", file_path))?;

        if article.id != article_id {
            anyhow::bail!(
                "文件名 {} 与文章 id {} 不匹配",
                filename,
                article.id
            );
        }

        Ok(article)
    }

    /// 删除文章文件
    pub fn delete(&self, articles_dir: &Path) {
        let path = articles_dir.join(format!("{}.json", self.id));
        let _ = fs::remove_file(&path);
    }

    /// 标记为已读
    pub fn mark_as_read(&mut self) {
        if self.read.is_none() {
            self.read = Some(Utc::now());
        }
    }

    /// 标记为未读
    pub fn mark_as_unread(&mut self) {
        self.read = None;
    }
}

/// 发送给前端的文章快照
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowingArticleSnapshot {
    pub id: String,
    pub link: String,
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
    pub created: String,
    pub read: Option<String>,
    pub starred: Option<String>,
    pub video_filename: Option<String>,
    pub audio_filename: Option<String>,
    pub attachments: Option<Vec<String>>,
}

impl FollowingArticle {
    pub fn snapshot(&self) -> FollowingArticleSnapshot {
        FollowingArticleSnapshot {
            id: self.id.to_string(),
            link: self.link.clone(),
            title: self.title.clone(),
            content: self.content.clone(),
            summary: self.summary.clone(),
            created: self.created.to_rfc3339(),
            read: self.read.map(|d| d.to_rfc3339()),
            starred: self.starred.map(|d| d.to_rfc3339()),
            video_filename: self.video_filename.clone(),
            audio_filename: self.audio_filename.clone(),
            attachments: self.attachments.clone(),
        }
    }
}

// ============================================================
// 工具函数
// ============================================================

/// 清理文章链接
fn sanitize_article_link(link: &str) -> String {
    let mut s = link.to_string();
    if s.starts_with("http://127.0.0.1:") && s.len() > 22 {
        s = s[22..].to_string();
    }
    if s.starts_with("/ipfs/Qm") && s.len() > 52 {
        s = s[52..].to_string();
    }
    s
}

/// 从 HTML 中提取纯文本（简单实现）
///
/// 对标 Swift SwiftSoup.parse().text()
fn strip_html_tags(html: &str) -> String {
    // 简单正则方案，生产环境建议用 scraper crate
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    let text = re.replace_all(html, " ");
    // 合并多余空白
    let re_space = regex::Regex::new(r"\s+").unwrap();
    re_space.replace_all(&text, " ").trim().to_string()
}
```

---

## 5. Step 3：实现 Feed 解析器 (`helpers/feed.rs`)

### 5.1 对应 Swift 代码

```swift
// Planet/Helper/FeedUtils.swift
struct FeedUtils {
    static func findFeed(url: URL) async throws -> (feed: Data?, html: Document?)
    static func parseFeed(data: Data, url: URL) async throws -> (name, about, avatar, articles)
}
```

### 5.2 Rust 实现

创建文件 `src-tauri/src/helpers/feed.rs`：

```rust
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use feed_rs::parser;
use scraper::{Html, Selector};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::models::following_planet::PublicArticleInfo;

/// Feed 解析结果
pub struct ParsedFeed {
    pub name: Option<String>,
    pub about: Option<String>,
    pub avatar: Option<Vec<u8>>,
    pub articles: Option<Vec<PublicArticleInfo>>,
}

/// 从 URL 发现 Feed
///
/// 对标 Swift FeedUtils.findFeed(url:)
///
/// 返回: (feed_data, html_content)
///  - 如果 URL 直接是 Feed → 返回 (Some(data), None)
///  - 如果 URL 是 HTML 页面 → 查找 <link rel="alternate"> → 返回 (Some(feed_data), None)
///  - 如果找不到 Feed → 返回 (None, Some(html))
pub async fn find_feed(url: &str) -> Result<(Option<Vec<u8>>, Option<String>)> {
    let resp = reqwest::get(url).await?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("HTTP 请求失败: {}", status);
    }

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    let body = resp.bytes().await?;
    let body_vec = body.to_vec();

    // 检查是否直接是 Feed
    if is_feed_mime(&content_type) {
        return Ok((Some(body_vec), None));
    }

    // 尝试解析为 HTML，查找 <link rel="alternate">
    if content_type.contains("text/html") {
        let html_str = String::from_utf8_lossy(&body_vec).to_string();
        let document = Html::parse_document(&html_str);
        let selector = Selector::parse(r#"link[rel="alternate"]"#).unwrap();

        let mut available_feeds: Vec<(String, String)> = Vec::new();
        for element in document.select(&selector) {
            if let (Some(href), Some(feed_type)) =
                (element.value().attr("href"), element.value().attr("type"))
            {
                if is_feed_mime(feed_type) {
                    // 解析相对 URL
                    let feed_url = resolve_url(url, href);
                    available_feeds.push((feed_url, feed_type.to_string()));
                }
            }
        }

        if available_feeds.is_empty() {
            return Ok((None, Some(html_str)));
        }

        // 优先选择 JSON Feed
        let best_feed = available_feeds
            .iter()
            .find(|(_, mime)| mime.contains("json"))
            .or(available_feeds.first());

        if let Some((feed_url, _)) = best_feed {
            debug!("发现 Feed: {}", feed_url);
            let feed_resp = reqwest::get(feed_url).await?;
            if feed_resp.status().is_success() {
                let feed_data = feed_resp.bytes().await?.to_vec();
                return Ok((Some(feed_data), None));
            }
        }

        return Ok((None, Some(html_str)));
    }

    // 尝试直接作为 Feed 解析
    if parser::parse(&body_vec[..]).is_ok() {
        return Ok((Some(body_vec), None));
    }

    Ok((None, None))
}

/// 解析 Feed 数据
///
/// 对标 Swift FeedUtils.parseFeed(data:url:)
pub async fn parse_feed(data: &[u8], base_url: &str) -> Result<ParsedFeed> {
    let feed = parser::parse(data).with_context(|| "Feed 解析失败")?;

    let name = feed.title.map(|t| t.content);
    let about = feed.description.map(|d| d.content);

    // 解析文章
    let articles: Vec<PublicArticleInfo> = feed
        .entries
        .iter()
        .filter_map(|entry| {
            let link = entry
                .links
                .first()
                .map(|l| l.href.clone())
                .or_else(|| entry.id.clone().into())
                .unwrap_or_default();

            if link.is_empty() {
                return None;
            }

            let title = entry
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_else(|| "Untitled".to_string());

            let content = entry
                .content
                .as_ref()
                .and_then(|c| c.body.clone())
                .or_else(|| {
                    entry
                        .summary
                        .as_ref()
                        .map(|s| s.content.clone())
                })
                .unwrap_or_default();

            let created = entry
                .published
                .or(entry.updated)
                .unwrap_or_else(Utc::now);

            // 解析 link: 相对 URL → 绝对 URL
            let resolved_link = if link.starts_with("http://") || link.starts_with("https://") {
                sanitize_feed_link(&link)
            } else {
                sanitize_feed_link(&resolve_url(base_url, &link))
            };

            Some(PublicArticleInfo {
                id: Uuid::new_v4(),
                link: resolved_link,
                title,
                content: content.clone(),
                content_rendered: Some(content),
                created,
                has_video: Some(false),
                video_filename: None,
                has_audio: Some(false),
                audio_filename: None,
                audio_duration: None,
                audio_byte_length: None,
                attachments: None,
                hero_image: None,
                tags: None,
            })
        })
        .collect();

    // 尝试获取头像 (从 Feed 的 icon/logo)
    let avatar = if let Some(icon_url) = feed.icon.map(|i| i.uri).or(feed.logo.map(|l| l.uri)) {
        match reqwest::get(&icon_url).await {
            Ok(resp) if resp.status().is_success() => {
                resp.bytes().await.ok().map(|b| b.to_vec())
            }
            _ => None,
        }
    } else {
        None
    };

    Ok(ParsedFeed {
        name,
        about,
        avatar,
        articles: if articles.is_empty() {
            None
        } else {
            Some(articles)
        },
    })
}

/// 从 HTML 中查找 og:image 作为头像
pub async fn find_avatar_from_html(html: &str, base_url: &str) -> Option<Vec<u8>> {
    let document = Html::parse_document(html);

    // 1. 尝试 og:image
    let og_selector = Selector::parse(r#"meta[property="og:image"]"#).ok()?;
    if let Some(element) = document.select(&og_selector).next() {
        if let Some(url) = element.value().attr("content") {
            let full_url = resolve_url(base_url, url);
            if let Ok(resp) = reqwest::get(&full_url).await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        return Some(bytes.to_vec());
                    }
                }
            }
        }
    }

    // 2. 尝试 <link rel="icon">
    let icon_selector = Selector::parse(r#"link[rel="icon"], link[rel="apple-touch-icon"]"#).ok()?;
    for element in document.select(&icon_selector) {
        if let Some(href) = element.value().attr("href") {
            let full_url = resolve_url(base_url, href);
            if let Ok(resp) = reqwest::get(&full_url).await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        if !bytes.is_empty() {
                            return Some(bytes.to_vec());
                        }
                    }
                }
            }
        }
    }

    None
}

// ============================================================
// 工具函数
// ============================================================

fn is_feed_mime(mime: &str) -> bool {
    let m = mime.to_lowercase();
    m.contains("application/xml")
        || m.contains("text/xml")
        || m.contains("application/atom+xml")
        || m.contains("application/rss+xml")
        || m.contains("application/json")
        || m.contains("application/feed+json")
}

/// 解析相对 URL
fn resolve_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    match url::Url::parse(base) {
        Ok(base_url) => match base_url.join(relative) {
            Ok(resolved) => resolved.to_string(),
            Err(_) => relative.to_string(),
        },
        Err(_) => relative.to_string(),
    }
}

/// 清理 Feed 链接（保留路径部分）
fn sanitize_feed_link(link: &str) -> String {
    if let Ok(url) = url::Url::parse(link) {
        let path = url.path();
        let query = url.query().map(|q| format!("?{}", q)).unwrap_or_default();
        let fragment = url.fragment().map(|f| format!("#{}", f)).unwrap_or_default();
        // 对于 HTTP Feed，保留完整 URL
        return link.to_string();
    }
    link.to_string()
}
```

### 5.3 更新 `helpers/mod.rs`

```rust
pub mod paths;
pub mod net;
pub mod markdown;
pub mod feed;     // 新增
pub mod ens;      // 新增 (Step 4)
pub mod dotbit;   // 新增 (Step 5)
```

---

## 6. Step 4：实现 ENS 解析 (`helpers/ens.rs`)

### 6.1 对应 Swift 代码

```swift
// Planet/Helper/ENSUtils.swift
struct ENSUtils {
    static func isIPNS(_ str: String) -> Bool {
        // k51... (62 chars) 或 k2... (56 chars)
    }
    static func getCID(from contenthash: URL) async throws -> String? {
        // ipns:// → IPFS name/resolve → CID
        // ipfs:// → 直接返回 CID
    }
}
```

### 6.2 ENS 解析方案选择

ENS 解析需要从 `.eth` 域名获取 `contenthash`。原项目使用 `ENSDataKit`（Swift）直接调用 Ethereum 智能合约。

Rust 有两种方案：

| 方案 | 依赖 | 复杂度 | 说明 |
|------|------|--------|------|
| A. HTTP API | 仅 `reqwest` | 低 | 使用 eth.limo 或 enstate.rs 等公共 API |
| B. 直接合约调用 | `alloy` | 高 | 自己调用 ENS Registry + Resolver 合约 |

**推荐方案 A**（先用 HTTP API 快速实现）。

### 6.3 Rust 实现 — 方案 A：HTTP API

创建文件 `src-tauri/src/helpers/ens.rs`：

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::ipfs::daemon::IpfsDaemon;

// ============================================================
// IPNS 判断
// ============================================================

/// 判断一个字符串是否为 IPNS 地址
///
/// 对标 Swift ENSUtils.isIPNS()
pub fn is_ipns(s: &str) -> bool {
    if !s.starts_with('k') {
        return false;
    }
    // CIDv1 in libp2p-key codec: k51... (62 chars)
    if s.starts_with("k51") && s.len() == 62 {
        return true;
    }
    // CIDv1 in older format: k2... (56 chars)
    if s.starts_with("k2") && s.len() == 56 {
        return true;
    }
    false
}

// ============================================================
// ENS 解析
// ============================================================

/// ENS 解析结果
#[derive(Debug, Clone)]
pub struct EnsResolution {
    pub content_hash: Option<String>,
    pub address: Option<String>,
    pub avatar: Option<String>,
}

/// 解析 ENS 域名的 contenthash
///
/// 对标 Swift ENSDataKit 的 resolve()
///
/// 使用公共 HTTP API (enstate.rs) 进行解析
pub async fn resolve_ens(name: &str) -> Result<EnsResolution> {
    // 方案1: 使用 enstate.rs API (免费、可靠)
    let url = format!("https://enstate.rs/n/{}", name);
    let resp = reqwest::get(&url)
        .await
        .with_context(|| format!("ENS 解析请求失败: {}", name))?;

    if !resp.status().is_success() {
        anyhow::bail!("ENS 解析失败 (HTTP {}): {}", resp.status(), name);
    }

    let data: EnstateResponse = resp
        .json()
        .await
        .with_context(|| format!("解析 ENS 响应失败: {}", name))?;

    info!("ENS 解析成功: {} -> contenthash={:?}", name, data.contenthash);

    Ok(EnsResolution {
        content_hash: data.contenthash,
        address: data.address,
        avatar: data.avatar,
    })
}

/// enstate.rs API 响应结构
#[derive(Debug, Deserialize)]
struct EnstateResponse {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub contenthash: Option<String>,
    #[serde(default)]
    pub display: Option<String>,
}

// ============================================================
// contenthash → CID
// ============================================================

/// 从 contenthash 解析出 CID
///
/// 对标 Swift ENSUtils.getCID(from:)
///
/// contenthash 格式示例：
///   - "ipns://k51..." → 调用 IPFS resolve
///   - "ipfs://Qm..." → 直接返回 CID
///   - "ipfs://bafy..." → 直接返回 CID
///   - "k51..." → 当作 IPNS 解析
///   - "Qm..." → 当作 CID 直接返回
///   - "bafy..." → 当作 CID 直接返回
pub async fn resolve_contenthash_to_cid(
    contenthash: &str,
    daemon: &IpfsDaemon,
) -> Result<String> {
    let hash = contenthash.trim();

    // 已经是纯 CID
    if hash.starts_with("Qm") || hash.starts_with("bafy") {
        return Ok(hash.to_string());
    }

    // ipfs:// 协议
    if let Some(cid) = hash.strip_prefix("ipfs://") {
        return Ok(cid.to_string());
    }

    // ipns:// 协议 → 需要 IPFS resolve
    if let Some(ipns) = hash.strip_prefix("ipns://") {
        let cid = daemon
            .resolve_ipns_or_dnslink(ipns)
            .await
            .with_context(|| format!("IPNS 解析失败: {}", ipns))?;
        return Ok(cid);
    }

    // k51... 或 k2... → IPNS 地址
    if is_ipns(hash) {
        let cid = daemon
            .resolve_ipns_or_dnslink(hash)
            .await
            .with_context(|| format!("IPNS 解析失败: {}", hash))?;
        return Ok(cid);
    }

    anyhow::bail!("不支持的 contenthash 格式: {}", hash)
}

/// 组装 contenthash URL
///
/// 根据 contenthash 字符串判断是 ipns 还是 ipfs
pub fn contenthash_to_url(contenthash: &str) -> String {
    if contenthash.starts_with("k51") {
        format!("ipns://{}", contenthash)
    } else if contenthash.starts_with("Qm") || contenthash.starts_with("bafy") {
        format!("ipfs://{}", contenthash)
    } else {
        format!("ipfs://{}", contenthash)
    }
}
```

### 6.4 方案 B（可选）：使用 alloy 直接调用合约

如果将来需要脱离第三方 API，可以添加 `alloy` 依赖直接调用 ENS 合约。这里提供参考框架：

```rust
// 仅供参考，Phase 4 建议先用方案 A
// 需要在 Cargo.toml 添加:
// alloy = { version = "0.3", features = ["provider-http", "contract"] }

/*
use alloy::providers::{Provider, ProviderBuilder};
use alloy::primitives::Address;

const ENS_REGISTRY: &str = "0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e";
const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

pub async fn resolve_ens_onchain(name: &str) -> Result<EnsResolution> {
    let provider = ProviderBuilder::new().on_http(ETH_RPC_URL.parse()?);
    // ... 调用 ENS Registry → Resolver → contenthash
    // 这部分代码较复杂，建议在后续优化时实现
    todo!()
}
*/
```

---

## 7. Step 5：实现 .bit 域名解析 (`helpers/dotbit.rs`)

### 7.1 对应 Swift 代码

```swift
// Planet/Helper/DotBitKit.swift
class DotBitKit {
    static let indexerURL = URL(string: "https://indexer-v1.did.id")!
    func resolve(_ account: String) async -> DWebRecord? {
        // POST /v1/account/records { "account": "xxx.bit" }
        // 从 records 中查找 dweb.ipns 或 dweb.ipfs
    }
}
```

### 7.2 Rust 实现

创建文件 `src-tauri/src/helpers/dotbit.rs`：

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ============================================================
// 数据结构
// ============================================================

/// DWeb 记录类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DWebRecordType {
    Ipfs,
    Ipns,
}

/// DWeb 记录
#[derive(Debug, Clone)]
pub struct DWebRecord {
    pub record_type: DWebRecordType,
    pub value: String,
}

// ============================================================
// .bit API 响应结构
// ============================================================

#[derive(Debug, Deserialize)]
struct DotBitResponse {
    err_no: i32,
    #[serde(default)]
    err_msg: Option<String>,
    #[serde(default)]
    data: Option<DotBitData>,
}

#[derive(Debug, Deserialize)]
struct DotBitData {
    #[serde(default)]
    records: Vec<DotBitRecord>,
}

#[derive(Debug, Deserialize)]
struct DotBitRecord {
    key: String,
    value: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    ttl: Option<String>,
}

// ============================================================
// .bit 解析
// ============================================================

const DOTBIT_INDEXER_URL: &str = "https://indexer-v1.did.id";

/// 解析 .bit 域名的 DWeb 记录
///
/// 对标 Swift DotBitKit.resolve()
pub async fn resolve_dotbit(account: &str) -> Result<Option<DWebRecord>> {
    if account.len() <= 4 {
        return Ok(None);
    }

    let url = format!("{}/v1/account/records", DOTBIT_INDEXER_URL);

    #[derive(Serialize)]
    struct Request {
        account: String,
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&Request {
            account: account.to_string(),
        })
        .send()
        .await
        .with_context(|| format!(".bit 解析请求失败: {}", account))?;

    if !resp.status().is_success() {
        warn!(".bit 解析 HTTP 错误: {} for {}", resp.status(), account);
        return Ok(None);
    }

    let data: DotBitResponse = resp
        .json()
        .await
        .with_context(|| format!("解析 .bit 响应失败: {}", account))?;

    if data.err_no != 0 {
        warn!(
            ".bit API 错误: err_no={}, msg={:?} for {}",
            data.err_no, data.err_msg, account
        );
        return Ok(None);
    }

    let records = match data.data {
        Some(d) => d.records,
        None => return Ok(None),
    };

    debug!(".bit account {} has {} records", account, records.len());

    // 遍历 records，查找 dweb 记录
    // 优先 IPNS，其次 IPFS
    let mut ipns_record: Option<DWebRecord> = None;
    let mut ipfs_record: Option<DWebRecord> = None;

    for record in &records {
        if record.key.starts_with("dweb") {
            if record.key.ends_with("ipns") {
                ipns_record = Some(DWebRecord {
                    record_type: DWebRecordType::Ipns,
                    value: record.value.clone(),
                });
            }
            if record.key.ends_with("ipfs") {
                ipfs_record = Some(DWebRecord {
                    record_type: DWebRecordType::Ipfs,
                    value: record.value.clone(),
                });
            }
        }
    }

    // 优先返回 IPNS，因为可以更新
    if let Some(r) = ipns_record {
        info!(".bit {} resolved to IPNS: {}", account, r.value);
        return Ok(Some(r));
    }
    if let Some(r) = ipfs_record {
        info!(".bit {} resolved to IPFS: {}", account, r.value);
        return Ok(Some(r));
    }

    info!(".bit {} has no dweb records", account);
    Ok(None)
}
```

---

## 8. Step 6：实现关注流程 — follow (`models/following_planet.rs` 扩展)

### 8.1 对应 Swift 代码

```swift
// FollowingPlanetModel.swift
static func follow(link raw: String) async throws -> FollowingPlanetModel
static func followENS(ens: String) async throws -> FollowingPlanetModel
static func followDotBit(dotbit: String) async throws -> FollowingPlanetModel
static func followHTTP(link: String) async throws -> FollowingPlanetModel
static func followIPNSorDNSLink(name: String) async throws -> FollowingPlanetModel
static func getPublicPlanet(from cid: String) async throws -> PublicPlanetModel?
```

### 8.2 Rust 实现

在 `models/following_planet.rs` 中添加 `impl FollowingPlanet` 的扩展方法：

```rust
// ============================================================
// 在 following_planet.rs 底部添加以下 impl 块
// ============================================================

use crate::helpers::dotbit::{self, DWebRecordType};
use crate::helpers::ens;
use crate::helpers::feed;
use crate::ipfs::daemon::IpfsDaemon;

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
            Self::follow_ens(&link, daemon).await
        } else if link.ends_with(".bit") {
            Self::follow_dotbit(&link, daemon).await
        } else if link.to_lowercase().starts_with("http://")
            || link.to_lowercase().starts_with("https://")
        {
            Self::follow_http(&link).await
        } else {
            Self::follow_ipns_or_dnslink(&link, daemon).await
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
    async fn follow_ens(ens_name: &str, daemon: &IpfsDaemon) -> Result<Self> {
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

        let gateway = daemon.get_gateway();

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

            planet.ensure_directories()?;

            // 创建文章
            let articles: Vec<FollowingArticle> = public_planet
                .articles
                .iter()
                .map(|a| FollowingArticle::from_public_article(a, PlanetType::Ens))
                .collect();
            for article in &articles {
                article.save(&planet.articles_path())?;
            }
            planet.articles = articles;
            planet.articles.sort_by(|a, b| b.created.cmp(&a.created));

            planet.save()?;

            // 异步下载头像
            let _ = planet.download_avatar(&gateway).await;

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

        planet.ensure_directories()?;
        for article in &planet.articles {
            article.save(&planet.articles_path())?;
        }

        // 处理头像：ENS avatar > Feed avatar
        if let Some(avatar_url) = &resolution.avatar {
            if let Ok(resp) = reqwest::get(avatar_url).await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        let _ = std::fs::write(planet.avatar_path(), &bytes);
                    }
                }
            }
        } else if let Some(avatar_data) = feed_avatar_data {
            let _ = std::fs::write(planet.avatar_path(), &avatar_data);
        }

        planet.save()?;
        Ok(planet)
    }

    // ============================================================
    // followDotBit
    // ============================================================

    /// 关注 .bit 域名
    ///
    /// 对标 Swift FollowingPlanetModel.followDotBit()
    async fn follow_dotbit(dotbit_name: &str, daemon: &IpfsDaemon) -> Result<Self> {
        info!("关注 .bit: {}", dotbit_name);

        let dweb = dotbit::resolve_dotbit(dotbit_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!(".bit 没有 DWeb 记录: {}", dotbit_name))?;

        let cid = match dweb.record_type {
            DWebRecordType::Ipfs => dweb.value.clone(),
            DWebRecordType::Ipns => {
                daemon
                    .resolve_ipns_or_dnslink(&dweb.value)
                    .await
                    .with_context(|| format!(".bit IPNS 解析失败: {}", dweb.value))?
            }
        };

        info!(".bit {} → CID: {}", dotbit_name, cid);
        let gateway = daemon.get_gateway();

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
            planet.ensure_directories()?;

            let articles: Vec<FollowingArticle> = public_planet
                .articles
                .iter()
                .map(|a| FollowingArticle::from_public_article(a, PlanetType::DotBit))
                .collect();
            for article in &articles {
                article.save(&planet.articles_path())?;
            }
            planet.articles = articles;
            planet.articles.sort_by(|a, b| b.created.cmp(&a.created));

            let _ = planet.download_avatar(&gateway).await;
            planet.save()?;
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

        planet.ensure_directories()?;
        for article in &planet.articles {
            article.save(&planet.articles_path())?;
        }
        if let Some(avatar_data) = feed_avatar_data {
            let _ = std::fs::write(planet.avatar_path(), &avatar_data);
        }
        planet.save()?;
        Ok(planet)
    }

    // ============================================================
    // followHTTP
    // ============================================================

    /// 关注 HTTP RSS/Atom Feed
    ///
    /// 对标 Swift FollowingPlanetModel.followHTTP()
    async fn follow_http(link: &str) -> Result<Self> {
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

        planet.ensure_directories()?;

        if let Some(pub_articles) = &parsed.articles {
            let deduped = deduplicate_articles(pub_articles.clone());
            planet.articles = deduped
                .iter()
                .map(|a| FollowingArticle::from_public_article(a, PlanetType::Dns))
                .collect();
            planet.articles.sort_by(|a, b| b.created.cmp(&a.created));
        }

        for article in &planet.articles {
            article.save(&planet.articles_path())?;
        }

        // 下载 Feed 头像
        if let Some(avatar_data) = parsed.avatar {
            let _ = std::fs::write(planet.avatar_path(), &avatar_data);
        }

        planet.save()?;
        Ok(planet)
    }

    // ============================================================
    // followIPNSorDNSLink
    // ============================================================

    /// 关注 IPNS 或 DNSLink 地址
    ///
    /// 对标 Swift FollowingPlanetModel.followIPNSorDNSLink()
    async fn follow_ipns_or_dnslink(name: &str, daemon: &IpfsDaemon) -> Result<Self> {
        let planet_type = if ens::is_ipns(name) {
            PlanetType::Planet
        } else {
            PlanetType::DnsLink
        };

        info!("关注 {:?}: {}", planet_type, name);

        // 1. 解析 IPNS/DNSLink → CID
        let cid = daemon
            .resolve_ipns_or_dnslink(name)
            .await
            .with_context(|| format!("IPNS/DNSLink 解析失败: {}", name))?;

        info!("{} → CID: {}", name, cid);
        let gateway = daemon.get_gateway();

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
            planet.ensure_directories()?;

            let articles: Vec<FollowingArticle> = public_planet
                .articles
                .iter()
                .map(|a| FollowingArticle::from_public_article(a, planet_type))
                .collect();
            for article in &articles {
                article.save(&planet.articles_path())?;
            }
            planet.articles = articles;
            planet.articles.sort_by(|a, b| b.created.cmp(&a.created));

            let _ = planet.download_avatar(&gateway).await;
            planet.save()?;
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

        planet.ensure_directories()?;
        for article in &planet.articles {
            article.save(&planet.articles_path())?;
        }
        if let Some(avatar_data) = feed_avatar_data {
            let _ = std::fs::write(planet.avatar_path(), &avatar_data);
        }
        planet.save()?;
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
```

---

## 9. Step 7：实现更新流程 — update (`models/following_planet.rs` 扩展)

### 9.1 对应 Swift 代码

```swift
// FollowingPlanetModel.swift
func update() async throws {
    // 根据 planetType 分支处理
    switch planetType {
    case .planet, .dnslink:  // IPNS resolve → 比较 CID → 拉取更新
    case .ens:               // ENS resolve → contenthash → CID → 同上
    case .dotbit:            // .bit resolve → 同 ens
    case .dns:               // 重新拉取 Feed → updateArticles
    }
}
```

### 9.2 Rust 实现

在 `models/following_planet.rs` 中继续添加：

```rust
impl FollowingPlanet {
    // ============================================================
    // 更新入口
    // ============================================================

    /// 检查并拉取更新
    ///
    /// 对标 Swift FollowingPlanetModel.update()
    pub async fn update(&mut self, daemon: &IpfsDaemon) -> Result<()> {
        info!("更新 Planet: {} ({})", self.name, self.link);
        self.is_updating = true;

        let result = match self.planet_type {
            PlanetType::Planet | PlanetType::DnsLink => {
                self.update_ipns(daemon).await
            }
            PlanetType::Ens => {
                self.update_ens(daemon).await
            }
            PlanetType::DotBit => {
                self.update_dotbit(daemon).await
            }
            PlanetType::Dns => {
                self.update_http().await
            }
        };

        self.is_updating = false;
        result
    }

    // ============================================================
    // IPNS / DNSLink 更新
    // ============================================================

    async fn update_ipns(&mut self, daemon: &IpfsDaemon) -> Result<()> {
        let new_cid = daemon
            .resolve_ipns_or_dnslink(&self.link)
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

        let gateway = daemon.get_gateway();

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

            self.update_articles(&public_planet.articles, true)?;

            // 下载头像
            let _ = self.download_avatar(&gateway).await;

            self.cid = Some(new_cid);
            self.last_retrieved = Utc::now();
            self.save()?;
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
                self.update_articles(pub_articles, false)?;
            }

            if let Some(avatar_data) = parsed.avatar {
                let _ = std::fs::write(self.avatar_path(), &avatar_data);
            }

            self.save()?;
        }

        Ok(())
    }

    // ============================================================
    // ENS 更新
    // ============================================================

    async fn update_ens(&mut self, daemon: &IpfsDaemon) -> Result<()> {
        info!("更新 ENS Planet: {} ({})", self.name, self.link);

        // 1. 解析 ENS
        let resolution = ens::resolve_ens(&self.link).await?;

        // 更新钱包地址
        if let Some(addr) = &resolution.address {
            if self.wallet_address.as_deref() != Some(addr) {
                self.wallet_address = Some(addr.clone());
                self.wallet_address_resolved_at = Some(Utc::now());
                self.save()?;
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
        let gateway = daemon.get_gateway();

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

            self.update_articles(&public_planet.articles, true)?;

            // 头像：优先 ENS avatar, 然后 IPFS avatar
            if let Some(avatar_url) = &resolution.avatar {
                if let Ok(resp) = reqwest::get(avatar_url).await {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            let _ = std::fs::write(self.avatar_path(), &bytes);
                        }
                    }
                }
            } else {
                let _ = self.download_avatar(&gateway).await;
            }

            self.cid = Some(new_cid);
            self.last_retrieved = Utc::now();
            self.save()?;
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
                self.update_articles(pub_articles, false)?;
            }

            // 头像
            if let Some(avatar_url) = &resolution.avatar {
                if let Ok(resp) = reqwest::get(avatar_url).await {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            let _ = std::fs::write(self.avatar_path(), &bytes);
                        }
                    }
                }
            } else if let Some(avatar_data) = parsed.avatar {
                let _ = std::fs::write(self.avatar_path(), &avatar_data);
            }

            self.save()?;
        }

        Ok(())
    }

    // ============================================================
    // .bit 更新
    // ============================================================

    async fn update_dotbit(&mut self, daemon: &IpfsDaemon) -> Result<()> {
        info!("更新 .bit Planet: {} ({})", self.name, self.link);

        let dweb = dotbit::resolve_dotbit(&self.link)
            .await?
            .ok_or_else(|| anyhow::anyhow!(".bit 无 DWeb 记录: {}", self.link))?;

        let new_cid = match dweb.record_type {
            DWebRecordType::Ipfs => dweb.value.clone(),
            DWebRecordType::Ipns => {
                daemon.resolve_ipns_or_dnslink(&dweb.value).await?
            }
        };

        if self.cid.as_deref() == Some(&new_cid) {
            info!(".bit Planet {} 无更新", self.name);
            return Ok(());
        }

        let _ = daemon.pin(&new_cid).await;
        let gateway = daemon.get_gateway();

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

            self.update_articles(&public_planet.articles, true)?;
            let _ = self.download_avatar(&gateway).await;

            self.cid = Some(new_cid);
            self.last_retrieved = Utc::now();
            self.save()?;
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
                self.update_articles(pub_articles, false)?;
            }
            if let Some(avatar_data) = parsed.avatar {
                let _ = std::fs::write(self.avatar_path(), &avatar_data);
            }
            self.save()?;
        }

        Ok(())
    }

    // ============================================================
    // HTTP Feed 更新
    // ============================================================

    async fn update_http(&mut self) -> Result<()> {
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
            self.update_articles(pub_articles, false)?;
        }

        // 更新头像
        if let Some(avatar_data) = parsed.avatar {
            let _ = std::fs::write(self.avatar_path(), &avatar_data);
        } else {
            // 尝试从 HTML 页面获取头像
            if let Ok((_, Some(html))) = feed::find_feed(&self.link).await {
                if let Some(avatar_data) =
                    feed::find_avatar_from_html(&html, &self.link).await
                {
                    let _ = std::fs::write(self.avatar_path(), &avatar_data);
                }
            }
        }

        self.save()?;
        Ok(())
    }
}
```

---

## 10. Step 8：实现定时后台更新 (`store/mod.rs` 扩展)

### 10.1 对应 Swift 逻辑

原项目在 `PlanetStore` 中使用 Timer 定期调用所有 following planet 的 `update()`。

### 10.2 Rust 实现

在 `src-tauri/src/store/mod.rs` 中添加后台更新任务：

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};

use crate::models::following_planet::FollowingPlanet;
use crate::ipfs::daemon::IpfsDaemon;

/// 全局状态中添加 following_planets 列表
pub struct AppState {
    // ... 已有字段 ...
    pub following_planets: Arc<Mutex<Vec<FollowingPlanet>>>,
}

impl AppState {
    /// 加载所有 Following Planets
    pub fn load_following_planets(&self) -> Vec<FollowingPlanet> {
        let base = FollowingPlanet::following_planets_path();
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

    /// 启动后台更新定时器
    ///
    /// 对标 Swift PlanetStore 的定时更新逻辑
    pub fn start_background_updater(
        following: Arc<Mutex<Vec<FollowingPlanet>>>,
        daemon: Arc<IpfsDaemon>,
        app_handle: tauri::AppHandle,
    ) {
        tokio::spawn(async move {
            // 首次延迟 30 秒再开始
            tokio::time::sleep(Duration::from_secs(30)).await;

            // 每 5 分钟更新一次（可配置）
            let mut timer = interval(Duration::from_secs(5 * 60));

            loop {
                timer.tick().await;
                info!("开始后台更新所有 Following Planets...");

                let planets_snapshot = {
                    let lock = following.lock().await;
                    lock.iter()
                        .map(|p| (p.id, p.link.clone(), p.planet_type))
                        .collect::<Vec<_>>()
                };

                for (id, _link, _ptype) in &planets_snapshot {
                    let mut lock = following.lock().await;
                    if let Some(planet) = lock.iter_mut().find(|p| p.id == *id) {
                        match planet.update(&daemon).await {
                            Ok(_) => {
                                info!("更新完成: {}", planet.name);
                                // 通知前端
                                let _ = app_handle.emit_all(
                                    "following-updated",
                                    serde_json::json!({
                                        "id": planet.id.to_string(),
                                        "name": planet.name,
                                    }),
                                );
                            }
                            Err(e) => {
                                warn!("更新失败 {}: {}", planet.name, e);
                            }
                        }
                    }
                }

                info!("后台更新完成");
            }
        });
    }
}
```

### 10.3 在 `main.rs` 中启动后台更新

```rust
// 在 Tauri setup 中
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();

            // 加载 Following Planets
            let planets = state.load_following_planets();
            let following = state.following_planets.clone();
            {
                let mut lock = following.blocking_lock();
                *lock = planets;
            }

            // 启动后台更新
            let daemon = state.daemon.clone();
            AppState::start_background_updater(
                following.clone(),
                daemon,
                handle,
            );

            Ok(())
        })
        // ...
}
```

---

## 11. Step 9：注册 Tauri Commands (`commands/planet.rs` 扩展)

### 11.1 新增 Tauri Commands

在 `src-tauri/src/commands/planet.rs` 中添加以下命令：

```rust
use crate::models::following_planet::{FollowingPlanet, FollowingPlanetSnapshot};
use crate::models::following_article::FollowingArticleSnapshot;
use crate::store::AppState;

// ============================================================
// 关注
// ============================================================

/// 关注一个 Planet / ENS / Feed
#[tauri::command]
pub async fn planet_follow(
    link: String,
    state: tauri::State<'_, AppState>,
) -> Result<FollowingPlanetSnapshot, String> {
    let daemon = state.daemon.clone();
    let existing_links: Vec<String> = {
        let lock = state.following_planets.lock().await;
        lock.iter().map(|p| p.link.clone()).collect()
    };

    match FollowingPlanet::follow(&link, &existing_links, &daemon).await {
        Ok(planet) => {
            let snapshot = planet.snapshot();
            let mut lock = state.following_planets.lock().await;
            lock.insert(0, planet);
            Ok(snapshot)
        }
        Err(e) => Err(format!("关注失败: {}", e)),
    }
}

/// 取消关注
#[tauri::command]
pub async fn planet_unfollow(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut lock = state.following_planets.lock().await;

    if let Some(pos) = lock.iter().position(|p| p.id == uuid) {
        let planet = lock.remove(pos);
        planet.delete().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("未找到该 Planet".to_string())
    }
}

// ============================================================
// 列表查询
// ============================================================

/// 获取所有关注的 Planet 列表
#[tauri::command]
pub async fn following_list(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FollowingPlanetSnapshot>, String> {
    let lock = state.following_planets.lock().await;
    Ok(lock.iter().map(|p| p.snapshot()).collect())
}

/// 获取某个关注 Planet 的文章列表
#[tauri::command]
pub async fn following_articles(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FollowingArticleSnapshot>, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let lock = state.following_planets.lock().await;

    if let Some(planet) = lock.iter().find(|p| p.id == uuid) {
        Ok(planet.articles.iter().map(|a| a.snapshot()).collect())
    } else {
        Err("未找到该 Planet".to_string())
    }
}

/// 获取单篇文章详情
#[tauri::command]
pub async fn following_article_get(
    planet_id: String,
    article_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<FollowingArticleSnapshot, String> {
    let p_uuid = uuid::Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let a_uuid = uuid::Uuid::parse_str(&article_id).map_err(|e| e.to_string())?;

    let mut lock = state.following_planets.lock().await;
    if let Some(planet) = lock.iter_mut().find(|p| p.id == p_uuid) {
        if let Some(article) = planet.articles.iter_mut().find(|a| a.id == a_uuid) {
            // 标记为已读
            article.mark_as_read();
            let _ = article.save(&planet.articles_path());
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
#[tauri::command]
pub async fn following_update(
    id: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<FollowingPlanetSnapshot, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let daemon = state.daemon.clone();

    let mut lock = state.following_planets.lock().await;
    if let Some(planet) = lock.iter_mut().find(|p| p.id == uuid) {
        planet.update(&daemon).await.map_err(|e| e.to_string())?;

        let _ = app_handle.emit_all(
            "following-updated",
            serde_json::json!({ "id": planet.id.to_string() }),
        );

        Ok(planet.snapshot())
    } else {
        Err("未找到该 Planet".to_string())
    }
}

/// 更新所有 Following Planets
#[tauri::command]
pub async fn following_update_all(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let daemon = state.daemon.clone();

    let ids: Vec<uuid::Uuid> = {
        let lock = state.following_planets.lock().await;
        lock.iter().map(|p| p.id).collect()
    };

    for id in ids {
        let mut lock = state.following_planets.lock().await;
        if let Some(planet) = lock.iter_mut().find(|p| p.id == id) {
            match planet.update(&daemon).await {
                Ok(_) => {
                    let _ = app_handle.emit_all(
                        "following-updated",
                        serde_json::json!({
                            "id": planet.id.to_string(),
                            "name": planet.name.clone(),
                        }),
                    );
                }
                Err(e) => {
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
pub async fn following_article_mark_read(
    planet_id: String,
    article_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let p_uuid = uuid::Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let a_uuid = uuid::Uuid::parse_str(&article_id).map_err(|e| e.to_string())?;

    let mut lock = state.following_planets.lock().await;
    if let Some(planet) = lock.iter_mut().find(|p| p.id == p_uuid) {
        if let Some(article) = planet.articles.iter_mut().find(|a| a.id == a_uuid) {
            article.mark_as_read();
            article.save(&planet.articles_path()).map_err(|e| e.to_string())?;
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
pub async fn following_article_mark_unread(
    planet_id: String,
    article_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let p_uuid = uuid::Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let a_uuid = uuid::Uuid::parse_str(&article_id).map_err(|e| e.to_string())?;

    let mut lock = state.following_planets.lock().await;
    if let Some(planet) = lock.iter_mut().find(|p| p.id == p_uuid) {
        if let Some(article) = planet.articles.iter_mut().find(|a| a.id == a_uuid) {
            article.mark_as_unread();
            article.save(&planet.articles_path()).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("未找到该文章".to_string())
        }
    } else {
        Err("未找到该 Planet".to_string())
    }
}
```

### 11.2 在 `main.rs` 中注册命令

```rust
// main.rs
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        // ... 已有命令 ...
        // Phase 4 新增
        commands::planet::planet_follow,
        commands::planet::planet_unfollow,
        commands::planet::following_list,
        commands::planet::following_articles,
        commands::planet::following_article_get,
        commands::planet::following_update,
        commands::planet::following_update_all,
        commands::planet::following_article_mark_read,
        commands::planet::following_article_mark_unread,
    ])
```

---

## 12. Step 10：前端实现

### 12.1 类型定义

创建 `src/types/following.ts`：

```typescript
export interface FollowingPlanet {
  id: string;
  planetType: number;  // 0=planet, 1=ens, 2=dnslink, 3=dns, 4=dotbit
  name: string;
  about: string;
  link: string;
  cid: string | null;
  created: string;
  updated: string;
  lastRetrieved: string;
  isUpdating: boolean;
  articleCount: number;
  unreadCount: number;
  hasAvatar: boolean;
}

export interface FollowingArticle {
  id: string;
  link: string;
  title: string;
  content: string;
  summary: string | null;
  created: string;
  read: string | null;
  starred: string | null;
  videoFilename: string | null;
  audioFilename: string | null;
  attachments: string[] | null;
}

export type PlanetType = 'planet' | 'ens' | 'dnslink' | 'dns' | 'dotbit';

export function planetTypeName(type: number): PlanetType {
  switch (type) {
    case 0: return 'planet';
    case 1: return 'ens';
    case 2: return 'dnslink';
    case 3: return 'dns';
    case 4: return 'dotbit';
    default: return 'planet';
  }
}
```

### 12.2 FollowPlanetDialog 组件

创建 `src/components/FollowPlanetDialog.tsx`：

```tsx
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import type { FollowingPlanet } from '../types/following';

interface FollowPlanetDialogProps {
  open: boolean;
  onClose: () => void;
  onFollowed: (planet: FollowingPlanet) => void;
}

export function FollowPlanetDialog({ open, onClose, onFollowed }: FollowPlanetDialogProps) {
  const [link, setLink] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const handleFollow = async () => {
    if (!link.trim()) return;
    setLoading(true);
    setError(null);

    try {
      const planet = await invoke<FollowingPlanet>('planet_follow', { link: link.trim() });
      onFollowed(planet);
      setLink('');
      onClose();
    } catch (e: any) {
      setError(typeof e === 'string' ? e : e.message || '关注失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-[480px]">
        <h2 className="text-lg font-semibold mb-4">关注 Planet</h2>

        <input
          type="text"
          value={link}
          onChange={(e) => setLink(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleFollow()}
          placeholder="输入 IPNS 地址、ENS 域名、.bit 域名或 RSS Feed URL"
          className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600
                     rounded-md bg-white dark:bg-gray-700 text-sm
                     focus:outline-none focus:ring-2 focus:ring-blue-500"
          disabled={loading}
          autoFocus
        />

        <p className="text-xs text-gray-500 mt-2">
          支持: k51... (IPNS) · vitalik.eth (ENS) · example.bit · https://blog.example.com/feed.xml
        </p>

        {error && (
          <p className="text-sm text-red-500 mt-2">{error}</p>
        )}

        <div className="flex justify-end gap-2 mt-4">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm rounded-md border border-gray-300
                       dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700"
            disabled={loading}
          >
            取消
          </button>
          <button
            onClick={handleFollow}
            className="px-4 py-2 text-sm rounded-md bg-blue-500 text-white
                       hover:bg-blue-600 disabled:opacity-50"
            disabled={loading || !link.trim()}
          >
            {loading ? '关注中...' : '关注'}
          </button>
        </div>
      </div>
    </div>
  );
}
```

### 12.3 FollowingList 组件

创建 `src/components/FollowingList.tsx`：

```tsx
import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import type { FollowingPlanet } from '../types/following';
import { planetTypeName } from '../types/following';

interface FollowingListProps {
  onSelectPlanet: (planet: FollowingPlanet) => void;
  selectedId: string | null;
}

export function FollowingList({ onSelectPlanet, selectedId }: FollowingListProps) {
  const [planets, setPlanets] = useState<FollowingPlanet[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchList = useCallback(async () => {
    try {
      const list = await invoke<FollowingPlanet[]>('following_list');
      setPlanets(list);
    } catch (e) {
      console.error('获取关注列表失败:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchList();

    // 监听后台更新事件
    const unlisten = listen('following-updated', () => {
      fetchList();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchList]);

  const handleUpdateAll = async () => {
    try {
      await invoke('following_update_all');
    } catch (e) {
      console.error('更新失败:', e);
    }
  };

  if (loading) {
    return <div className="p-4 text-sm text-gray-500">加载中...</div>;
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between p-3 border-b border-gray-200 dark:border-gray-700">
        <h3 className="text-sm font-semibold">关注 ({planets.length})</h3>
        <button
          onClick={handleUpdateAll}
          className="text-xs text-blue-500 hover:text-blue-700"
          title="更新所有"
        >
          ⟳ 更新全部
        </button>
      </div>

      <div className="flex-1 overflow-y-auto">
        {planets.map((planet) => (
          <div
            key={planet.id}
            onClick={() => onSelectPlanet(planet)}
            className={`flex items-center gap-3 px-3 py-2 cursor-pointer border-b
                        border-gray-100 dark:border-gray-700/50 hover:bg-gray-50
                        dark:hover:bg-gray-800 ${
                          selectedId === planet.id
                            ? 'bg-blue-50 dark:bg-blue-900/20'
                            : ''
                        }`}
          >
            {/* 头像 */}
            <div className="w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700
                            flex items-center justify-center text-xs font-semibold shrink-0">
              {planet.name.charAt(0).toUpperCase()}
            </div>

            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-1">
                <span className="text-sm font-medium truncate">{planet.name}</span>
                {planet.isUpdating && (
                  <span className="text-xs text-blue-500 animate-pulse">⟳</span>
                )}
              </div>
              <div className="text-xs text-gray-500 truncate">
                <span className="uppercase">{planetTypeName(planet.planetType)}</span>
                {' · '}
                {planet.articleCount} 篇
                {planet.unreadCount > 0 && (
                  <span className="text-blue-500 font-medium ml-1">
                    ({planet.unreadCount} 未读)
                  </span>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
```

### 12.4 FollowingArticleList 组件

创建 `src/components/FollowingArticleList.tsx`：

```tsx
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import type { FollowingPlanet, FollowingArticle } from '../types/following';

interface FollowingArticleListProps {
  planet: FollowingPlanet;
  onSelectArticle: (article: FollowingArticle) => void;
  selectedArticleId: string | null;
}

export function FollowingArticleList({
  planet,
  onSelectArticle,
  selectedArticleId,
}: FollowingArticleListProps) {
  const [articles, setArticles] = useState<FollowingArticle[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    invoke<FollowingArticle[]>('following_articles', { id: planet.id })
      .then(setArticles)
      .catch((e) => console.error('获取文章列表失败:', e))
      .finally(() => setLoading(false));
  }, [planet.id]);

  const handleUpdate = async () => {
    try {
      await invoke('following_update', { id: planet.id });
      // 重新加载文章列表
      const updated = await invoke<FollowingArticle[]>('following_articles', {
        id: planet.id,
      });
      setArticles(updated);
    } catch (e) {
      console.error('更新失败:', e);
    }
  };

  const handleUnfollow = async () => {
    if (!confirm(`确认取消关注 "${planet.name}"？`)) return;
    try {
      await invoke('planet_unfollow', { id: planet.id });
      window.location.reload(); // 简单刷新
    } catch (e) {
      console.error('取消关注失败:', e);
    }
  };

  if (loading) {
    return <div className="p-4 text-sm text-gray-500">加载中...</div>;
  }

  return (
    <div className="flex flex-col h-full">
      {/* 顶部操作栏 */}
      <div className="flex items-center justify-between p-3 border-b border-gray-200 dark:border-gray-700">
        <div>
          <h3 className="text-sm font-semibold">{planet.name}</h3>
          <p className="text-xs text-gray-500">{articles.length} 篇文章</p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={handleUpdate}
            className="text-xs px-2 py-1 rounded border border-gray-300
                       dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700"
          >
            更新
          </button>
          <button
            onClick={handleUnfollow}
            className="text-xs px-2 py-1 rounded border border-red-300
                       text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
          >
            取消关注
          </button>
        </div>
      </div>

      {/* 文章列表 */}
      <div className="flex-1 overflow-y-auto">
        {articles.map((article) => (
          <div
            key={article.id}
            onClick={() => onSelectArticle(article)}
            className={`px-3 py-2 cursor-pointer border-b border-gray-100
                        dark:border-gray-700/50 hover:bg-gray-50
                        dark:hover:bg-gray-800 ${
                          selectedArticleId === article.id
                            ? 'bg-blue-50 dark:bg-blue-900/20'
                            : ''
                        } ${!article.read ? 'font-semibold' : ''}`}
          >
            <div className="text-sm truncate">{article.title}</div>
            {article.summary && (
              <div className="text-xs text-gray-500 truncate mt-0.5">
                {article.summary}
              </div>
            )}
            <div className="text-xs text-gray-400 mt-0.5">
              {new Date(article.created).toLocaleDateString()}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
```

### 12.5 FollowingArticleDetail 组件

创建 `src/components/FollowingArticleDetail.tsx`：

```tsx
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import type { FollowingArticle } from '../types/following';

interface FollowingArticleDetailProps {
  planetId: string;
  article: FollowingArticle;
}

export function FollowingArticleDetail({ planetId, article }: FollowingArticleDetailProps) {
  const [fullArticle, setFullArticle] = useState<FollowingArticle | null>(null);

  useEffect(() => {
    invoke<FollowingArticle>('following_article_get', {
      planetId,
      articleId: article.id,
    })
      .then(setFullArticle)
      .catch((e) => console.error('获取文章详情失败:', e));
  }, [planetId, article.id]);

  const displayArticle = fullArticle || article;

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <article className="max-w-3xl mx-auto">
        <h1 className="text-2xl font-bold mb-2">{displayArticle.title}</h1>

        <div className="text-sm text-gray-500 mb-6">
          {new Date(displayArticle.created).toLocaleString()}
          {displayArticle.read && (
            <span className="ml-2 text-green-500">✓ 已读</span>
          )}
        </div>

        {/* 音频播放器 */}
        {displayArticle.audioFilename && (
          <div className="mb-4 p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
            <audio controls className="w-full">
              <source src={displayArticle.audioFilename} />
            </audio>
          </div>
        )}

        {/* 视频播放器 */}
        {displayArticle.videoFilename && (
          <div className="mb-4">
            <video controls className="w-full rounded-lg">
              <source src={displayArticle.videoFilename} />
            </video>
          </div>
        )}

        {/* 文章内容 */}
        <div
          className="prose dark:prose-invert max-w-none"
          dangerouslySetInnerHTML={{ __html: displayArticle.content }}
        />

        {/* 附件 */}
        {displayArticle.attachments && displayArticle.attachments.length > 0 && (
          <div className="mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
            <h3 className="text-sm font-semibold mb-2">附件</h3>
            <ul className="text-sm">
              {displayArticle.attachments.map((att, i) => (
                <li key={i} className="text-blue-500 hover:underline">
                  <a href={att} target="_blank" rel="noopener noreferrer">
                    {att}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        )}
      </article>
    </div>
  );
}
```

---

## 13. Step 11：测试与调试

### 13.1 单元测试

```rust
// src-tauri/src/helpers/ens.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ipns() {
        // k51... (62 chars)
        assert!(is_ipns("k51qzi5uqu5dgutgmsbx7rlff2kj1coemc7tgrchii04upey7s75btk2dpceeu"));
        // k2... (56 chars)
        assert!(is_ipns("k2k4r8jx63q4e5n7s0gu5b8wq0m7j2a1k4l9n3p5h7d8f2c6"));
        // 太短
        assert!(!is_ipns("k51short"));
        // 不以 k 开头
        assert!(!is_ipns("Qm1234567890"));
    }

    #[tokio::test]
    async fn test_resolve_ens() {
        // 注意: 需要网络访问
        let result = resolve_ens("vitalik.eth").await;
        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(res.content_hash.is_some());
    }
}

// src-tauri/src/helpers/dotbit.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_dotbit() {
        // 注意: 需要网络访问且 .bit 服务在线
        let result = resolve_dotbit("test.bit").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_short_account() {
        // 太短的账户应返回 None
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(resolve_dotbit("ab"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}

// src-tauri/src/helpers/feed.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_feed_mime() {
        assert!(is_feed_mime("application/rss+xml"));
        assert!(is_feed_mime("application/atom+xml"));
        assert!(is_feed_mime("application/json"));
        assert!(is_feed_mime("application/feed+json"));
        assert!(is_feed_mime("text/xml"));
        assert!(!is_feed_mime("text/html"));
        assert!(!is_feed_mime("image/png"));
    }

    #[test]
    fn test_resolve_url() {
        assert_eq!(
            resolve_url("https://example.com/blog/", "/feed.xml"),
            "https://example.com/feed.xml"
        );
        assert_eq!(
            resolve_url("https://example.com/blog/", "feed.xml"),
            "https://example.com/blog/feed.xml"
        );
        assert_eq!(
            resolve_url("https://example.com", "https://other.com/feed"),
            "https://other.com/feed"
        );
    }

    #[tokio::test]
    async fn test_parse_rss_feed() {
        let rss = r#"<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Blog</title>
            <description>A test blog</description>
            <item>
              <title>First Post</title>
              <link>https://example.com/post1</link>
              <description>Hello world</description>
              <pubDate>Mon, 01 Jan 2024 00:00:00 GMT</pubDate>
            </item>
          </channel>
        </rss>"#;

        let result = parse_feed(rss.as_bytes(), "https://example.com").await;
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.name, Some("Test Blog".to_string()));
        assert!(parsed.articles.is_some());
        assert_eq!(parsed.articles.unwrap().len(), 1);
    }
}

// src-tauri/src/models/following_planet.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_link() {
        assert_eq!(
            sanitize_link("http://127.0.0.1:18181/ipfs/Qm.../path"),
            "/ipfs/Qm.../path"  // 移除网关前缀
        );
        assert_eq!(
            sanitize_link("/some/path"),
            "/some/path"
        );
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
                has_video: None, video_filename: None,
                has_audio: None, audio_filename: None,
                audio_duration: None, audio_byte_length: None,
                attachments: None, hero_image: None, tags: None,
            },
            PublicArticleInfo {
                id: Uuid::new_v4(),
                link: "/post1".to_string(),  // 重复
                title: "Post 1 Dup".to_string(),
                content: "".to_string(),
                content_rendered: None,
                created: Utc::now(),
                has_video: None, video_filename: None,
                has_audio: None, audio_filename: None,
                audio_duration: None, audio_byte_length: None,
                attachments: None, hero_image: None, tags: None,
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
```

### 13.2 集成测试清单

| # | 测试场景 | 操作 | 预期结果 |
|---|---------|------|---------|
| 1 | 关注 IPNS 地址 | 输入 `k51qzi5uqu5dg...` | 成功获取 planet.json，显示文章列表 |
| 2 | 关注 ENS 域名 | 输入 `vitalik.eth` | ENS 解析成功，获取内容，显示在列表中 |
| 3 | 关注 .bit 域名 | 输入 `example.bit` | .bit 解析成功（如有 dweb 记录） |
| 4 | 关注 HTTP Feed | 输入 `https://blog.rust-lang.org/feed.xml` | Feed 解析成功，文章列表正确 |
| 5 | 关注 DNSLink | 输入 `docs.ipfs.tech` | DNSLink 解析 → CID → planet.json |
| 6 | 重复关注 | 关注已关注的地址 | 返回错误："已关注该地址" |
| 7 | 取消关注 | 点击取消关注按钮 | Planet 从列表中移除，数据目录被删除 |
| 8 | 手动更新 | 点击 Update 按钮 | 检查新 CID，拉取新文章 |
| 9 | 自动更新 | 等待 5 分钟 | 后台自动更新，前端收到通知刷新 |
| 10 | 文章已读 | 点击打开文章 | 文章标记为已读，未读计数减少 |
| 11 | 持久化 | 关闭并重新打开应用 | Following 列表和文章不丢失 |
| 12 | 无网络 | 断网后更新 | 优雅失败，显示错误提示 |
| 13 | 文章内容渲染 | 查看 Markdown/HTML 文章 | 内容正确渲染 |

### 13.3 调试技巧

```bash
# 查看 IPFS name resolve 是否正常
curl http://127.0.0.1:5001/api/v0/name/resolve?arg=k51qzi5uqu5dg...

# 查看 planet.json
curl http://127.0.0.1:8080/ipfs/<CID>/planet.json

# 测试 ENS API
curl https://enstate.rs/n/vitalik.eth

# 测试 .bit API
curl -X POST https://indexer-v1.did.id/v1/account/records \
  -H "Content-Type: application/json" \
  -d '{"account": "test.bit"}'

# 查看 Following 目录内容
ls -la ~/.planet/Following/
```

---

## 14. 文件清单

### 新增文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/models/following_planet.rs` | FollowingPlanet 数据模型 + follow/update 逻辑 |
| `src-tauri/src/models/following_article.rs` | FollowingArticle 数据模型 |
| `src-tauri/src/helpers/feed.rs` | RSS/Atom/JSON Feed 解析器 |
| `src-tauri/src/helpers/ens.rs` | ENS 域名解析 |
| `src-tauri/src/helpers/dotbit.rs` | .bit 域名解析 |
| `src/types/following.ts` | 前端 TypeScript 类型定义 |
| `src/components/FollowPlanetDialog.tsx` | 关注 Planet 弹窗 |
| `src/components/FollowingList.tsx` | 关注列表组件 |
| `src/components/FollowingArticleList.tsx` | 文章列表组件 |
| `src/components/FollowingArticleDetail.tsx` | 文章详情组件 |

### 修改文件

| 文件 | 修改内容 |
|------|---------|
| `src-tauri/Cargo.toml` | 添加 `feed-rs`, `scraper`, `url` 依赖 |
| `src-tauri/src/models/mod.rs` | 添加 `following_planet`, `following_article` 模块 |
| `src-tauri/src/helpers/mod.rs` | 添加 `feed`, `ens`, `dotbit` 模块 |
| `src-tauri/src/commands/planet.rs` | 添加 Phase 4 Tauri 命令 |
| `src-tauri/src/store/mod.rs` | 添加 `following_planets` 状态和后台更新 |
| `src-tauri/src/main.rs` | 注册新命令，启动后台更新器 |

---

## 15. Swift → Rust 对照表

| 功能 | Swift 文件/方法 | Rust 文件/方法 |
|------|----------------|---------------|
| Planet 类型枚举 | `PlanetType` enum | `PlanetType` enum |
| 关注入口路由 | `follow(link:)` | `FollowingPlanet::follow()` |
| 关注 ENS | `followENS(ens:)` | `FollowingPlanet::follow_ens()` |
| 关注 .bit | `followDotBit(dotbit:)` | `FollowingPlanet::follow_dotbit()` |
| 关注 HTTP | `followHTTP(link:)` | `FollowingPlanet::follow_http()` |
| 关注 IPNS/DNSLink | `followIPNSorDNSLink(name:)` | `FollowingPlanet::follow_ipns_or_dnslink()` |
| 获取远端 Planet | `getPublicPlanet(from:)` | `FollowingPlanet::get_public_planet()` |
| 更新逻辑 | `update()` | `FollowingPlanet::update()` |
| 文章增量更新 | `updateArticles()` | `FollowingPlanet::update_articles()` |
| 下载头像 | `refreshIcon()` | `FollowingPlanet::download_avatar()` |
| ENS 判断 | `ENSUtils.isIPNS()` | `ens::is_ipns()` |
| ENS 解析 | `ENSDataClient.resolve()` | `ens::resolve_ens()` |
| contenthash→CID | `ENSUtils.getCID(from:)` | `ens::resolve_contenthash_to_cid()` |
| .bit 解析 | `DotBitKit.resolve()` | `dotbit::resolve_dotbit()` |
| Feed 发现 | `FeedUtils.findFeed()` | `feed::find_feed()` |
| Feed 解析 | `FeedUtils.parseFeed()` | `feed::parse_feed()` |
| Feed MIME 检测 | `FeedUtils.isFeed(mime:)` | `feed::is_feed_mime()` |
| HTML 头像提取 | `FeedUtils.findAvatarFromHTML*()` | `feed::find_avatar_from_html()` |
| 文章摘要提取 | `FollowingArticleModel.extractSummary()` | `FollowingArticle::extract_summary()` |
| 文章序列化 | `FollowingArticleModel.from(publicArticle:)` | `FollowingArticle::from_public_article()` |
| 去重 | `deduplicate()` | `deduplicate_articles()` |
| Link 清理 | 内联 `dropFirst(22)` 等 | `sanitize_link()` |
| 保存/加载/删除 | `save()` / `load(from:)` / `delete()` | `save()` / `load()` / `delete()` |
| 持久化路径 | `followingPlanetsPath()` | `FollowingPlanet::following_planets_path()` |
| 定时更新 | `PlanetStore` Timer | `AppState::start_background_updater()` |
| Keychain | `KeychainHelper` | Phase 3 已实现 `keyring-rs` |

---

## 16. 执行顺序总结

```
Phase 4 执行顺序：

Week 1:
  ┌─ Step 1: FollowingPlanet 数据模型 (Day 1)
  ├─ Step 2: FollowingArticle 数据模型 (Day 1)
  ├─ Step 3: Feed 解析器 (Day 2)
  ├─ Step 4: ENS 解析 (Day 2-3)
  ├─ Step 5: .bit 解析 (Day 3)
  └─ 里程碑: 所有 helper 和 model 编译通过 ✓

Week 2:
  ┌─ Step 6: follow 关注流程 (Day 4-5)
  ├─ Step 7: update 更新流程 (Day 5-6)
  ├─ Step 8: 后台更新定时器 (Day 6)
  ├─ Step 9: Tauri Commands (Day 7)
  ├─ Step 10: 前端实现 (Day 7-8)
  ├─ Step 11: 测试与调试 (Day 8-10)
  └─ 里程碑: 端到端关注/更新流程跑通 ✓

验收检查点:
  □ 输入 IPNS 地址 → 成功关注并拉取到 planet.json 和文章
  □ 输入 ENS 域名 (如 vitalik.eth) → 成功解析并关注
  □ 输入 HTTP Feed URL → 成功解析 RSS/Atom 并显示文章
  □ 文章内容在 ArticleDetail 中正确渲染
  □ 手动点击 Update → 检查并拉取新内容
  □ 后台定时器自动更新，前端收到通知并刷新
  □ 关闭重开后 Following 列表和已拉取的文章不丢失
  □ 取消关注后数据目录被清理
  □ 文章已读/未读状态正确维护
```

---

## 17. 常见问题与注意事项

### 17.1 IPNS 解析超时

IPNS 解析可能非常慢（30 秒以上），原因：
- 本地 IPFS 节点刚启动，DHT 还未稳定
- 被解析的 IPNS 密钥很少被访问，DHT 中的记录稀少

**解决方案**：
1. 在 `resolve_ipns_or_dnslink` 中设置合理的超时时间（建议 60 秒）
2. 在 UI 中显示进度提示（"正在解析 IPNS，可能需要较长时间..."）
3. 考虑添加公共 IPFS Gateway 作为备选解析路径

```rust
// 示例：带超时的 IPNS 解析
use tokio::time::timeout;

pub async fn resolve_ipns_with_timeout(
    daemon: &IpfsDaemon,
    name: &str,
    secs: u64,
) -> Result<String> {
    match timeout(
        Duration::from_secs(secs),
        daemon.resolve_ipns_or_dnslink(name),
    ).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!("IPNS 解析超时 ({}s): {}", secs, name),
    }
}
```

### 17.2 ENS API 可用性

`enstate.rs` 是免费公共服务，可能有速率限制。备选方案：

| API | URL | 说明 |
|-----|-----|------|
| enstate.rs | `https://enstate.rs/n/{name}` | 推荐，免费 |
| ens.domains | `https://api.ens.domains/v1/...` | 官方 API |
| eth.limo | 通过 IPFS Gateway | 直接访问 ENS 内容 |

### 17.3 Feed 解析兼容性

不同网站的 RSS/Atom Feed 格式差异较大，常见问题：
- 日期格式不标准 → `feed-rs` 已内置宽松解析
- 相对 URL → `resolve_url()` 函数处理
- 编码问题 → 使用 `String::from_utf8_lossy`
- Feed 内容可能是 HTML 也可能是纯文本 → 摘要提取需兼容

### 17.4 并发安全

`following_planets` 使用 `Arc<Mutex<Vec<FollowingPlanet>>>` 保护：
- 后台更新和前端请求可能同时访问 → Mutex 确保安全
- 更新单个 Planet 时持有锁 → 其他操作需要等待
- 如果更新耗时太长导致 UI 卡顿，可以考虑：
  1. 使用 `RwLock` 代替 `Mutex`（读多写少场景）
  2. 将更新结果暂存，仅在写入时短暂持锁
  3. 使用 channel 模式异步通知

### 17.5 磁盘空间管理

每关注一个 Planet 会在 `Following/` 下创建目录：
- `planet.json`：几 KB
- `Articles/`：每篇文章一个 JSON 文件（几 KB）
- `avatar.png`：通常 < 100 KB
- 总计：关注 100 个 Planet（每个 50 篇文章）≈ 50-100 MB

IPFS Pin 的数据会占用 IPFS 仓库空间，定期运行 `repo/gc` 可以清理。

---

## 18. 与后续 Phase 的衔接

Phase 4 完成后，已具备：
- ✅ 关注 IPNS/ENS/.bit/HTTP Feed
- ✅ 拉取和增量更新文章
- ✅ 后台自动更新
- ✅ 已读/未读状态管理

**Phase 5（UI/UX 完善）** 将在此基础上：
- 完善三栏布局中的 Following 区域
- 添加拖拽排序
- 实现 "Today" / "Unread" 聚合视图
- 优化文章内容渲染（代码高亮、图片懒加载）
- 添加搜索功能

**Phase 6（高级功能）** 可能包括：
- WalletConnect / Tipping 集成
- Juicebox 项目支持
- 文章收藏 / 星标
- 导出为 OPML