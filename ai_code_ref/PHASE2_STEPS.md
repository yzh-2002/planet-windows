# Phase 2：数据模型与持久化 — 详细开发文档

> **目标**：实现 Planet、Article、Draft 的数据模型和 JSON 文件持久化，对应原项目 `MyPlanetModel.swift`、`FollowingPlanetModel.swift`、`MyArticleModel.swift`、`DraftModel.swift`、`PlanetStore.swift`
>
> **预计工期**：2 周
>
> **验收标准**：可以创建 Planet，创建文章（含标题、Markdown 内容），文章列表正常展示，关闭应用重新打开后数据不丢失。能删除 Planet 和文章。数据存储在 `%APPDATA%/planet-desktop/Planet/` (Windows) 或 `~/Library/Application Support/planet-desktop/Planet/` (macOS)。

---

## 目录

- [1. 整体架构](#1-整体架构)
- [2. 数据存储结构](#2-数据存储结构)
- [3. Step 1：实现 Planet 数据模型 (`models/planet.rs`)](#3-step-1实现-planet-数据模型-modelsplanetrs)
- [4. Step 2：实现 Article 数据模型 (`models/article.rs`)](#4-step-2实现-article-数据模型-modelsarticlers)
- [5. Step 3：实现 Draft 数据模型 (`models/draft.rs`)](#5-step-3实现-draft-数据模型-modelsdraftrs)
- [6. Step 4：实现 PlanetStore 全局状态 (`store/mod.rs`)](#6-step-4实现-planetstore-全局状态-storemodrs)
- [7. Step 5：注册 Tauri Commands (`commands/planet.rs`)](#7-step-5注册-tauri-commands-commandsplanetrs)
- [8. Step 6：前端实现](#8-step-6前端实现)
- [9. Step 7：测试与调试](#9-step-7测试与调试)
- [10. 文件清单](#10-文件清单)
- [11. Swift → Rust 对照表](#11-swift--rust-对照表)

---

## 1. 整体架构

```
┌─────────────────────────────────────────────────────┐
│                    前端 (React)                     │
│                                                     │
│  PlanetList 组件                                   │
│  ArticleList 组件                                   │
│  ArticleDetail 组件                                 │
│  NewPlanetDialog / NewArticleDialog                │
│                                                     │
│  invoke("planet_create")                            │
│  invoke("planet_list")                              │
│  invoke("article_create")                           │
│  invoke("article_list")                             │
└──────────────────────┬──────────────────────────────┘
                       │ IPC (Tauri invoke)
┌──────────────────────▼──────────────────────────────┐
│                Rust 后端 (Tauri)                    │
│                                                     │
│  commands/planet.rs   ← Tauri Commands             │
│  commands/article.rs   ← Tauri Commands            │
│       │                                             │
│       ▼                                             │
│  store/mod.rs         ← 全局状态 (PlanetStore)      │
│       │                                             │
│       ▼                                             │
│  models/planet.rs     ← MyPlanet / FollowingPlanet │
│  models/article.rs    ← MyArticle / FollowingArticle│
│  models/draft.rs      ← Draft                       │
│       │                                             │
│       ▼                                             │
│  helpers/paths.rs     ← 文件路径工具                │
└──────────────────────┬──────────────────────────────┘
                       │ 文件系统操作
┌──────────────────────▼──────────────────────────────┐
│              文件系统 (JSON 持久化)                  │
│                                                     │
│  ~/.planet/My/{uuid}/planet.json                    │
│  ~/.planet/My/{uuid}/Articles/{uuid}.json          │
│  ~/.planet/My/{uuid}/Drafts/{uuid}/Draft.json      │
│  ~/.planet/Following/{uuid}/planet.json            │
└─────────────────────────────────────────────────────┘
```

---

## 2. 数据存储结构

### 2.1 目录结构

```
~/.planet/  (或 %APPDATA%/planet-desktop/Planet/)
├── My/
│   └── {planet_uuid}/
│       ├── planet.json          # Planet 元数据
│       ├── avatar.png           # 头像（可选）
│       ├── favicon.ico          # 网站图标（可选）
│       ├── Articles/
│       │   ├── {article_uuid}.json
│       │   └── {article_uuid}/
│       │       └── Attachments/  # 附件目录
│       └── Drafts/
│           └── {draft_uuid}/
│               ├── Draft.json
│               └── Attachments/
└── Following/
    └── {planet_uuid}/
        ├── planet.json
        ├── avatar.png
        └── Articles/
            └── {article_uuid}.json
```

### 2.2 JSON 文件格式

**`planet.json` (MyPlanet)**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "My Blog",
  "about": "A personal blog",
  "domain": null,
  "author_name": "John Doe",
  "created": "2024-01-01T00:00:00Z",
  "ipns": "k51qzi5uqu5dibstm2yxidly22jx94embd7j3xjstfk65ulictn2ajnjvpiac7",
  "updated": "2024-01-15T10:30:00Z",
  "template_name": "Writer",
  "last_published": "2024-01-15T10:30:00Z",
  "last_published_cid": "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
  "archived": false,
  "twitter_username": null,
  "github_username": null
}
```

**`{article_uuid}.json` (MyArticle)**:
```json
{
  "id": "660e8400-e29b-41d4-a716-446655440000",
  "title": "Hello World",
  "content": "# Hello World\n\nThis is my first article.",
  "created": "2024-01-10T12:00:00Z",
  "updated": "2024-01-10T12:00:00Z",
  "link": "/660e8400-e29b-41d4-a716-446655440000/",
  "slug": null,
  "attachments": [],
  "tags": {},
  "pinned": null
}
```

---

## 3. Step 1：实现 Planet 数据模型 (`models/planet.rs`)

### 3.1 对应关系

| Swift 结构体 | Rust 结构体 |
|-------------|------------|
| `MyPlanetModel` | `MyPlanet` |
| `FollowingPlanetModel` | `FollowingPlanet` |
| `PlanetType` enum | `PlanetType` enum |
| `PublicPlanetModel` | `PublicPlanet` |

### 3.2 完整代码

创建 `src-tauri/src/models/planet.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::{anyhow, Result};
use tracing::{debug, error, info};

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
    pub fn my_planets_path() -> PathBuf {
        let path = paths::get_data_path().join("My");
        fs::create_dir_all(&path).ok();
        path
    }

    /// 当前 Planet 的基础路径
    pub fn base_path(&self) -> PathBuf {
        Self::my_planets_path().join(self.id.to_string())
    }

    /// planet.json 文件路径
    pub fn info_path(&self) -> PathBuf {
        self.base_path().join("planet.json")
    }

    /// Articles 目录路径
    pub fn articles_path(&self) -> PathBuf {
        self.base_path().join("Articles")
    }

    /// Drafts 目录路径
    pub fn drafts_path(&self) -> PathBuf {
        self.base_path().join("Drafts")
    }

    /// Avatar 图片路径
    pub fn avatar_path(&self) -> PathBuf {
        self.base_path().join("avatar.png")
    }

    /// Favicon 路径
    pub fn favicon_path(&self) -> PathBuf {
        self.base_path().join("favicon.ico")
    }

    // ============================================================
    // CRUD 操作
    // ============================================================

    /// 创建新的 Planet
    pub fn create(name: String, about: String, template_name: String) -> Result<Self> {
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
        fs::create_dir_all(planet.articles_path())?;
        fs::create_dir_all(planet.drafts_path())?;

        // 保存到磁盘
        planet.save()?;

        info!("Created new planet: {} ({})", planet.name, planet.id);
        Ok(planet)
    }

    /// 从磁盘加载 Planet
    pub fn load(planet_id: Uuid) -> Result<Self> {
        let base_path = Self::my_planets_path().join(planet_id.to_string());
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
    pub fn load_all() -> Result<Vec<Self>> {
        let my_planets_path = Self::my_planets_path();
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
                        match Self::load(planet_id) {
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
    pub fn save(&self) -> Result<()> {
        let info_path = self.info_path();
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&info_path, content)?;
        debug!("Saved planet: {}", self.id);
        Ok(())
    }

    /// 更新 Planet
    pub fn update<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated = Utc::now();
        self.save()?;
        Ok(())
    }

    /// 删除 Planet（包括所有文章和草稿）
    pub fn delete(&self) -> Result<()> {
        let base_path = self.base_path();
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
    pub fn following_planets_path() -> PathBuf {
        let path = paths::get_data_path().join("Following");
        fs::create_dir_all(&path).ok();
        path
    }

    /// 当前 Planet 的基础路径
    pub fn base_path(&self) -> PathBuf {
        Self::following_planets_path().join(self.id.to_string())
    }

    /// planet.json 文件路径
    pub fn info_path(&self) -> PathBuf {
        self.base_path().join("planet.json")
    }

    /// Articles 目录路径
    pub fn articles_path(&self) -> PathBuf {
        self.base_path().join("Articles")
    }

    /// Avatar 图片路径
    pub fn avatar_path(&self) -> PathBuf {
        self.base_path().join("avatar.png")
    }

    /// 创建新的 Following Planet
    pub fn create(
        name: String,
        about: String,
        planet_type: PlanetType,
        link: String,
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
        fs::create_dir_all(planet.articles_path())?;

        // 保存到磁盘
        planet.save()?;

        info!("Created new following planet: {} ({})", planet.name, planet.id);
        Ok(planet)
    }

    /// 从磁盘加载 Following Planet
    pub fn load(planet_id: Uuid) -> Result<Self> {
        let base_path = Self::following_planets_path().join(planet_id.to_string());
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
    pub fn load_all() -> Result<Vec<Self>> {
        let following_planets_path = Self::following_planets_path();
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
                        match Self::load(planet_id) {
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
    pub fn save(&self) -> Result<()> {
        let info_path = self.info_path();
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&info_path, content)?;
        debug!("Saved following planet: {}", self.id);
        Ok(())
    }

    /// 更新 Following Planet
    pub fn update<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated = Utc::now();
        self.save()?;
        Ok(())
    }

    /// 删除 Following Planet
    pub fn delete(&self) -> Result<()> {
        let base_path = self.base_path();
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
```

### 3.3 更新 `models/mod.rs`

```rust
pub mod planet;
pub mod article;  // Step 2 实现
pub mod draft;    // Step 3 实现
```

### 3.4 新增依赖

在 `Cargo.toml` 中添加：

```toml
uuid = { version = "1", features = ["serde", "v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

### 3.5 验证

```bash
cargo check
```

---

## 4. Step 2：实现 Article 数据模型 (`models/article.rs`)

### 4.1 对应关系

| Swift 结构体 | Rust 结构体 |
|-------------|------------|
| `MyArticleModel` | `MyArticle` |
| `FollowingArticleModel` | `FollowingArticle` |
| `ArticleType` enum | `ArticleType` enum |
| `PublicArticleModel` | `PublicArticle` |

### 4.2 完整代码

创建 `src-tauri/src/models/article.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::{anyhow, Result};
use tracing::{debug, error, info};

use crate::models::planet::MyPlanet;

// ============================================================
// ArticleType 枚举
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArticleType {
    Blog = 0,
    Page = 1,
}

// ============================================================
// Attachment 结构体
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

// ============================================================
// MyArticle 结构体
// 对应原项目 MyArticleModel.swift
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyArticle {
    pub id: Uuid,
    pub planet_id: Uuid,  // 关联的 Planet ID
    pub title: String,
    pub content: String,  // Markdown 内容
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub link: String,  // 相对路径，如 "/{id}/"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,  // 自定义 slug，如 "hello-world"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hero_image: Option<String>,  // 文件名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_link: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub article_type: Option<ArticleType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_included_in_navigation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub navigation_weight: Option<i32>,
}

impl MyArticle {
    /// 获取 Article 文件路径
    pub fn path(&self, planet: &MyPlanet) -> PathBuf {
        planet.articles_path().join(format!("{}.json", self.id))
    }

    /// 获取 Article 附件目录路径
    pub fn attachments_path(&self, planet: &MyPlanet) -> PathBuf {
        planet.articles_path().join(self.id.to_string()).join("Attachments")
    }

    /// 创建新文章
    pub fn create(
        planet_id: Uuid,
        title: String,
        content: String,
    ) -> Result<Self> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let link = format!("/{}/", id);

        let article = Self {
            id,
            planet_id,
            title,
            content,
            created: now,
            updated: now,
            link,
            slug: None,
            hero_image: None,
            external_link: None,
            attachments: Vec::new(),
            tags: HashMap::new(),
            pinned: None,
            article_type: Some(ArticleType::Blog),
            summary: None,
            is_included_in_navigation: Some(false),
            navigation_weight: Some(1),
        };

        info!("Created new article: {} ({})", article.title, article.id);
        Ok(article)
    }

    /// 从磁盘加载文章
    pub fn load(planet: &MyPlanet, article_id: Uuid) -> Result<Self> {
        let article_path = planet.articles_path().join(format!("{}.json", article_id));

        if !article_path.exists() {
            return Err(anyhow!("Article not found: {}", article_id));
        }

        let content = fs::read_to_string(&article_path)?;
        let mut article: Self = serde_json::from_str(&content)?;

        // 验证关联关系
        if article.planet_id != planet.id {
            return Err(anyhow!("Article planet_id mismatch"));
        }
        if article.id != article_id {
            return Err(anyhow!("Article ID mismatch"));
        }

        Ok(article)
    }

    /// 从目录加载 Planet 的所有文章
    pub fn load_all(planet: &MyPlanet) -> Result<Vec<Self>> {
        let articles_path = planet.articles_path();
        let mut articles = Vec::new();

        if !articles_path.exists() {
            return Ok(articles);
        }

        for entry in fs::read_dir(&articles_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_name) = path.file_stem().and_then(|n| n.to_str()) {
                    if let Ok(article_id) = Uuid::parse_str(file_name) {
                        match Self::load(planet, article_id) {
                            Ok(article) => articles.push(article),
                            Err(e) => {
                                error!("Failed to load article {}: {}", article_id, e);
                            }
                        }
                    }
                }
            }
        }

        // 按创建时间倒序排序
        articles.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(articles)
    }

    /// 保存文章到磁盘
    pub fn save(&self, planet: &MyPlanet) -> Result<()> {
        let article_path = self.path(planet);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&article_path, content)?;
        debug!("Saved article: {} ({})", self.title, self.id);
        Ok(())
    }

    /// 更新文章
    pub fn update<F>(&mut self, planet: &MyPlanet, f: F) -> Result<()>
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated = Utc::now();
        self.save(planet)?;
        Ok(())
    }

    /// 删除文章
    pub fn delete(&self, planet: &MyPlanet) -> Result<()> {
        let article_path = self.path(planet);
        if article_path.exists() {
            fs::remove_file(&article_path)?;
        }

        // 删除附件目录
        let attachments_path = self.attachments_path(planet);
        if attachments_path.exists() {
            fs::remove_dir_all(&attachments_path)?;
        }

        info!("Deleted article: {} ({})", self.title, self.id);
        Ok(())
    }

    /// 添加附件
    pub fn add_attachment(&mut self, planet: &MyPlanet, attachment: Attachment) -> Result<()> {
        // 创建附件目录
        let attachments_path = self.attachments_path(planet);
        fs::create_dir_all(&attachments_path)?;

        self.attachments.push(attachment);
        self.save(planet)?;
        Ok(())
    }

    /// 删除附件
    pub fn remove_attachment(&mut self, planet: &MyPlanet, name: &str) -> Result<()> {
        self.attachments.retain(|a| a.name != name);
        self.save(planet)?;
        Ok(())
    }
}

// ============================================================
// FollowingArticle 结构体
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowingArticle {
    pub id: Uuid,
    pub planet_id: Uuid,
    pub title: String,
    pub content: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub link: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<DateTime<Utc>>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

impl FollowingArticle {
    pub fn path(&self, planet: &crate::models::planet::FollowingPlanet) -> PathBuf {
        planet.articles_path().join(format!("{}.json", self.id))
    }

    pub fn load(
        planet: &crate::models::planet::FollowingPlanet,
        article_id: Uuid,
    ) -> Result<Self> {
        let article_path = planet.articles_path().join(format!("{}.json", article_id));

        if !article_path.exists() {
            return Err(anyhow!("Following article not found: {}", article_id));
        }

        let content = fs::read_to_string(&article_path)?;
        let article: Self = serde_json::from_str(&content)?;

        Ok(article)
    }

    pub fn load_all(planet: &crate::models::planet::FollowingPlanet) -> Result<Vec<Self>> {
        let articles_path = planet.articles_path();
        let mut articles = Vec::new();

        if !articles_path.exists() {
            return Ok(articles);
        }

        for entry in fs::read_dir(&articles_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_name) = path.file_stem().and_then(|n| n.to_str()) {
                    if let Ok(article_id) = Uuid::parse_str(file_name) {
                        match Self::load(planet, article_id) {
                            Ok(article) => articles.push(article),
                            Err(e) => {
                                error!("Failed to load following article {}: {}", article_id, e);
                            }
                        }
                    }
                }
            }
        }

        articles.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(articles)
    }

    pub fn save(&self, planet: &crate::models::planet::FollowingPlanet) -> Result<()> {
        let article_path = self.path(planet);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&article_path, content)?;
        Ok(())
    }
}

// ============================================================
// PublicArticle 结构体（用于模板渲染）
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicArticle {
    pub id: Uuid,
    pub link: String,
    pub slug: String,
    pub title: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_rendered: Option<String>,
    pub created: DateTime<Utc>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<DateTime<Utc>>,
}

impl From<&MyArticle> for PublicArticle {
    fn from(article: &MyArticle) -> Self {
        Self {
            id: article.id,
            link: article.link.clone(),
            slug: article.slug.clone().unwrap_or_default(),
            title: article.title.clone(),
            content: article.content.clone(),
            content_rendered: None,  // Phase 3 渲染时填充
            created: article.created,
            attachments: article.attachments.clone(),
            tags: article.tags.clone(),
            pinned: article.pinned,
        }
    }
}
```

### 4.3 验证

```bash
cargo check
```

---

## 5. Step 3：实现 Draft 数据模型 (`models/draft.rs`)

### 5.1 对应关系

| Swift 结构体 | Rust 结构体 |
|-------------|------------|
| `DraftModel` | `Draft` |

### 5.2 完整代码

创建 `src-tauri/src/models/draft.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::{anyhow, Result};
use tracing::{debug, info};

use crate::models::planet::MyPlanet;
use crate::models::article::{Attachment, MyArticle};

// ============================================================
// Draft 结构体
// 对应原项目 DraftModel.swift
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    pub id: Uuid,
    pub planet_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub article_id: Option<Uuid>,  // 如果为 None，表示新文章草稿；否则表示编辑现有文章
    pub date: DateTime<Utc>,
    pub title: String,
    pub content: String,  // Markdown 内容
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hero_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_link: Option<String>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

impl Draft {
    /// 获取 Draft 目录路径
    pub fn base_path(&self, planet: &MyPlanet) -> PathBuf {
        if let Some(article_id) = self.article_id {
            // 编辑现有文章的草稿
            planet.articles_path()
                .join("Drafts")
                .join(article_id.to_string())
        } else {
            // 新文章草稿
            planet.drafts_path().join(self.id.to_string())
        }
    }

    /// Draft.json 文件路径
    pub fn info_path(&self, planet: &MyPlanet) -> PathBuf {
        self.base_path(planet).join("Draft.json")
    }

    /// 附件目录路径
    pub fn attachments_path(&self, planet: &MyPlanet) -> PathBuf {
        self.base_path(planet).join("Attachments")
    }

    /// 创建新草稿（用于新文章）
    pub fn create_new(planet_id: Uuid, title: String, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            planet_id,
            article_id: None,
            date: Utc::now(),
            title,
            content,
            attachments: Vec::new(),
            hero_image: None,
            external_link: None,
            tags: HashMap::new(),
        }
    }

    /// 创建编辑草稿（用于编辑现有文章）
    pub fn create_edit(planet_id: Uuid, article_id: Uuid, article: &MyArticle) -> Self {
        Self {
            id: Uuid::new_v4(),
            planet_id,
            article_id: Some(article_id),
            date: Utc::now(),
            title: article.title.clone(),
            content: article.content.clone(),
            attachments: article.attachments.clone(),
            hero_image: article.hero_image.clone(),
            external_link: article.external_link.clone(),
            tags: article.tags.clone(),
        }
    }

    /// 从磁盘加载草稿
    pub fn load(planet: &MyPlanet, draft_id: Uuid) -> Result<Self> {
        // 先尝试在新文章草稿目录
        let new_draft_path = planet.drafts_path().join(draft_id.to_string()).join("Draft.json");
        if new_draft_path.exists() {
            let content = fs::read_to_string(&new_draft_path)?;
            let draft: Self = serde_json::from_str(&content)?;
            if draft.id == draft_id {
                return Ok(draft);
            }
        }

        // 再尝试在文章草稿目录中查找
        let articles_drafts_path = planet.articles_path().join("Drafts");
        if articles_drafts_path.exists() {
            for entry in fs::read_dir(&articles_drafts_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let draft_path = path.join("Draft.json");
                    if draft_path.exists() {
                        let content = fs::read_to_string(&draft_path)?;
                        let draft: Self = serde_json::from_str(&content)?;
                        if draft.id == draft_id {
                            return Ok(draft);
                        }
                    }
                }
            }
        }

        Err(anyhow!("Draft not found: {}", draft_id))
    }

    /// 加载 Planet 的所有草稿（新文章草稿 + 文章编辑草稿）
    pub fn load_all(planet: &MyPlanet) -> Result<Vec<Self>> {
        let mut drafts = Vec::new();

        // 加载新文章草稿
        let drafts_path = planet.drafts_path();
        if drafts_path.exists() {
            for entry in fs::read_dir(&drafts_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let draft_path = path.join("Draft.json");
                    if draft_path.exists() {
                        match fs::read_to_string(&draft_path)
                            .ok()
                            .and_then(|c| serde_json::from_str::<Self>(&c).ok())
                        {
                            Some(draft) => drafts.push(draft),
                            None => {
                                debug!("Skipped invalid draft in {:?}", path);
                            }
                        }
                    }
                }
            }
        }

        // 加载文章编辑草稿
        let articles_drafts_path = planet.articles_path().join("Drafts");
        if articles_drafts_path.exists() {
            for entry in fs::read_dir(&articles_drafts_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let draft_path = path.join("Draft.json");
                    if draft_path.exists() {
                        match fs::read_to_string(&draft_path)
                            .ok()
                            .and_then(|c| serde_json::from_str::<Self>(&c).ok())
                        {
                            Some(draft) => drafts.push(draft),
                            None => {
                                debug!("Skipped invalid article draft in {:?}", path);
                            }
                        }
                    }
                }
            }
        }

        drafts.sort_by(|a, b| b.date.cmp(&a.date));
        Ok(drafts)
    }

    /// 保存草稿到磁盘
    pub fn save(&self, planet: &MyPlanet) -> Result<()> {
        let base_path = self.base_path(planet);
        fs::create_dir_all(&base_path)?;
        fs::create_dir_all(self.attachments_path(planet))?;

        let info_path = self.info_path(planet);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&info_path, content)?;
        debug!("Saved draft: {}", self.id);
        Ok(())
    }

    /// 删除草稿
    pub fn delete(&self, planet: &MyPlanet) -> Result<()> {
        let base_path = self.base_path(planet);
        if base_path.exists() {
            fs::remove_dir_all(&base_path)?;
            info!("Deleted draft: {}", self.id);
        }
        Ok(())
    }

    /// 将草稿发布为文章
    /// 如果 article_id 为 None，创建新文章；否则更新现有文章
    pub fn publish_to_article(&self, planet: &mut MyPlanet) -> Result<MyArticle> {
        let article = if let Some(article_id) = self.article_id {
            // 更新现有文章
            let mut article = MyArticle::load(planet, article_id)?;
            article.update(planet, |a| {
                a.title = self.title.clone();
                a.content = self.content.clone();
                a.hero_image = self.hero_image.clone();
                a.external_link = self.external_link.clone();
                a.tags = self.tags.clone();
                // 附件在 Phase 3+ 实现完整拷贝逻辑
            })?;
            info!("Updated article from draft: {} -> {}", self.id, article.id);
            article
        } else {
            // 创建新文章
            let mut article = MyArticle::create(
                self.planet_id,
                self.title.clone(),
                self.content.clone(),
            )?;
            article.hero_image = self.hero_image.clone();
            article.external_link = self.external_link.clone();
            article.tags = self.tags.clone();
            article.save(planet)?;
            info!("Created article from draft: {} -> {}", self.id, article.id);
            article
        };

        // 删除草稿
        self.delete(planet)?;

        // 更新 Planet 时间戳
        planet.updated = Utc::now();
        planet.save()?;

        Ok(article)
    }
}
```

### 5.3 验证

```bash
cargo check
```

---

## 6. Step 4：实现 PlanetStore 全局状态 (`store/mod.rs`)

### 6.1 对应关系

| Swift 结构体 | Rust 结构体 |
|-------------|------------|
| `PlanetStore` (ObservableObject) | `PlanetStore` (Arc<Mutex<>>) |
| `PlanetDetailViewType` | `SelectedView` |
| `@Published var myPlanets` | `my_planets: Vec<MyPlanet>` |
| `@Published var followingPlanets` | `following_planets: Vec<FollowingPlanet>` |

### 6.2 完整代码

创建 `src-tauri/src/store/mod.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use anyhow::{anyhow, Result};
use tracing::{debug, error, info};

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
    pub fn load(&mut self) -> Result<()> {
        info!("Loading planets from disk...");

        // 加载 My Planets
        self.my_planets = MyPlanet::load_all()?;
        info!("Loaded {} my planets", self.my_planets.len());

        // 加载 Following Planets
        self.following_planets = FollowingPlanet::load_all()?;
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
    ) -> Result<MyPlanet> {
        let planet = MyPlanet::create(name, about, template_name)?;
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
    pub fn update_planet<F>(&mut self, planet_id: Uuid, f: F) -> Result<()>
    where
        F: FnOnce(&mut MyPlanet),
    {
        let planet = self.my_planets.iter_mut().find(|p| p.id == planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        planet.update(f)?;
        Ok(())
    }

    /// 删除 Planet
    pub fn delete_planet(&mut self, planet_id: Uuid) -> Result<()> {
        if let Some(idx) = self.my_planets.iter().position(|p| p.id == planet_id) {
            let planet = &self.my_planets[idx];
            planet.delete()?;
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
    pub fn list_articles(&self, planet_id: Uuid) -> Result<Vec<MyArticle>> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        MyArticle::load_all(planet)
    }

    /// 创建新文章
    pub fn create_article(
        &mut self,
        planet_id: Uuid,
        title: String,
        content: String,
    ) -> Result<MyArticle> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        let article = MyArticle::create(planet_id, title, content)?;
        article.save(planet)?;

        // 更新 Planet 时间戳
        if let Some(planet) = self.get_planet_mut(planet_id) {
            planet.updated = chrono::Utc::now();
            planet.save()?;
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
    ) -> Result<MyArticle> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        let mut article = MyArticle::load(planet, article_id)?;

        article.update(planet, |a| {
            if let Some(t) = title {
                a.title = t;
            }
            if let Some(c) = content {
                a.content = c;
            }
        })?;

        Ok(article)
    }

    /// 删除文章
    pub fn delete_article(&self, planet_id: Uuid, article_id: Uuid) -> Result<()> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        let article = MyArticle::load(planet, article_id)?;
        article.delete(planet)
    }

    // ============================================================
    // Draft CRUD
    // ============================================================

    /// 获取 Planet 的所有草稿
    pub fn list_drafts(&self, planet_id: Uuid) -> Result<Vec<Draft>> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        Draft::load_all(planet)
    }

    /// 创建新草稿
    pub fn create_draft(
        &self,
        planet_id: Uuid,
        title: String,
        content: String,
    ) -> Result<Draft> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        let draft = Draft::create_new(planet_id, title, content);
        draft.save(planet)?;
        Ok(draft)
    }

    /// 保存草稿
    pub fn save_draft(&self, planet_id: Uuid, draft: &Draft) -> Result<()> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        draft.save(planet)
    }

    /// 删除草稿
    pub fn delete_draft(&self, planet_id: Uuid, draft_id: Uuid) -> Result<()> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?;
        let draft = Draft::load(planet, draft_id)?;
        draft.delete(planet)
    }

    /// 发布草稿为文章
    pub fn publish_draft(
        &mut self,
        planet_id: Uuid,
        draft_id: Uuid,
    ) -> Result<MyArticle> {
        let planet = self.get_planet(planet_id)
            .ok_or_else(|| anyhow!("Planet not found: {}", planet_id))?
            .clone();  // 需要 clone 因为 publish_to_article 需要 &mut MyPlanet

        let draft = Draft::load(&planet, draft_id)?;
        let mut planet_mut = planet;
        let article = draft.publish_to_article(&mut planet_mut)?;

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
    ) -> Result<FollowingPlanet> {
        let planet = FollowingPlanet::create(name, about, planet_type, link)?;
        self.following_planets.insert(0, planet.clone());
        Ok(planet)
    }

    /// 取消关注 Planet
    pub fn unfollow_planet(&mut self, planet_id: Uuid) -> Result<()> {
        if let Some(idx) = self.following_planets.iter().position(|p| p.id == planet_id) {
            let planet = &self.following_planets[idx];
            planet.delete()?;
            self.following_planets.remove(idx);
            Ok(())
        } else {
            Err(anyhow!("Following planet not found: {}", planet_id))
        }
    }

    /// 获取 Following Planet 的所有文章
    pub fn list_following_articles(&self, planet_id: Uuid) -> Result<Vec<FollowingArticle>> {
        let planet = self.following_planets.iter().find(|p| p.id == planet_id)
            .ok_or_else(|| anyhow!("Following planet not found: {}", planet_id))?;
        FollowingArticle::load_all(planet)
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
    pub fn emit_state_changed(&self, app: &tauri::AppHandle) {
        let snapshot = self.snapshot();
        if let Err(e) = app.emit_all("planet:state-changed", &snapshot) {
            error!("Failed to emit planet state: {}", e);
        }
    }
}
```

### 6.3 验证

```bash
cargo check
```

---

## 7. Step 5：注册 Tauri Commands

### 7.1 commands/planet.rs

创建 `src-tauri/src/commands/planet.rs`：

```rust
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
        .create_planet(request.name, request.about, request.template_name)
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
    }).map_err(|e| e.to_string())?;

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
    store.delete_planet(uuid).map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);
    Ok(())
}
```

### 7.2 commands/article.rs

创建 `src-tauri/src/commands/article.rs`：

```rust
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
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
) -> Result<Vec<MyArticle>, String> {
    let uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store.list_articles(uuid).map_err(|e| e.to_string())
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
        .create_article(planet_id, request.title, request.content)
        .map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);
    Ok(article)
}

/// 获取单篇文章
#[tauri::command]
pub fn article_get(
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
    article_id: String,
) -> Result<MyArticle, String> {
    let planet_uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let article_uuid = Uuid::parse_str(&article_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    let planet = store.get_planet(planet_uuid)
        .ok_or_else(|| format!("Planet not found: {}", planet_id))?;
    MyArticle::load(planet, article_uuid).map_err(|e| e.to_string())
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
        .update_article(planet_uuid, article_uuid, request.title, request.content)
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
    store.delete_article(planet_uuid, article_uuid).map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);
    Ok(())
}

// ============================================================
// Draft Commands
// ============================================================

/// 列出 Planet 的所有草稿
#[tauri::command]
pub fn draft_list(
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
) -> Result<Vec<Draft>, String> {
    let uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store.list_drafts(uuid).map_err(|e| e.to_string())
}

/// 创建草稿
#[tauri::command]
pub fn draft_create(
    store: State<'_, PlanetStoreHandle>,
    request: CreateDraftRequest,
) -> Result<Draft, String> {
    let planet_id = Uuid::parse_str(&request.planet_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store
        .create_draft(planet_id, request.title, request.content)
        .map_err(|e| e.to_string())
}

/// 保存草稿（更新标题和内容）
#[tauri::command]
pub fn draft_save(
    store: State<'_, PlanetStoreHandle>,
    request: SaveDraftRequest,
) -> Result<(), String> {
    let planet_id = Uuid::parse_str(&request.planet_id).map_err(|e| e.to_string())?;
    let draft_id = Uuid::parse_str(&request.draft_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    let planet = store.get_planet(planet_id)
        .ok_or_else(|| format!("Planet not found: {}", request.planet_id))?;

    let mut draft = Draft::load(planet, draft_id).map_err(|e| e.to_string())?;
    draft.title = request.title;
    draft.content = request.content;
    draft.date = chrono::Utc::now();
    draft.save(planet).map_err(|e| e.to_string())
}

/// 删除草稿
#[tauri::command]
pub fn draft_delete(
    store: State<'_, PlanetStoreHandle>,
    planet_id: String,
    draft_id: String,
) -> Result<(), String> {
    let planet_uuid = Uuid::parse_str(&planet_id).map_err(|e| e.to_string())?;
    let draft_uuid = Uuid::parse_str(&draft_id).map_err(|e| e.to_string())?;
    let store = store.lock().map_err(|e| e.to_string())?;
    store.delete_draft(planet_uuid, draft_uuid).map_err(|e| e.to_string())
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
        .publish_draft(planet_uuid, draft_uuid)
        .map_err(|e| e.to_string())?;
    store.emit_state_changed(&app);
    Ok(article)
}
```

### 7.3 更新 `commands/mod.rs`

```rust
pub mod ipfs;      // Phase 1 已实现
pub mod planet;    // Phase 2 新增
pub mod article;   // Phase 2 新增
pub mod app;       // Phase 0 已实现
```

### 7.4 更新 `main.rs`

在 Phase 1 的基础上添加 PlanetStore 的集成：

```rust
// src-tauri/src/main.rs

mod commands;
mod helpers;
mod ipfs;
mod models;
mod store;
mod template;
mod keystore;

use std::sync::{Arc, Mutex};
use store::{PlanetStore, PlanetStoreHandle};

fn main() {
    // 初始化日志（Phase 1 已设置）
    tracing_subscriber::fmt::init();

    // 创建 IPFS 状态（Phase 1）
    let ipfs_state = ipfs::state::IpfsState::new();
    let ipfs_state_handle: ipfs::state::IpfsStateHandle = Arc::new(Mutex::new(ipfs_state));

    // 创建 PlanetStore（Phase 2 新增）
    let mut planet_store = PlanetStore::new();
    if let Err(e) = planet_store.load() {
        tracing::error!("Failed to load planets: {}", e);
    }
    let planet_store_handle: PlanetStoreHandle = Arc::new(Mutex::new(planet_store));

    tauri::Builder::default()
        .manage(ipfs_state_handle.clone())
        .manage(planet_store_handle.clone())  // ← Phase 2 新增
        .invoke_handler(tauri::generate_handler![
            // Phase 0
            commands::app::greet,
            // Phase 1: IPFS Commands
            commands::ipfs::ipfs_get_state,
            commands::ipfs::ipfs_setup,
            commands::ipfs::ipfs_launch,
            commands::ipfs::ipfs_shutdown,
            commands::ipfs::ipfs_gc,
            commands::ipfs::ipfs_refresh_status,
            // Phase 2: Planet Commands ← 新增
            commands::planet::planet_get_state,
            commands::planet::planet_list,
            commands::planet::planet_create,
            commands::planet::planet_get,
            commands::planet::planet_update,
            commands::planet::planet_delete,
            // Phase 2: Article Commands ← 新增
            commands::article::article_list,
            commands::article::article_create,
            commands::article::article_get,
            commands::article::article_update,
            commands::article::article_delete,
            // Phase 2: Draft Commands ← 新增
            commands::article::draft_list,
            commands::article::draft_create,
            commands::article::draft_save,
            commands::article::draft_delete,
            commands::article::draft_publish,
        ])
        .setup(move |app| {
            // Phase 1: 自动启动 IPFS
            let app_handle = app.handle();
            let ipfs_handle = ipfs_state_handle.clone();
            tauri::async_runtime::spawn(async move {
                ipfs::state::auto_start(app_handle, ipfs_handle).await;
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let handle = app_handle.clone();
                tauri::async_runtime::block_on(async {
                    ipfs::state::graceful_shutdown(handle).await;
                });
            }
        });
}
```

### 7.5 验证

```bash
cargo check
```

---

## 8. Step 6：前端实现

### 8.1 TypeScript 类型定义

创建 `src/types/planet.ts`：

```typescript
// ============================================================
// Planet 相关类型定义
// ============================================================

export interface MyPlanet {
  id: string
  name: string
  about: string
  domain?: string
  author_name?: string
  created: string
  ipns: string
  updated: string
  template_name: string
  last_published?: string
  last_published_cid?: string
  archived?: boolean
  twitter_username?: string
  github_username?: string
  telegram_username?: string
  mastodon_username?: string
  discord_link?: string
}

export interface FollowingPlanet {
  id: string
  name: string
  about: string
  created: string
  planet_type: string
  link: string
  cid?: string
  updated: string
  last_retrieved: string
  archived?: boolean
}

export interface MyArticle {
  id: string
  planet_id: string
  title: string
  content: string
  created: string
  updated: string
  link: string
  slug?: string
  hero_image?: string
  external_link?: string
  attachments: Attachment[]
  tags: Record<string, string>
  pinned?: string
  article_type?: string
  summary?: string
}

export interface Attachment {
  name: string
  url?: string
  mime_type?: string
  size?: number
}

export interface Draft {
  id: string
  planet_id: string
  article_id?: string
  date: string
  title: string
  content: string
  attachments: Attachment[]
  hero_image?: string
  external_link?: string
  tags: Record<string, string>
}

export interface PlanetStoreSnapshot {
  my_planets: MyPlanet[]
  following_planets: FollowingPlanet[]
  selected_view?: SelectedView
}

export type SelectedView =
  | { type: 'Today' }
  | { type: 'Unread' }
  | { type: 'Starred' }
  | { type: 'MyPlanet'; value: string }
  | { type: 'FollowingPlanet'; value: string }
```

### 8.2 React Hook：usePlanetStore

创建 `src/hooks/usePlanetStore.ts`：

```typescript
import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import type {
  MyPlanet,
  MyArticle,
  Draft,
  PlanetStoreSnapshot,
} from '../types/planet'

export function usePlanetStore() {
  const [myPlanets, setMyPlanets] = useState<MyPlanet[]>([])
  const [loading, setLoading] = useState(true)

  // 初始加载
  useEffect(() => {
    invoke<PlanetStoreSnapshot>('planet_get_state')
      .then((state) => {
        setMyPlanets(state.my_planets)
      })
      .catch(console.error)
      .finally(() => setLoading(false))
  }, [])

  // 监听状态变化事件
  useEffect(() => {
    const unlisten = listen<PlanetStoreSnapshot>(
      'planet:state-changed',
      (event) => {
        setMyPlanets(event.payload.my_planets)
      }
    )
    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  // 创建 Planet
  const createPlanet = useCallback(
    async (name: string, about: string, templateName?: string) => {
      return invoke<MyPlanet>('planet_create', {
        request: {
          name,
          about,
          template_name: templateName || 'Plain',
        },
      })
    },
    []
  )

  // 删除 Planet
  const deletePlanet = useCallback(async (planetId: string) => {
    return invoke<void>('planet_delete', { planetId })
  }, [])

  // 更新 Planet
  const updatePlanet = useCallback(
    async (planetId: string, updates: Partial<MyPlanet>) => {
      return invoke<MyPlanet>('planet_update', {
        planetId,
        request: updates,
      })
    },
    []
  )

  return {
    myPlanets,
    loading,
    createPlanet,
    deletePlanet,
    updatePlanet,
  }
}

export function useArticles(planetId: string | null) {
  const [articles, setArticles] = useState<MyArticle[]>([])
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!planetId) {
      setArticles([])
      return
    }
    setLoading(true)
    invoke<MyArticle[]>('article_list', { planetId })
      .then(setArticles)
      .catch(console.error)
      .finally(() => setLoading(false))
  }, [planetId])

  const createArticle = useCallback(
    async (title: string, content: string) => {
      if (!planetId) throw new Error('No planet selected')
      const article = await invoke<MyArticle>('article_create', {
        request: { planet_id: planetId, title, content },
      })
      setArticles((prev) => [article, ...prev])
      return article
    },
    [planetId]
  )

  const deleteArticle = useCallback(
    async (articleId: string) => {
      if (!planetId) throw new Error('No planet selected')
      await invoke<void>('article_delete', { planetId, articleId })
      setArticles((prev) => prev.filter((a) => a.id !== articleId))
    },
    [planetId]
  )

  const updateArticle = useCallback(
    async (articleId: string, title?: string, content?: string) => {
      if (!planetId) throw new Error('No planet selected')
      const updated = await invoke<MyArticle>('article_update', {
        planetId,
        articleId,
        request: { title, content },
      })
      setArticles((prev) =>
        prev.map((a) => (a.id === articleId ? updated : a))
      )
      return updated
    },
    [planetId]
  )

  return { articles, loading, createArticle, deleteArticle, updateArticle }
}
```

### 8.3 Sidebar 组件

创建 `src/components/Sidebar.tsx`：

```tsx
import React, { useState } from 'react'
import type { MyPlanet } from '../types/planet'

interface SidebarProps {
  planets: MyPlanet[]
  selectedPlanetId: string | null
  onSelectPlanet: (id: string) => void
  onCreatePlanet: () => void
}

export function Sidebar({
  planets,
  selectedPlanetId,
  onSelectPlanet,
  onCreatePlanet,
}: SidebarProps) {
  return (
    <div className="w-60 bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700 flex flex-col h-full">
      {/* 头部 */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-gray-600 dark:text-gray-400 uppercase tracking-wider">
          My Planets
        </h2>
        <button
          onClick={onCreatePlanet}
          className="w-6 h-6 flex items-center justify-center rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500"
          title="New Planet"
        >
          +
        </button>
      </div>

      {/* Planet 列表 */}
      <div className="flex-1 overflow-y-auto">
        {planets.length === 0 ? (
          <div className="p-4 text-sm text-gray-400 text-center">
            暂无 Planet，点击 + 创建
          </div>
        ) : (
          planets.map((planet) => (
            <div
              key={planet.id}
              onClick={() => onSelectPlanet(planet.id)}
              className={`px-4 py-3 cursor-pointer border-b border-gray-100 dark:border-gray-800 transition-colors ${
                selectedPlanetId === planet.id
                  ? 'bg-blue-50 dark:bg-blue-900/30 border-l-2 border-l-blue-500'
                  : 'hover:bg-gray-100 dark:hover:bg-gray-800'
              }`}
            >
              <div className="flex items-center gap-3">
                {/* 头像占位 */}
                <div className="w-8 h-8 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center text-white text-xs font-bold">
                  {planet.name.charAt(0).toUpperCase()}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                    {planet.name}
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 truncate">
                    {planet.about || 'No description'}
                  </div>
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
```

### 8.4 ArticleList 组件

创建 `src/components/ArticleList.tsx`：

```tsx
import React from 'react'
import type { MyArticle } from '../types/planet'

interface ArticleListProps {
  articles: MyArticle[]
  selectedArticleId: string | null
  onSelectArticle: (id: string) => void
  onCreateArticle: () => void
  loading: boolean
}

export function ArticleList({
  articles,
  selectedArticleId,
  onSelectArticle,
  onCreateArticle,
  loading,
}: ArticleListProps) {
  if (loading) {
    return (
      <div className="w-72 border-r border-gray-200 dark:border-gray-700 flex items-center justify-center">
        <span className="text-gray-400">加载中...</span>
      </div>
    )
  }

  return (
    <div className="w-72 border-r border-gray-200 dark:border-gray-700 flex flex-col h-full bg-white dark:bg-gray-950">
      {/* 头部 */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-gray-600 dark:text-gray-400">
          Articles ({articles.length})
        </h2>
        <button
          onClick={onCreateArticle}
          className="px-3 py-1 text-xs bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
        >
          New
        </button>
      </div>

      {/* 文章列表 */}
      <div className="flex-1 overflow-y-auto">
        {articles.length === 0 ? (
          <div className="p-4 text-sm text-gray-400 text-center">
            暂无文章
          </div>
        ) : (
          articles.map((article) => (
            <div
              key={article.id}
              onClick={() => onSelectArticle(article.id)}
              className={`px-4 py-3 cursor-pointer border-b border-gray-100 dark:border-gray-800 transition-colors ${
                selectedArticleId === article.id
                  ? 'bg-blue-50 dark:bg-blue-900/20'
                  : 'hover:bg-gray-50 dark:hover:bg-gray-900'
              }`}
            >
              <div className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                {article.title || 'Untitled'}
              </div>
              <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                {new Date(article.created).toLocaleDateString()}
              </div>
              {article.summary && (
                <div className="text-xs text-gray-400 mt-1 line-clamp-2">
                  {article.summary}
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  )
}
```

### 8.5 ArticleDetail 组件

创建 `src/components/ArticleDetail.tsx`：

```tsx
import React from 'react'
import type { MyArticle } from '../types/planet'

interface ArticleDetailProps {
  article: MyArticle | null
  onDelete?: (articleId: string) => void
}

export function ArticleDetail({ article, onDelete }: ArticleDetailProps) {
  if (!article) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-400">
        选择一篇文章查看
      </div>
    )
  }

  return (
    <div className="flex-1 overflow-y-auto bg-white dark:bg-gray-950">
      <div className="max-w-3xl mx-auto p-8">
        {/* 标题 */}
        <h1 className="text-3xl font-bold text-gray-900 dark:text-gray-100 mb-4">
          {article.title}
        </h1>

        {/* 元信息 */}
        <div className="flex items-center gap-4 text-sm text-gray-500 dark:text-gray-400 mb-8 pb-4 border-b border-gray-200 dark:border-gray-700">
          <span>创建于 {new Date(article.created).toLocaleString()}</span>
          <span>更新于 {new Date(article.updated).toLocaleString()}</span>
          {Object.keys(article.tags).length > 0 && (
            <div className="flex gap-1">
              {Object.keys(article.tags).map((tag) => (
                <span
                  key={tag}
                  className="px-2 py-0.5 bg-gray-100 dark:bg-gray-800 rounded text-xs"
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Markdown 内容（Phase 2 先直接展示原文，Phase 3 再渲染 HTML） */}
        <div className="prose dark:prose-invert max-w-none">
          <pre className="whitespace-pre-wrap text-sm text-gray-800 dark:text-gray-200 font-mono bg-gray-50 dark:bg-gray-900 p-4 rounded-lg">
            {article.content}
          </pre>
        </div>

        {/* 操作按钮 */}
        {onDelete && (
          <div className="mt-8 pt-4 border-t border-gray-200 dark:border-gray-700">
            <button
              onClick={() => onDelete(article.id)}
              className="px-4 py-2 text-sm text-red-600 border border-red-300 rounded hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
            >
              删除文章
            </button>
          </div>
        )}
      </div>
    </div>
  )
}
```

### 8.6 NewPlanetDialog 对话框

创建 `src/components/NewPlanetDialog.tsx`：

```tsx
import React, { useState } from 'react'

interface NewPlanetDialogProps {
  open: boolean
  onClose: () => void
  onCreate: (name: string, about: string) => Promise<void>
}

export function NewPlanetDialog({ open, onClose, onCreate }: NewPlanetDialogProps) {
  const [name, setName] = useState('')
  const [about, setAbout] = useState('')
  const [loading, setLoading] = useState(false)

  if (!open) return null

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) return

    setLoading(true)
    try {
      await onCreate(name.trim(), about.trim())
      setName('')
      setAbout('')
      onClose()
    } catch (err) {
      console.error('Failed to create planet:', err)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-[480px] max-w-[90vw]">
        <form onSubmit={handleSubmit}>
          <div className="p-6">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
              New Planet
            </h2>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  名称
                </label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="My Blog"
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 outline-none"
                  autoFocus
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  简介
                </label>
                <textarea
                  value={about}
                  onChange={(e) => setAbout(e.target.value)}
                  placeholder="A personal blog about..."
                  rows={3}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 outline-none resize-none"
                />
              </div>
            </div>
          </div>

          <div className="flex justify-end gap-3 p-4 border-t border-gray-200 dark:border-gray-700">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={!name.trim() || loading}
              className="px-4 py-2 text-sm bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {loading ? '创建中...' : '创建'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
```

### 8.7 NewArticleDialog 对话框

创建 `src/components/NewArticleDialog.tsx`：

```tsx
import React, { useState } from 'react'

interface NewArticleDialogProps {
  open: boolean
  onClose: () => void
  onCreate: (title: string, content: string) => Promise<void>
}

export function NewArticleDialog({ open, onClose, onCreate }: NewArticleDialogProps) {
  const [title, setTitle] = useState('')
  const [content, setContent] = useState('')
  const [loading, setLoading] = useState(false)

  if (!open) return null

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!title.trim()) return

    setLoading(true)
    try {
      await onCreate(title.trim(), content)
      setTitle('')
      setContent('')
      onClose()
    } catch (err) {
      console.error('Failed to create article:', err)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-[640px] max-w-[90vw] max-h-[80vh] flex flex-col">
        <form onSubmit={handleSubmit} className="flex flex-col flex-1">
          <div className="p-6 flex-1 overflow-y-auto">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
              New Article
            </h2>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  标题
                </label>
                <input
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="Article title..."
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 outline-none"
                  autoFocus
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  内容（Markdown）
                </label>
                <textarea
                  value={content}
                  onChange={(e) => setContent(e.target.value)}
                  placeholder="Write your article in Markdown..."
                  rows={12}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 outline-none resize-none font-mono text-sm"
                />
              </div>
            </div>
          </div>

          <div className="flex justify-end gap-3 p-4 border-t border-gray-200 dark:border-gray-700">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={!title.trim() || loading}
              className="px-4 py-2 text-sm bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {loading ? '创建中...' : '创建'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
```

### 8.8 更新 App.tsx 主布局

更新 `src/App.tsx`，集成所有组件：

```tsx
import React, { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import { Sidebar } from './components/Sidebar'
import { ArticleList } from './components/ArticleList'
import { ArticleDetail } from './components/ArticleDetail'
import { NewPlanetDialog } from './components/NewPlanetDialog'
import { NewArticleDialog } from './components/NewArticleDialog'
import { usePlanetStore, useArticles } from './hooks/usePlanetStore'
import type { MyArticle } from './types/planet'

function App() {
  // 全局状态
  const { myPlanets, loading: planetsLoading, createPlanet, deletePlanet } = usePlanetStore()

  // 选中状态
  const [selectedPlanetId, setSelectedPlanetId] = useState<string | null>(null)
  const [selectedArticle, setSelectedArticle] = useState<MyArticle | null>(null)

  // 文章列表
  const {
    articles,
    loading: articlesLoading,
    createArticle,
    deleteArticle,
  } = useArticles(selectedPlanetId)

  // 对话框状态
  const [showNewPlanet, setShowNewPlanet] = useState(false)
  const [showNewArticle, setShowNewArticle] = useState(false)

  // 选中文章
  const handleSelectArticle = useCallback(
    (articleId: string) => {
      const article = articles.find((a) => a.id === articleId)
      setSelectedArticle(article || null)
    },
    [articles]
  )

  // 创建 Planet
  const handleCreatePlanet = useCallback(
    async (name: string, about: string) => {
      const planet = await createPlanet(name, about)
      setSelectedPlanetId(planet.id)
    },
    [createPlanet]
  )

  // 创建文章
  const handleCreateArticle = useCallback(
    async (title: string, content: string) => {
      const article = await createArticle(title, content)
      setSelectedArticle(article)
    },
    [createArticle]
  )

  // 删除文章
  const handleDeleteArticle = useCallback(
    async (articleId: string) => {
      await deleteArticle(articleId)
      setSelectedArticle(null)
    },
    [deleteArticle]
  )

  if (planetsLoading) {
    return (
      <div className="h-screen flex items-center justify-center bg-white dark:bg-gray-950">
        <span className="text-gray-400">Loading...</span>
      </div>
    )
  }

  return (
    <div className="h-screen flex bg-white dark:bg-gray-950">
      {/* 左侧：Planet 列表 */}
      <Sidebar
        planets={myPlanets}
        selectedPlanetId={selectedPlanetId}
        onSelectPlanet={setSelectedPlanetId}
        onCreatePlanet={() => setShowNewPlanet(true)}
      />

      {/* 中间：文章列表 */}
      {selectedPlanetId && (
        <ArticleList
          articles={articles}
          selectedArticleId={selectedArticle?.id || null}
          onSelectArticle={handleSelectArticle}
          onCreateArticle={() => setShowNewArticle(true)}
          loading={articlesLoading}
        />
      )}

      {/* 右侧：文章详情 */}
      {selectedPlanetId ? (
        <ArticleDetail
          article={selectedArticle}
          onDelete={handleDeleteArticle}
        />
      ) : (
        <div className="flex-1 flex items-center justify-center text-gray-400">
          <div className="text-center">
            <div className="text-4xl mb-4">🪐</div>
            <div className="text-lg">选择一个 Planet 开始</div>
            <div className="text-sm mt-2">或点击左侧 + 创建新 Planet</div>
          </div>
        </div>
      )}

      {/* 对话框 */}
      <NewPlanetDialog
        open={showNewPlanet}
        onClose={() => setShowNewPlanet(false)}
        onCreate={handleCreatePlanet}
      />
      <NewArticleDialog
        open={showNewArticle}
        onClose={() => setShowNewArticle(false)}
        onCreate={handleCreateArticle}
      />
    </div>
  )
}

export default App
```

### 8.9 验证前端

```bash
npm run dev
# 打开浏览器查看三栏布局
```

---

## 9. Step 7：测试与调试

### 9.1 功能验收清单

按顺序测试以下功能：

| # | 测试项 | 操作 | 预期结果 |
|---|--------|------|----------|
| 1 | 创建 Planet | 点击 + → 填写名称和简介 → 创建 | 左侧列表出现新 Planet |
| 2 | 查看 Planet | 点击 Planet | 中间栏显示文章列表（空） |
| 3 | 创建文章 | 点击 New → 填写标题和 Markdown → 创建 | 文章列表出现新文章 |
| 4 | 查看文章 | 点击文章 | 右侧显示文章标题和 Markdown 内容 |
| 5 | 删除文章 | 在文章详情点击删除 | 文章从列表消失 |
| 6 | 删除 Planet | 在 Planet 上右键删除（后续实现） | Planet 和所有文章被删除 |
| 7 | 持久化 | 关闭应用，重新打开 | 所有数据还在 |
| 8 | 多 Planet | 创建 3 个以上 Planet | 列表正常，切换正常 |
| 9 | 多文章 | 在一个 Planet 下创建 10+ 文章 | 滚动正常，排序正确（最新在前） |

### 9.2 数据文件验证

手动检查数据文件是否正确生成：

```bash
# Windows
dir %APPDATA%\planet-desktop\Planet\My\
type %APPDATA%\planet-desktop\Planet\My\<uuid>\planet.json

# macOS
ls ~/Library/Application\ Support/planet-desktop/Planet/My/
cat ~/Library/Application\ Support/planet-desktop/Planet/My/<uuid>/planet.json
```

预期输出示例：

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "My Blog",
  "about": "A personal blog",
  "created": "2024-01-01T00:00:00Z",
  "ipns": "k51qzi5uqu5...",
  "updated": "2024-01-15T10:30:00Z",
  "template_name": "Plain",
  "archived": false,
  "do_not_index": false,
  "prewarm_new_post": true
}
```

### 9.3 常见问题排查

| 问题 | 可能原因 | 解决方案 |
|------|---------|---------|
| 创建 Planet 失败 | 数据目录无写入权限 | 检查 `get_data_path()` 返回值及权限 |
| 重启后数据丢失 | `load_all()` 路径不正确 | 在 Rust 日志中搜索 "Loading planets" |
| 文章列表不刷新 | 前端未监听 `planet:state-changed` 事件 | 检查 `usePlanetStore` hook 中的 `listen` |
| JSON 反序列化失败 | 字段名大小写不一致 | 确认 serde 的 `rename_all` 配置 |
| UUID 解析错误 | 前端传递的 ID 格式不对 | 在 DevTools 中检查 invoke 参数 |
| `emit_all` 报错 | `tauri.conf.json` 缺少配置 | 确认 `withGlobalTauri: true` |
| Mutex 死锁 | 嵌套 lock 调用 | 确保每个 command 只 lock 一次 |

### 9.4 调试命令

```bash
# 查看 Rust 后端日志
RUST_LOG=debug cargo tauri dev

# 前端 DevTools Console 中测试命令
await window.__TAURI__.invoke('planet_list')
await window.__TAURI__.invoke('planet_create', { request: { name: 'Test', about: 'test' } })
await window.__TAURI__.invoke('article_list', { planetId: '<uuid>' })
await window.__TAURI__.invoke('article_create', { request: { planet_id: '<uuid>', title: 'Hello', content: '# Hello' } })
```

---

## 10. 文件清单

Phase 2 完成后，以下文件应已创建或修改：

### 10.1 新建文件

| 文件路径 | 说明 |
|----------|------|
| `src-tauri/src/models/planet.rs` | MyPlanet / FollowingPlanet / PublicPlanet 数据模型 |
| `src-tauri/src/models/article.rs` | MyArticle / FollowingArticle / PublicArticle / Attachment 数据模型 |
| `src-tauri/src/models/draft.rs` | Draft 数据模型 |
| `src-tauri/src/store/mod.rs` | PlanetStore 全局状态管理 |
| `src-tauri/src/commands/planet.rs` | Planet 相关 Tauri Commands |
| `src-tauri/src/commands/article.rs` | Article / Draft 相关 Tauri Commands |
| `src/types/planet.ts` | TypeScript 类型定义 |
| `src/hooks/usePlanetStore.ts` | React Hook：Planet 和 Article 状态管理 |
| `src/components/Sidebar.tsx` | 左侧 Planet 列表组件 |
| `src/components/ArticleList.tsx` | 中间文章列表组件 |
| `src/components/ArticleDetail.tsx` | 右侧文章详情组件 |
| `src/components/NewPlanetDialog.tsx` | 创建 Planet 对话框 |
| `src/components/NewArticleDialog.tsx` | 创建 Article 对话框 |

### 10.2 修改文件

| 文件路径 | 改动内容 |
|----------|---------|
| `src-tauri/Cargo.toml` | 添加 `uuid`, `chrono` 依赖 |
| `src-tauri/src/main.rs` | 集成 PlanetStore 状态、注册新 Commands |
| `src-tauri/src/models/mod.rs` | 声明 planet / article / draft 子模块 |
| `src-tauri/src/commands/mod.rs` | 声明 planet / article 子模块 |
| `src/App.tsx` | 集成三栏布局、连接所有组件 |

---

## 11. Swift → Rust 对照表

| 分类 | Swift (原项目) | Rust (新项目) |
|------|---------------|--------------|
| **文件** | `MyPlanetModel.swift` | `models/planet.rs` (MyPlanet) |
| **文件** | `FollowingPlanetModel.swift` | `models/planet.rs` (FollowingPlanet) |
| **文件** | `MyArticleModel.swift` | `models/article.rs` (MyArticle) |
| **文件** | `FollowingArticleModel.swift` | `models/article.rs` (FollowingArticle) |
| **文件** | `DraftModel.swift` | `models/draft.rs` (Draft) |
| **文件** | `PlanetStore.swift` | `store/mod.rs` (PlanetStore) |
| **文件** | `PlanetAPIController.swift` | `commands/planet.rs` + `commands/article.rs` |
| **序列化** | `Codable` + `JSONEncoder/Decoder` | `serde::Serialize/Deserialize` + `serde_json` |
| **UUID** | `Foundation.UUID` | `uuid::Uuid` |
| **时间** | `Foundation.Date` | `chrono::DateTime<Utc>` |
| **文件操作** | `FileManager.default` | `std::fs` |
| **路径** | `Foundation.URL` | `std::path::PathBuf` |
| **全局状态** | `@MainActor class PlanetStore: ObservableObject` | `Arc<Mutex<PlanetStore>>` |
| **状态推送** | `@Published` 自动刷新 SwiftUI | `app.emit_all("planet:state-changed")` → `listen()` |
| **枚举类型** | `PlanetType: Int, Codable` | `PlanetType: Serialize, Deserialize` |
| **错误处理** | `throws` + `PlanetError` | `Result<T, anyhow::Error>` |
| **可选值** | `Optional<T>` | `Option<T>` |
| **字典** | `[String: String]` | `HashMap<String, String>` |
| **数组** | `[MyPlanetModel]` | `Vec<MyPlanet>` |
| **目录创建** | `FileManager.createDirectory(withIntermediateDirectories:)` | `fs::create_dir_all()` |
| **JSON 文件读写** | `Data(contentsOf:)` + `JSONDecoder` | `fs::read_to_string()` + `serde_json::from_str()` |
| **排序** | `articles.sorted(by:)` | `articles.sort_by(|a, b| ...)` |
| **查找** | `planets.first(where:)` | `planets.iter().find(|p| ...)` |
| **删除目录** | `FileManager.removeItem(at:)` | `fs::remove_dir_all()` |

---

## 12. 执行顺序总结

```
Step 1 → models/planet.rs    (MyPlanet / FollowingPlanet，无依赖)
Step 2 → models/article.rs   (MyArticle / FollowingArticle，依赖 Step 1)
Step 3 → models/draft.rs     (Draft，依赖 Step 1 + 2)
Step 4 → store/mod.rs        (PlanetStore 全局状态，依赖 Step 1+2+3)
Step 5 → commands/planet.rs  (Planet Commands，依赖 Step 4)
       → commands/article.rs (Article/Draft Commands，依赖 Step 4)
       → main.rs 集成         (注册 Commands + State)
Step 6 → 前端组件              (types → hooks → 组件 → App.tsx)
Step 7 → 测试验证              (功能验收 + 持久化检查)
```

**每完成一个 Step 都运行 `cargo check` 确保编译通过。**

---

> **Phase 2 完成后，你将拥有一个完整的三栏式 Planet 管理界面：左侧选 Planet、中间看文章列表、右侧看文章详情。数据以 JSON 文件持久化在本地，重启后不丢失。这是后续 Phase 3（发布流程）和 Phase 4（关注与订阅）的坚实基础。**