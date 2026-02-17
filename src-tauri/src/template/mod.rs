use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tera::Tera;
use tracing::{debug, error, warn};

use crate::helpers::markdown::render_markdown_html;

// ============================================================
// 模板元数据
// ============================================================

/// 模板设置项定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSetting {
    pub name: String,
    #[serde(rename = "type")]
    pub setting_type: String,
    #[serde(rename = "defaultValue")]
    pub default_value: String,
    pub description: String,
    #[serde(default)]
    pub advanced: bool,
}

/// 模板元数据，从 template.json 反序列化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    #[serde(rename = "idealItemsPerPage", default = "default_items_per_page")]
    pub ideal_items_per_page: usize,
    #[serde(rename = "generateIndexPagination", default)]
    pub generate_index_pagination: bool,
    #[serde(rename = "generateTagPages", default)]
    pub generate_tag_pages: bool,
    #[serde(rename = "generateArchive", default)]
    pub generate_archive: bool,
    #[serde(rename = "buildNumber", default = "default_build_number")]
    pub build_number: u32,
    #[serde(rename = "generateNFTMetadata", default)]
    pub generate_nft_metadata: bool,
    #[serde(default)]
    pub settings: HashMap<String, TemplateSetting>,
}

fn default_items_per_page() -> usize {
    10
}
fn default_build_number() -> u32 {
    1
}

/// 完整的模板对象
#[derive(Debug, Clone)]
pub struct Template {
    pub info: TemplateInfo,
    pub path: PathBuf,
}

impl Template {
    /// 从目录加载模板
    pub fn from_path(dir: &Path) -> Result<Self> {
        let json_path = dir.join("template.json");
        let data = fs::read_to_string(&json_path)
            .with_context(|| format!("读取 template.json 失败: {:?}", json_path))?;
        let info: TemplateInfo = serde_json::from_str(&data)
            .with_context(|| format!("解析 template.json 失败: {:?}", json_path))?;
        Ok(Template {
            info,
            path: dir.to_path_buf(),
        })
    }

    // ---- 路径工具 ----

    pub fn blog_path(&self) -> PathBuf {
        self.path.join("templates").join("blog.html")
    }

    pub fn index_path(&self) -> PathBuf {
        self.path.join("templates").join("index.html")
    }

    pub fn tags_path(&self) -> PathBuf {
        self.path.join("templates").join("tags.html")
    }

    pub fn archive_path(&self) -> PathBuf {
        self.path.join("templates").join("archive.html")
    }

    pub fn assets_path(&self) -> PathBuf {
        self.path.join("assets")
    }

    pub fn style_css_path(&self) -> PathBuf {
        self.path.join("assets").join("style.css")
    }

    pub fn has_tags_html(&self) -> bool {
        self.tags_path().exists()
    }

    pub fn has_archive_html(&self) -> bool {
        self.archive_path().exists()
    }

    /// 计算 style.css 的 SHA256 哈希
    pub fn style_css_hash(&self) -> Option<String> {
        let css_path = self.style_css_path();
        if let Ok(data) = fs::read(&css_path) {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let result = hasher.finalize();
            Some(hex::encode(result))
        } else {
            None
        }
    }

    // ---- 渲染方法 ----

    /// 渲染文章页面 (blog.html 模板)
    ///
    /// 对标 Swift Template.render(article:)
    pub fn render_article(&self, context: &tera::Context) -> Result<String> {
        let template_str = fs::read_to_string(self.blog_path())
            .with_context(|| "读取 blog.html 模板失败")?;
        let mut tera = Tera::default();
        // 注册自定义 filter
        register_filters(&mut tera);
        tera.add_raw_template("blog.html", &template_str)?;
        let rendered = tera.render("blog.html", context)?;
        Ok(rendered)
    }

    /// 渲染首页 (index.html 模板)
    ///
    /// 对标 Swift Template.renderIndex(context:)
    pub fn render_index(&self, context: &tera::Context) -> Result<String> {
        let template_str = fs::read_to_string(self.index_path())
            .with_context(|| "读取 index.html 模板失败")?;
        let mut tera = Tera::default();
        register_filters(&mut tera);
        tera.add_raw_template("index.html", &template_str)?;
        let rendered = tera.render("index.html", context)?;
        Ok(rendered)
    }

    /// 渲染标签页 (tags.html 模板)
    pub fn render_tags(&self, context: &tera::Context) -> Result<String> {
        let template_str = fs::read_to_string(self.tags_path())
            .with_context(|| "读取 tags.html 模板失败")?;
        let mut tera = Tera::default();
        register_filters(&mut tera);
        tera.add_raw_template("tags.html", &template_str)?;
        let rendered = tera.render("tags.html", context)?;
        Ok(rendered)
    }

    /// 渲染归档页 (archive.html 模板)
    pub fn render_archive(&self, context: &tera::Context) -> Result<String> {
        let template_str = fs::read_to_string(self.archive_path())
            .with_context(|| "读取 archive.html 模板失败")?;
        let mut tera = Tera::default();
        register_filters(&mut tera);
        tera.add_raw_template("archive.html", &template_str)?;
        let rendered = tera.render("archive.html", context)?;
        Ok(rendered)
    }
}

// ============================================================
// 自定义 Tera Filters (对标 Swift StencilExtension)
// ============================================================

/// 注册所有自定义 filter
/// 
/// **注意**：需要导出为 `pub` 以便在其他模块中使用（如 RSS 渲染）
pub fn register_filters(tera: &mut Tera) {
    tera.register_filter("md2html", md2html_filter);
    tera.register_filter("escape", escape_filter);
    tera.register_filter("rfc822", rfc822_filter);
    tera.register_filter("hhmmss", hhmmss_filter);
    tera.register_filter("absoluteImageURL", absolute_image_url_filter);
}

