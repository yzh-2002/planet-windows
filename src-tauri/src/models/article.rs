use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::{anyhow, Context, Result};
use tracing::{debug, error, info};
use tauri::AppHandle;

use crate::models::planet::MyPlanet;
use crate::helpers::markdown::render_markdown_html;
use crate::helpers::paths;
use crate::template::Template;

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
    pub fn path(&self, planet: &MyPlanet, app: &AppHandle) -> PathBuf {
        planet.articles_path(app).join(format!("{}.json", self.id))
    }

    /// 获取 Article 附件目录路径
    pub fn attachments_path(&self, planet: &MyPlanet, app: &AppHandle) -> PathBuf {
        planet.articles_path(app).join(self.id.to_string()).join("Attachments")
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
    pub fn load(planet: &MyPlanet, article_id: Uuid, app: &AppHandle) -> Result<Self> {
        let article_path = planet.articles_path(app).join(format!("{}.json", article_id));

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
    pub fn load_all(planet: &MyPlanet, app: &AppHandle) -> Result<Vec<Self>> {
        let articles_path = planet.articles_path(app);
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
                        match Self::load(planet, article_id, app) {
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
    pub fn save(&self, planet: &MyPlanet, app: &AppHandle) -> Result<()> {
        let article_path = self.path(planet, app);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&article_path, content)?;
        debug!("Saved article: {} ({})", self.title, self.id);
        Ok(())
    }

    /// 更新文章
    pub fn update<F>(&mut self, planet: &MyPlanet, f: F, app: &AppHandle) -> Result<()>
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated = Utc::now();
        self.save(planet, app)?;
        Ok(())
    }

    /// 删除文章
    pub fn delete(&self, planet: &MyPlanet, app: &AppHandle) -> Result<()> {
        let article_path = self.path(planet, app);
        if article_path.exists() {
            fs::remove_file(&article_path)?;
        }

        // 删除附件目录
        let attachments_path = self.attachments_path(planet, app);
        if attachments_path.exists() {
            fs::remove_dir_all(&attachments_path)?;
        }

        info!("Deleted article: {} ({})", self.title, self.id);
        Ok(())
    }

    /// 添加附件
    pub fn add_attachment(&mut self, planet: &MyPlanet, attachment: Attachment, app: &AppHandle) -> Result<()> {
        // 创建附件目录
        let attachments_path = self.attachments_path(planet, app);
        fs::create_dir_all(&attachments_path)?;

        self.attachments.push(attachment);
        self.save(planet, app)?;
        Ok(())
    }

    /// 删除附件
    pub fn remove_attachment(&mut self, planet: &MyPlanet, name: &str, app: &AppHandle) -> Result<()> {
        self.attachments.retain(|a| a.name != name);
        self.save(planet, app)?;
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
    pub fn path(&self, planet: &crate::models::planet::FollowingPlanet, app: &AppHandle) -> PathBuf {
        planet.articles_path(app).join(format!("{}.json", self.id))
    }

    pub fn load(
        planet: &crate::models::planet::FollowingPlanet,
        article_id: Uuid,
        app: &AppHandle,
    ) -> Result<Self> {
        let article_path = planet.articles_path(app).join(format!("{}.json", article_id));

        if !article_path.exists() {
            return Err(anyhow!("Following article not found: {}", article_id));
        }

        let content = fs::read_to_string(&article_path)?;
        let article: Self = serde_json::from_str(&content)?;

        Ok(article)
    }

    pub fn load_all(planet: &crate::models::planet::FollowingPlanet, app: &AppHandle) -> Result<Vec<Self>> {
        let articles_path = planet.articles_path(app);
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
                        match Self::load(planet, article_id, app) {
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

    pub fn save(&self, planet: &crate::models::planet::FollowingPlanet, app: &AppHandle) -> Result<()> {
        let article_path = self.path(planet, app);
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

impl MyArticle {
    /// 将文章渲染为静态 HTML 文件
    ///
    /// 对标 Swift MyArticleModel.savePublic()
    /// 
    /// **注意**：实际实现需要传入 `app: &AppHandle` 参数用于路径获取。
    pub fn save_public(
        &mut self,
        planet: &MyPlanet,
        template: &Template,
        app: &AppHandle,
    ) -> Result<()> {
        // 构建 public 目录路径
        // 注意：planet.public_base_path() 方法将在 Step 5 中实现
        // 这里先直接构建路径
        let public_path = paths::get_data_path(app).join("Public");
        std::fs::create_dir_all(&public_path).ok();
        let planet_public_path = public_path.join(planet.id.to_string());
        let public_base = planet_public_path.join(self.id.to_string());

        // 1. 创建 public 目录
        fs::create_dir_all(&public_base)?;

        // 2. Markdown → HTML
        // 注意：MyArticle 结构体中没有 content_rendered 字段，需要在渲染时计算
        let content_html = if !self.content.is_empty() {
            render_markdown_html(&self.content)
        } else {
            String::new()
        };

        // 3. 用模板渲染文章页面
        let mut context = tera::Context::new();
        
        // 创建包含 content_rendered 的 PublicArticle 用于模板渲染
        // 注意：将 &mut self 转换为 &self，因为 From<&MyArticle> 已实现
        let mut public_article = PublicArticle::from(&*self);
        public_article.content_rendered = Some(content_html.clone());

        // 构建模板上下文（对标 Swift Template.render(article:) 的 context）
        let public_planet = crate::models::planet::PublicPlanet::from(planet);
        context.insert("planet", &public_planet);
        context.insert("planet_ipns", &planet.ipns.as_ref());
        context.insert("assets_prefix", "../");
        context.insert("article_id", &self.id.to_string());  // Uuid 需要转换为 String
        context.insert("article", &public_article);
        context.insert("article_title", &self.title);
        context.insert("page_title", &self.title);
        context.insert("content_html", &content_html);
        context.insert("style_css_sha256", &template.style_css_hash().unwrap_or_default());
        context.insert("build_timestamp", &chrono::Utc::now().timestamp());

        // 渲染 blog.html → index.html
        let article_html = template.render_article(&context)
            .with_context(|| format!("渲染文章 '{}' 失败", self.title))?;
        let index_path = public_base.join("index.html");
        fs::write(&index_path, article_html.as_bytes())
            .with_context(|| format!("写入文章 HTML 失败: {:?}", index_path))?;

        debug!("文章 '{}' 静态化完成: {:?}", self.title, index_path);

        // 4. 写入 article.json
        let info_path = public_base.join("article.json");
        let info_json = serde_json::to_string_pretty(&public_article)?;
        fs::write(&info_path, info_json.as_bytes())
            .with_context(|| format!("写入 article.json 失败: {:?}", info_path))?;

        // 5. 保存 article.md (原始 Markdown)
        let md_path = public_base.join("article.md");
        let md_content = format!("{}\n\n{}", self.title, self.content);
        fs::write(&md_path, md_content.as_bytes())
            .with_context(|| format!("写入 article.md 失败: {:?}", md_path))?;

        Ok(())
    }
}
