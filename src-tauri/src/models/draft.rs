use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::{anyhow, Result};
use tracing::{debug, info};
use tauri::AppHandle;

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
    pub fn base_path(&self, planet: &MyPlanet, app: &AppHandle) -> PathBuf {
        if let Some(article_id) = self.article_id {
            // 编辑现有文章的草稿
            planet.articles_path(app)
                .join("Drafts")
                .join(article_id.to_string())
        } else {
            // 新文章草稿
            planet.drafts_path(app).join(self.id.to_string())
        }
    }

    /// Draft.json 文件路径
    pub fn info_path(&self, planet: &MyPlanet, app: &AppHandle) -> PathBuf {
        self.base_path(planet, app).join("Draft.json")
    }

    /// 附件目录路径
    pub fn attachments_path(&self, planet: &MyPlanet, app: &AppHandle) -> PathBuf {
        self.base_path(planet, app).join("Attachments")
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
    pub fn load(planet: &MyPlanet, draft_id: Uuid, app: &AppHandle) -> Result<Self> {
        // 先尝试在新文章草稿目录
        let new_draft_path = planet.drafts_path(app).join(draft_id.to_string()).join("Draft.json");
        if new_draft_path.exists() {
            let content = fs::read_to_string(&new_draft_path)?;
            let draft: Self = serde_json::from_str(&content)?;
            if draft.id == draft_id {
                return Ok(draft);
            }
        }

        // 再尝试在文章草稿目录中查找
        let articles_drafts_path = planet.articles_path(app).join("Drafts");
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
    pub fn load_all(planet: &MyPlanet, app: &AppHandle) -> Result<Vec<Self>> {
        let mut drafts = Vec::new();

        // 加载新文章草稿
        let drafts_path = planet.drafts_path(app);
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
        let articles_drafts_path = planet.articles_path(app).join("Drafts");
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
    pub fn save(&self, planet: &MyPlanet, app: &AppHandle) -> Result<()> {
        let base_path = self.base_path(planet, app);
        fs::create_dir_all(&base_path)?;
        fs::create_dir_all(self.attachments_path(planet, app))?;

        let info_path = self.info_path(planet, app);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&info_path, content)?;
        debug!("Saved draft: {}", self.id);
        Ok(())
    }

    /// 删除草稿
    pub fn delete(&self, planet: &MyPlanet, app: &AppHandle) -> Result<()> {
        let base_path = self.base_path(planet, app);
        if base_path.exists() {
            fs::remove_dir_all(&base_path)?;
            info!("Deleted draft: {}", self.id);
        }
        Ok(())
    }

    /// 将草稿发布为文章
    /// 如果 article_id 为 None，创建新文章；否则更新现有文章
    pub fn publish_to_article(&self, planet: &mut MyPlanet, app: &AppHandle) -> Result<MyArticle> {
        let article = if let Some(article_id) = self.article_id {
            // 更新现有文章
            let mut article = MyArticle::load(planet, article_id, app)?;
            article.update(planet, |a| {
                a.title = self.title.clone();
                a.content = self.content.clone();
                a.hero_image = self.hero_image.clone();
                a.external_link = self.external_link.clone();
                a.tags = self.tags.clone();
                // 附件在 Phase 3+ 实现完整拷贝逻辑
            }, app)?;
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
            article.save(planet, app)?;
            info!("Created article from draft: {} -> {}", self.id, article.id);
            article
        };

        // 删除草稿
        self.delete(planet, app)?;

        // 更新 Planet 时间戳
        planet.updated = Utc::now();
        planet.save(app)?;

        Ok(article)
    }
}
