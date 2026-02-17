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

// ============================================================
// 单元测试
// ============================================================

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