/// md2html: 将 Markdown 转为 HTML
/// 对标 Swift StencilExtension.common 中的 "md2html" filter
fn md2html_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    match value.as_str() {
        Some(md) => Ok(tera::Value::String(render_markdown_html(md))),
        None => Ok(value.clone()),
    }
}

/// escape: HTML 转义
fn escape_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    match value.as_str() {
        Some(s) => {
            let escaped = s
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#x27;");
            Ok(tera::Value::String(escaped))
        }
        None => Ok(value.clone()),
    }
}

/// rfc822: 将 ISO 日期字符串转为 RFC 822 格式
/// 用于 RSS <pubDate>
fn rfc822_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    match value.as_str() {
        Some(date_str) => {
            // 尝试解析 ISO 8601 格式
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
                Ok(tera::Value::String(dt.format("%a, %d %b %Y %H:%M:%S %z").to_string()))
            } else {
                // 尝试解析 serde_json 的日期格式
                Ok(tera::Value::String(date_str.to_string()))
            }
        }
        None => Ok(value.clone()),
    }
}

/// hhmmss: 将秒数转为 HH:MM:SS 格式
/// 用于 podcast duration
fn hhmmss_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    match value.as_i64() {
        Some(seconds) => {
            let h = seconds / 3600;
            let m = (seconds % 3600) / 60;
            let s = seconds % 60;
            let formatted = if h > 0 {
                format!("{:02}:{:02}:{:02}", h, m, s)
            } else {
                format!("{:02}:{:02}", m, s)
            };
            Ok(tera::Value::String(formatted))
        }
        None => Ok(value.clone()),
    }
}

/// absoluteImageURL: 将相对图片 URL 转为绝对 URL
/// 对标 Swift StencilExtension 中的 "absoluteImageURL" filter
fn absolute_image_url_filter(
    value: &tera::Value,
    args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    match value.as_str() {
        Some(html) => {
            let root_prefix = args
                .get("root_prefix")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let article_id = args
                .get("article_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // 简单替换: src="xxx.png" → src="{root_prefix}/{article_id}/xxx.png"
            // 只替换不以 http 开头的相对路径
            let re = regex::Regex::new(r#"src="(?!https?://)(.*?)""#).unwrap();
            let result = re.replace_all(html, |caps: &regex::Captures| {
                let path = &caps[1];
                format!(r#"src="{}/{}/{}""#, root_prefix, article_id, path)
            });
            Ok(tera::Value::String(result.to_string()))
        }
        None => Ok(value.clone()),
    }
}

// ============================================================
// 模板仓库 (TemplateStore)
// ============================================================

/// 管理所有已安装的模板
///
/// 对标 Swift TemplateStore
pub struct TemplateStore {
    templates: HashMap<String, Template>,
}

impl TemplateStore {
    /// 初始化模板目录：从资源目录复制模板到应用数据目录
    ///
    /// 如果模板目录不存在或为空，则从资源目录复制默认模板
    pub fn initialize_templates(resource_dir: &Path, templates_dir: &Path) -> Result<()> {
        // 确保模板目录存在
        if !templates_dir.exists() {
            fs::create_dir_all(templates_dir)?;
        }

        // 检查资源目录中的模板
        let resource_templates_dir = resource_dir.join("templates");
        if !resource_templates_dir.exists() {
            warn!("资源目录中未找到模板目录: {:?}", resource_templates_dir);
            return Ok(());
        }

        // 遍历资源目录中的模板
        if let Ok(entries) = fs::read_dir(&resource_templates_dir) {
            for entry in entries.flatten() {
                let src_path = entry.path();
                if src_path.is_dir() {
                    // 获取模板名称（目录名）
                    if let Some(template_name) = src_path.file_name().and_then(|n| n.to_str()) {
                        let dst_path = templates_dir.join(template_name);
                        
                        // 如果目标模板不存在，则复制
                        if !dst_path.exists() {
                            debug!("复制模板 '{}' 从 {:?} 到 {:?}", template_name, src_path, dst_path);
                            copy_dir_recursive(&src_path, &dst_path)
                                .with_context(|| format!("复制模板 '{}' 失败", template_name))?;
                        } else {
                            // 如果目标已存在，检查是否需要更新（可选：比较版本号）
                            debug!("模板 '{}' 已存在，跳过复制", template_name);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 从模板目录加载所有模板
    pub fn load(templates_dir: &Path) -> Result<Self> {
        let mut templates = HashMap::new();

        if !templates_dir.exists() {
            fs::create_dir_all(templates_dir)?;
        }

        if let Ok(entries) = fs::read_dir(templates_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    match Template::from_path(&path) {
                        Ok(tmpl) => {
                            debug!("加载模板: {} (v{})", tmpl.info.name, tmpl.info.version);
                            templates.insert(tmpl.info.name.clone(), tmpl);
                        }
                        Err(e) => {
                            warn!("加载模板失败 {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        debug!("共加载 {} 个模板", templates.len());
        Ok(TemplateStore { templates })
    }

    /// 根据名称获取模板
    pub fn get(&self, name: &str) -> Option<&Template> {
        self.templates.get(name)
    }

    /// 获取所有模板名称
    pub fn template_names(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
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
            fs::copy(&src_path, &dst_path)
                .with_context(|| format!("复制文件失败: {:?} -> {:?}", src_path, dst_path))?;
        }
    }
    Ok(())
}