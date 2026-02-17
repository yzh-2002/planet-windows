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

        // 按字符数截取，避免在多字节字符（如中文）中间截断导致 panic
        let char_count = trimmed.chars().count();
        if char_count > 280 {
            let end: String = trimmed.chars().take(280).collect();
            Some(format!("{}...", end))
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