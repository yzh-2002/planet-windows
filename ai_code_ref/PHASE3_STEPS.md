# Phase 3：发布流程 — 详细开发文档

> **目标**：实现内容发布到 IPFS/IPNS 的完整流程，包含模板渲染、Markdown→HTML 转换、RSS 生成、IPNS 发布，以及可选的 Filebase/Pinnable 远程 Pinning 集成。对应原项目 `MyPlanetModel.publish()`、`MyPlanetModel.savePublic()`、`MyArticleModel.savePublic()`、`Template.swift`、`KeychainHelper.swift`。
>
> **预计工期**：2 周
>
> **验收标准**：创建 Planet → 写文章 → 点击 Publish，能看到发布进度，发布完成后显示 CID。通过 `http://127.0.0.1:{gateway_port}/ipns/{ipns_key}` 可以访问到发布的站点。重启应用后 IPNS key 仍然存在且可用。

## ⚠️ 重要说明：文档与实际代码的差异

本文档在编写时基于代码库的当前实现进行了修正，但请注意以下关键差异：

1. **结构体名称**：
   - 文档中使用 `Article` 和 `Planet`，实际代码中使用 `MyArticle` 和 `MyPlanet`
   - 文档中使用 `PublicArticle` 和 `PublicPlanet`，实际代码中已存在但字段可能略有不同

2. **ID 类型**：
   - 文档中部分示例使用 `String` 作为 ID，实际代码中使用 `Uuid` 类型
   - 在序列化/反序列化和 API 调用时需要转换：`id.to_string()` 和 `Uuid::parse_str()`

3. **方法签名**：
   - 许多方法需要传入 `app: &AppHandle` 参数用于路径获取
   - `export_key` 和 `import_key` 接受 `&str` 而不是 `&Path`，需要转换
   - `add_directory` 是同步方法，不是异步的
   - `api` 方法接受 `Option<&HashMap<String, String>>` 而不是数组

4. **路径函数**：
   - 文档中使用的 `paths::temporary_path()` 等函数不存在
   - 实际使用 `paths::get_temp_path(app)` 等方法，需要传入 `app` 参数

5. **字段差异**：
   - `MyPlanet` 结构体中可能没有 `is_publishing` 和 `publish_started_at` 字段
   - `PublicArticle` 中 `slug` 是 `String` 而不是 `Option<String>`
   - `PublicPlanet` 中 `created` 和 `updated` 是 `DateTime<Utc>` 而不是 `String`

**建议**：在实现时，请先查看实际代码中的结构体定义和方法签名，然后参考本文档进行实现。

---

## 目录

- [1. 整体架构](#1-整体架构)
- [2. 新增依赖](#2-新增依赖)
- [3. Step 1：实现 Markdown 渲染器 (`helpers/markdown.rs`)](#3-step-1实现-markdown-渲染器-helpersmarkdownrs)
- [4. Step 2：实现模板引擎 (`template/mod.rs`)](#4-step-2实现模板引擎-templatemodrs)
- [5. Step 3：实现 Keystore 密钥管理 (`keystore/mod.rs`)](#5-step-3实现-keystore-密钥管理-keystoremodrs)
- [6. Step 4：实现 savePublic — 文章静态化 (`models/article.rs` 扩展)](#6-step-4实现-savepublic--文章静态化-modelsarticlers-扩展)
- [7. Step 5：实现 savePublic — Planet 站点生成 (`models/planet.rs` 扩展)](#7-step-5实现-savepublic--planet-站点生成-modelsplanetrs-扩展)
- [8. Step 6：实现 Publish 发布到 IPNS (`models/planet.rs` 扩展)](#8-step-6实现-publish-发布到-ipns-modelsplanetrs-扩展)
- [9. Step 7：实现 Filebase 集成 (`integrations/filebase.rs`)](#9-step-7实现-filebase-集成-integrationsfilebasers)
- [10. Step 8：实现 Pinnable 集成 (`integrations/pinnable.rs`)](#10-step-8实现-pinnable-集成-integrationspinnablers)
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
│  PublishButton   ──→  invoke("planet_publish")              │
│  PublishStatus   ←──  listen("publish-state-changed")       │
│  FilebaseSettings     invoke("planet_update_filebase")      │
│  PinnableSettings     invoke("planet_update_pinnable")      │
│                                                             │
└────────────────────────┬────────────────────────────────────┘
                         │ IPC (Tauri invoke / events)
┌────────────────────────┴────────────────────────────────────┐
│                    Rust 后端                                 │
│                                                             │
│  commands/planet.rs                                          │
│    ├── planet_publish()         → 触发完整发布流程           │
│    ├── planet_get_publish_state() → 查询发布状态             │
│    ├── planet_update_filebase()   → 更新 Filebase 设置       │
│    └── planet_update_pinnable()   → 更新 Pinnable 设置       │
│                                                             │
│  models/planet.rs                                            │
│    ├── save_public()  → 渲染模板 → 生成静态站点              │
│    └── publish()      → add_directory → name/publish         │
│                                                             │
│  models/article.rs                                           │
│    └── save_public()  → Markdown→HTML → 渲染文章页面         │
│                                                             │
│  template/mod.rs                                             │
│    ├── Template       → 模板元数据 + Tera 渲染引擎           │
│    └── TemplateStore  → 加载/管理所有模板                     │
│                                                             │
│  helpers/markdown.rs                                         │
│    └── render_markdown_html() → pulldown-cmark               │
│                                                             │
│  keystore/mod.rs                                             │
│    ├── export_key_to_keystore()  → IPFS key → 安全存储       │
│    ├── import_key_from_keystore()→ 安全存储 → IPFS key       │
│    └── check_key() / delete_key()                            │
│                                                             │
│  integrations/                                               │
│    ├── filebase.rs    → Filebase pinning API                 │
│    └── pinnable.rs    → Pinnable.xyz pinning API             │
│                                                             │
│  ipfs/daemon.rs  (Phase 1 已实现)                            │
│    ├── add_directory()    → ipfs add -r                      │
│    ├── name_publish()     → ipfs name publish                │
│    ├── generate_key()     → ipfs key gen                     │
│    ├── check_key_exists() → ipfs key list                    │
│    └── export_key() / import_key()                           │
└─────────────────────────────────────────────────────────────┘
```

### 发布流程概览（对应 Swift `publish()`)

```
Planet.publish()
  │
  ├─ 1. 检查 IPFS key 是否存在
  │     └─ 不存在 → 从 Keystore 恢复 → import_key
  │
  ├─ 2. save_public()  ← 生成静态站点
  │     ├─ 对每篇文章: article.save_public()
  │     │    ├─ Markdown → HTML (pulldown-cmark)
  │     │    ├─ 用模板渲染 blog.html → index.html
  │     │    └─ 写入 article.json
  │     │
  │     ├─ 渲染 RSS / podcast RSS
  │     ├─ 渲染 index.html (首页 + 分页)
  │     ├─ 渲染 tags 页面 (可选)
  │     ├─ 渲染 archive 页面 (可选)
  │     ├─ 写入 planet.json
  │     └─ 复制模板 assets
  │
  ├─ 3. add_directory(public/) → CID
  │
  ├─ 4. name/publish CID → IPNS
  │
  ├─ 5. Filebase pin (可选, 异步)
  │
  └─ 6. Pinnable pin (可选, 异步)
```

---

## 2. 新增依赖

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中添加：

```toml
# Markdown → HTML
pulldown-cmark = { version = "0.10", features = ["simd"] }

# 模板引擎 (类似 Swift Stencil)
tera = "1"

# 密钥安全存储 (跨平台 Keychain/Credential Manager)
keyring = "2"

# SHA256 哈希 (用于 style.css hash)
sha2 = "0.10"
hex = "0.4"
```

> **说明**：
> - `pulldown-cmark`：Rust 最流行的 CommonMark + GFM 扩展解析器，对标 Swift 的 `libcmark_gfm`
> - `tera`：Django/Jinja2 风格模板引擎，对标 Swift 的 `Stencil`
> - `keyring`：跨平台密钥存储，Windows 用 Credential Manager，macOS 用 Keychain
> - `sha2` + `hex`：计算 style.css 的 SHA256 hash

### 已有依赖（Phase 1/2 已添加）

- `tokio`、`reqwest`、`anyhow`、`thiserror`、`tracing`、`serde`、`serde_json`、`uuid`、`chrono`

---

## 3. Step 1：实现 Markdown 渲染器 (`helpers/markdown.rs`)

### 3.1 对应 Swift 代码

```swift
// Planet/Helper/MarkdownUtils.swift
struct CMarkRenderer {
    static func renderMarkdownHTML(markdown: String) -> String? {
        cmark_gfm_core_extensions_ensure_registered()
        guard let parser = cmark_parser_new(CMARK_OPT_FOOTNOTES) else { return nil }
        // 注册 table, autolink, strikethrough, tasklist 扩展
        for name in ["table", "autolink", "strikethrough", "tasklist"] {
            if let ext = cmark_find_syntax_extension(name) {
                cmark_parser_attach_syntax_extension(parser, ext)
            }
        }
        cmark_parser_feed(parser, inputText, inputText.utf8.count)
        guard let node = cmark_parser_finish(parser) else { return nil }
        guard let htmlBuffer = cmark_render_html(node, CMARK_OPT_UNSAFE | CMARK_OPT_HARDBREAKS, nil) else { return nil }
        return String(validatingUTF8: htmlBuffer)
    }
}
```

### 3.2 Rust 实现

创建文件 `src-tauri/src/helpers/markdown.rs`：

```rust
use pulldown_cmark::{html, Options, Parser};

/// 将 Markdown 文本渲染为 HTML
///
/// 启用 GFM 扩展：表格、删除线、任务列表、脚注
/// 对标 Swift CMarkRenderer.renderMarkdownHTML()
pub fn render_markdown_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    // 预处理：替换 YouTube 链接为 iframe 嵌入
    let processed = replace_youtube_links(markdown);

    let parser = Parser::new_ext(&processed, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// 替换 YouTube 链接为嵌入式 iframe
/// 对标 Swift CMarkRenderer.replaceYouTubeLinks()
fn replace_youtube_links(text: &str) -> String {
    // 简化版：匹配独立行的 YouTube URL
    let re = regex::Regex::new(
        r#"https?://(?:www\.)?youtu(?:be\.com/watch\?v=|\.be/)([\w\-]+)"#
    ).unwrap();

    re.replace_all(text, |caps: &regex::Captures| {
        let video_id = &caps[1];
        format!(
            r#"<iframe width="100%" style="aspect-ratio: 16/9" src="https://www.youtube.com/embed/{}" title="YouTube Video" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>"#,
            video_id
        )
    }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_markdown() {
        let md = "# Hello\n\nThis is **bold** text.";
        let html = render_markdown_html(md);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let html = render_markdown_html(md);
        assert!(html.contains("<table>"));
    }

    #[test]
    fn test_tasklist() {
        let md = "- [x] Done\n- [ ] Todo";
        let html = render_markdown_html(md);
        assert!(html.contains("checked"));
    }

    #[test]
    fn test_youtube_replacement() {
        let text = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        let result = replace_youtube_links(text);
        assert!(result.contains("youtube.com/embed/dQw4w9WgXcQ"));
    }
}
```

### 3.3 添加 `regex` 依赖

在 `Cargo.toml` 中添加：

```toml
regex = "1"
```

### 3.4 更新 `helpers/mod.rs`

```rust
pub mod paths;
pub mod net;
pub mod markdown;  // 新增
```

### 3.5 ✅ Step 1 检查点

完成 Step 1 后，执行以下检查：

```bash
cd src-tauri

# 1. 编译检查
cargo check

# 2. 运行单元测试
cargo test helpers::markdown::tests

# 3. 验证模块导出
cargo check --message-format=short 2>&1 | grep -i "markdown" || echo "✓ markdown 模块编译通过"
```

**预期结果**：
- ✅ `cargo check` 无错误
- ✅ 所有单元测试通过
- ✅ `helpers::markdown` 模块可以正常导入

**如果遇到错误**：
- 检查 `Cargo.toml` 中是否添加了 `regex` 依赖
- 确认 `helpers/mod.rs` 中已添加 `pub mod markdown;`
- 检查 `render_markdown_html` 函数签名是否正确

---

## 4. Step 2：实现模板引擎 (`template/mod.rs`)

### 4.1 对应 Swift 代码

原项目使用 **Stencil** 模板引擎，模板文件位于 `Templates/` 目录下。每个模板包含：
- `template.json` — 元数据（name, description, author, version, settings）
- `templates/index.html` — 首页模板
- `templates/blog.html` — 文章页模板
- `templates/tags.html` — 标签页模板（可选）
- `templates/archive.html` — 归档页模板（可选）
- `assets/` — 静态资源（CSS、JS、图片）

### 4.2 Rust 实现

创建文件 `src-tauri/src/template/mod.rs`：

```rust
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
```

### 4.3 关于 Stencil → Tera 的模板语法差异

原项目使用 Stencil（Django 风格），Tera 也是 Django/Jinja2 风格，语法高度兼容。主要差异：

| Stencil 语法 | Tera 语法 | 说明 |
|---|---|---|
| `{{ variable }}` | `{{ variable }}` | ✅ 相同 |
| `{% for item in items %}` | `{% for item in items %}` | ✅ 相同 |
| `{% if condition %}` | `{% if condition %}` | ✅ 相同 |
| `{{ value\|filter }}` | `{{ value \| filter }}` | ✅ 相同 |
| `{{ value\|filter:arg }}` | `{{ value \| filter(arg=val) }}` | ⚠️ 参数语法不同 |

> **重要**：由于原项目的模板使用 Stencil 的 `filter:arg` 语法，而 Tera 使用 `filter(arg=val)` 语法，可能需要：
> 1. 修改模板文件中的 filter 调用语法，或
> 2. 在渲染前做简单的语法预处理替换
>
> 建议在 Phase 3 中先用内置模板（我们自己提供 Tera 语法的模板），后续 Phase 中再处理兼容性。

### 4.4 内置默认模板

在 `src-tauri/resources/templates/Plain/` 目录下创建一个简单的默认模板：

**`template.json`**:

```json
{
  "name": "Plain",
  "description": "A simple plain template",
  "author": "Planet",
  "version": "1.0.0",
  "idealItemsPerPage": 10,
  "generateIndexPagination": false,
  "generateTagPages": false,
  "generateArchive": false,
  "buildNumber": 1
}
```

**`templates/index.html`**:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{ planet.name }}</title>
    <link rel="stylesheet" href="{{ assets_prefix }}assets/style.css?v={{ style_css_sha256 }}">
    <link rel="alternate" type="application/rss+xml" title="{{ planet.name }}" href="rss.xml">
</head>
<body>
    <header>
        <h1>{{ planet.name }}</h1>
        <p>{{ page_description_html | safe }}</p>
    </header>
    <main>
        {% for article in articles %}
        <article>
            <h2><a href="{{ article.link }}">{{ article.title }}</a></h2>
            <time>{{ article.created }}</time>
            {% if article.contentRendered %}
            <div class="summary">{{ article.contentRendered | safe | truncate(length=200) }}</div>
            {% endif %}
        </article>
        {% endfor %}
    </main>
    {% if total_pages is defined and total_pages > 1 %}
    <nav class="pagination">
        {% if previous_page %}<a href="{{ previous_page }}">← Previous</a>{% endif %}
        <span>Page {{ current_page }} of {{ total_pages }}</span>
        {% if next_page %}<a href="{{ next_page }}">Next →</a>{% endif %}
    </nav>
    {% endif %}
</body>
</html>
```

**`templates/blog.html`**:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{ article_title }} - {{ planet.name }}</title>
    <link rel="stylesheet" href="{{ assets_prefix }}assets/style.css?v={{ style_css_sha256 }}">
</head>
<body>
    <header>
        <a href="{{ assets_prefix }}">← {{ planet.name }}</a>
    </header>
    <main>
        <article>
            <h1>{{ article_title }}</h1>
            <time>{{ article.created }}</time>
            <div class="content">
                {{ content_html | safe }}
            </div>
        </article>
    </main>
</body>
</html>
```

**`assets/style.css`**:

```css
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; max-width: 720px; margin: 0 auto; padding: 2rem 1rem; color: #333; line-height: 1.6; }
header { margin-bottom: 2rem; border-bottom: 1px solid #eee; padding-bottom: 1rem; }
header h1 { font-size: 1.5rem; }
article { margin-bottom: 2rem; }
article h2 { font-size: 1.2rem; }
article h2 a { color: #333; text-decoration: none; }
article h2 a:hover { color: #0066cc; }
time { color: #999; font-size: 0.85rem; }
.content img { max-width: 100%; height: auto; }
.pagination { display: flex; justify-content: space-between; margin-top: 2rem; padding-top: 1rem; border-top: 1px solid #eee; }
```

### 4.5 ✅ Step 2 检查点

完成 Step 2 后，执行以下检查：

```bash
cd src-tauri

# 1. 编译检查
cargo check

# 2. 验证模板模块可以编译
cargo check --lib 2>&1 | grep -i "template\|tera" || echo "✓ template 模块编译通过"

# 3. 检查模板文件是否存在
ls -la src-tauri/src/template/mod.rs && echo "✓ template/mod.rs 存在"
ls -la src-tauri/resources/templates/Plain/template.json && echo "✓ 默认模板存在"
ls -la src-tauri/resources/templates/Plain/templates/index.html && echo "✓ index.html 模板存在"
ls -la src-tauri/resources/templates/Plain/templates/blog.html && echo "✓ blog.html 模板存在"
ls -la src-tauri/resources/templates/Plain/assets/style.css && echo "✓ style.css 存在"
```

**预期结果**：
- ✅ `cargo check` 无错误
- ✅ 所有模板文件存在
- ✅ `template::Template` 和 `template::TemplateStore` 可以正常使用

**如果遇到错误**：
- 检查 `Cargo.toml` 中是否添加了 `tera`、`sha2`、`hex` 依赖
- 确认 `main.rs` 中已添加 `mod template;`
- 检查模板文件路径是否正确
- 验证 `register_filters` 函数是否已导出为 `pub`

---

## 5. Step 3：实现 Keystore 密钥管理 (`keystore/mod.rs`)

### 5.1 对应 Swift 代码

```swift
// Planet/Helper/KeychainHelper.swift
class KeychainHelper {
    func saveData(_ data: Data, forKey key: String) throws { ... }
    func loadData(forKey key: String) throws -> Data { ... }
    func check(forKey key: String) -> Bool { ... }
    func delete(forKey key: String) throws { ... }
    func exportKeyToKeychain(forPlanetKeyName keyName: String) throws { ... }
    func importKeyFromKeychain(forPlanetKeyName keyName: String) throws { ... }
}
```

原项目在创建 Planet 时：
1. `IPFSDaemon.generateKey(name: uuid)` → 在 IPFS 内部生成密钥
2. `KeychainHelper.exportKeyToKeychain(forPlanetKeyName: uuid)` → 导出到 macOS Keychain 备份

发布时如果 IPFS 密钥丢失：
1. `KeychainHelper.importKeyFromKeychain(forPlanetKeyName: uuid)` → 从 Keychain 恢复到 IPFS

### 5.2 Rust 实现

创建文件 `src-tauri/src/keystore/mod.rs`：

```rust
use anyhow::{Context, Result};
use keyring::Entry;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

use crate::helpers::paths;
use crate::ipfs::daemon::IpfsDaemon;

/// Keystore 服务名称（对标 Swift 的 appServiceName）
const SERVICE_NAME: &str = "xyz.planetable.Planet";

/// 密钥存储管理器
///
/// 使用 `keyring-rs` 实现跨平台密钥安全存储：
/// - Windows: Credential Manager
/// - macOS: Keychain
/// - Linux: Secret Service (GNOME Keyring / KDE Wallet)
///
/// 对标 Swift KeychainHelper
pub struct Keystore;

impl Keystore {
    // ============================================================
    // 基础操作: save / load / check / delete
    // ============================================================

    /// 保存数据到安全存储
    ///
    /// 对标 Swift KeychainHelper.saveData(_:forKey:)
    pub fn save_data(key: &str, data: &[u8]) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| anyhow::anyhow!("创建 keyring entry 失败: {}", e))?;
        // keyring-rs 2.x 使用 set_secret (bytes)
        entry.set_secret(data)
            .map_err(|e| anyhow::anyhow!("保存密钥数据失败: {}", e))?;
        debug!("Keystore: 已保存 key={}", key);
        Ok(())
    }

    /// 从安全存储加载数据
    ///
    /// 对标 Swift KeychainHelper.loadData(forKey:)
    pub fn load_data(key: &str) -> Result<Vec<u8>> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| anyhow::anyhow!("创建 keyring entry 失败: {}", e))?;
        let secret = entry.get_secret()
            .map_err(|e| anyhow::anyhow!("加载密钥数据失败: {}", e))?;
        debug!("Keystore: 已加载 key={}, {} bytes", key, secret.len());
        Ok(secret)
    }

    /// 检查密钥是否存在
    ///
    /// 对标 Swift KeychainHelper.check(forKey:)
    pub fn check(key: &str) -> bool {
        match Entry::new(SERVICE_NAME, key) {
            Ok(entry) => entry.get_secret().is_ok(),
            Err(_) => false,
        }
    }

    /// 删除密钥
    ///
    /// 对标 Swift KeychainHelper.delete(forKey:)
    pub fn delete(key: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| anyhow::anyhow!("创建 keyring entry 失败: {}", e))?;
        entry.delete_credential()
            .map_err(|e| anyhow::anyhow!("删除密钥失败: {}", e))?;
        info!("Keystore: 已删除 key={}", key);
        Ok(())
    }

    // ============================================================
    // IPFS 密钥操作
    // ============================================================

    /// 将 IPFS 密钥导出到安全存储 (备份)
    ///
    /// 流程：ipfs key export → 临时文件 → 读取 → 保存到 Keystore → 删除临时文件
    ///
    /// 对标 Swift KeychainHelper.exportKeyToKeychain(forPlanetKeyName:)
    pub fn export_key_to_keystore(daemon: &IpfsDaemon, key_name: &str, app: &tauri::AppHandle) -> Result<()> {
        let tmp_dir = crate::helpers::paths::get_temp_path(app);
        let tmp_key_path = tmp_dir.join(format!("{}.pem", key_name));

        // 确保临时目录存在
        std::fs::create_dir_all(&tmp_dir)?;

        // 如果临时文件已存在，先删除
        if tmp_key_path.exists() {
            std::fs::remove_file(&tmp_key_path)?;
        }

        // 从 IPFS 导出密钥到临时文件（需要将 PathBuf 转换为 &str）
        daemon.export_key(key_name, tmp_key_path.to_str().unwrap(), Some("pem-pkcs8-cleartext"))?;

        // 读取密钥数据
        let key_data = std::fs::read(&tmp_key_path)
            .with_context(|| format!("读取导出的密钥文件失败: {:?}", tmp_key_path))?;

        // 保存到安全存储
        Self::save_data(key_name, &key_data)?;

        // 清理临时文件
        let _ = std::fs::remove_file(&tmp_key_path);

        info!("Keystore: 已将 IPFS key '{}' 导出到安全存储", key_name);
        Ok(())
    }

    /// 从安全存储恢复 IPFS 密钥
    ///
    /// 流程：从 Keystore 加载 → 写入临时文件 → ipfs key import → 删除临时文件
    ///
    /// 对标 Swift KeychainHelper.importKeyFromKeychain(forPlanetKeyName:)
    pub fn import_key_from_keystore(daemon: &IpfsDaemon, key_name: &str, app: &tauri::AppHandle) -> Result<()> {
        if !Self::check(key_name) {
            anyhow::bail!("Keystore 中不存在密钥: {}", key_name);
        }

        let key_data = Self::load_data(key_name)?;
        let tmp_dir = crate::helpers::paths::get_temp_path(app);
        let tmp_key_path = tmp_dir.join(format!("{}.pem", key_name));

        std::fs::create_dir_all(&tmp_dir)?;

        if tmp_key_path.exists() {
            std::fs::remove_file(&tmp_key_path)?;
        }

        // 写入临时文件
        std::fs::write(&tmp_key_path, &key_data)
            .with_context(|| format!("写入临时密钥文件失败: {:?}", tmp_key_path))?;

        // 导入到 IPFS（需要将 PathBuf 转换为 &str）
        let result = daemon.import_key(key_name, tmp_key_path.to_str().unwrap(), Some("pem-pkcs8-cleartext"));

        // 清理临时文件
        let _ = std::fs::remove_file(&tmp_key_path);

        result?;
        info!("Keystore: 已从安全存储恢复 IPFS key '{}'", key_name);
        Ok(())
    }

    /// 将外部密钥文件导入到 IPFS 和 Keystore
    ///
    /// 对标 Swift KeychainHelper.importKeyFile(forPlanetKeyName:fileURL:)
    pub fn import_key_file(daemon: &IpfsDaemon, key_name: &str, file_path: &Path) -> Result<()> {
        // 读取密钥文件
        let key_data = std::fs::read(file_path)
            .with_context(|| format!("读取密钥文件失败: {:?}", file_path))?;

        // 导入到 IPFS（需要将 Path 转换为 &str）
        daemon.import_key(key_name, file_path.to_str().unwrap(), Some("pem-pkcs8-cleartext"))?;

        // 保存到 Keystore
        Self::save_data(key_name, &key_data)?;

        info!("Keystore: 已导入外部密钥 '{}' 到 IPFS 和安全存储", key_name);
        Ok(())
    }

    /// 将 IPFS 密钥导出到文件 (用于备份/分享)
    ///
    /// 对标 Swift KeychainHelper.exportKeyFile(forPlanetName:planetKeyName:toDirectory:)
    pub fn export_key_file(
        daemon: &IpfsDaemon,
        planet_name: &str,
        key_name: &str,
        target_dir: &Path,
        app: &tauri::AppHandle,
    ) -> Result<PathBuf> {
        let safe_name = planet_name.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
        let target_path = target_dir.join(format!("{}.pem", safe_name));

        if target_path.exists() {
            anyhow::bail!("目标文件已存在: {:?}", target_path);
        }

        let tmp_dir = crate::helpers::paths::get_temp_path(app);
        let tmp_key_path = tmp_dir.join(format!("{}.pem", safe_name));

        std::fs::create_dir_all(&tmp_dir)?;
        if tmp_key_path.exists() {
            std::fs::remove_file(&tmp_key_path)?;
        }

        // 从 IPFS 导出到临时文件（需要将 PathBuf 转换为 &str）
        daemon.export_key(key_name, tmp_key_path.to_str().unwrap(), Some("pem-pkcs8-cleartext"))?;

        // 复制到目标路径
        std::fs::copy(&tmp_key_path, &target_path)?;

        // 清理临时文件
        let _ = std::fs::remove_file(&tmp_key_path);

        info!("Keystore: 已导出密钥到 {:?}", target_path);
        Ok(target_path)
    }
}
```

### 5.3 IpfsDaemon 方法说明

**注意**：`export_key` 和 `import_key` 方法在 Phase 1 的 `ipfs/daemon.rs` 中已实现，签名如下：

```rust
// 在 ipfs/daemon.rs 中已存在的方法

/// 导出 IPFS 密钥到文件
/// 签名：pub fn export_key(&self, name: &str, target: &str, format: Option<&str>) -> Result<()>
/// 使用示例：daemon.export_key(key_name, tmp_path.to_str().unwrap(), Some("pem-pkcs8-cleartext"))?;

/// 导入 IPFS 密钥从文件
/// 签名：pub fn import_key(&self, name: &str, target: &str, format: Option<&str>) -> Result<String>
/// 使用示例：daemon.import_key(key_name, tmp_path.to_str().unwrap(), Some("pem-pkcs8-cleartext"))?;
```

**重要**：这两个方法接受 `&str` 类型的路径参数，而不是 `&Path`。在使用时需要将 `PathBuf` 转换为字符串。

### 5.4 ✅ Step 3 检查点

完成 Step 3 后，执行以下检查：

```bash
cd src-tauri

# 1. 编译检查
cargo check

# 2. 验证 keystore 模块可以编译
cargo check --lib 2>&1 | grep -i "keystore\|keyring" || echo "✓ keystore 模块编译通过"

# 3. 检查模块文件是否存在
ls -la src-tauri/src/keystore/mod.rs && echo "✓ keystore/mod.rs 存在"
```

**预期结果**：
- ✅ `cargo check` 无错误
- ✅ `keystore::Keystore` 可以正常使用
- ✅ 所有方法签名正确（注意 `app: &AppHandle` 参数）

**如果遇到错误**：
- 检查 `Cargo.toml` 中是否添加了 `keyring` 依赖
- 确认 `main.rs` 中已添加 `mod keystore;`
- 验证 `export_key_to_keystore` 和 `import_key_from_keystore` 方法签名是否正确
- 检查路径转换是否正确（`PathBuf` → `&str`）

---

## 6. Step 4：实现 savePublic — 文章静态化 (`models/article.rs` 扩展)

### 6.1 对应 Swift 代码

```swift
// MyArticleModel+Save.swift
extension MyArticleModel {
    func savePublic(usingTasks: Bool = false) throws {
        removeDSStore()
        // 创建 public 目录
        try FileManager.default.createDirectory(at: publicBasePath, ...)
        // Markdown → HTML
        try processContent()
        // 渲染文章页面 (blog.html 模板)
        try processArticleHTML(usingTasks: usingTasks)
        // 写入 article.json
        try JSONEncoder.shared.encode(publicArticle).write(to: publicInfoPath)
    }
}
```

### 6.2 Rust 实现

在 `src-tauri/src/models/article.rs` 中添加以下方法：

```rust
use crate::helpers::markdown::render_markdown_html;
use crate::template::Template;
use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use tracing::debug;

// ============================================================
// PublicArticle — 用于序列化到 article.json 和模板渲染
// ============================================================

/// 文章的公开数据（写入 article.json，也传给模板渲染）
///
/// 对标 Swift PublicArticleModel
/// 
/// **注意**：实际代码中 `PublicArticle` 结构体已存在，字段略有不同。
/// 实际实现中 `id` 为 `Uuid` 类型，`slug` 为 `String` 类型（非 Option）。
/// 以下为文档示例，实际使用时请参考 `src-tauri/src/models/article.rs` 中的实现。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicArticle {
    pub id: Uuid,  // 注意：实际代码中使用 Uuid，不是 String
    pub link: String,
    pub slug: String,  // 注意：实际代码中 slug 不是 Option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_link: Option<String>,
    pub title: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_rendered: Option<String>,
    pub created: String,        // ISO 8601
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_video: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_audio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_byte_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hero_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hero_image_width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hero_image_height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
}

impl MyArticle {  // 注意：实际代码中使用 MyArticle，不是 Article
    /// 生成公开文章数据
    ///
    /// 对标 Swift MyArticleModel.publicArticle 计算属性
    /// 
    /// **注意**：实际代码中已有 `From<&MyArticle> for PublicArticle` 实现，
    /// 可以直接使用 `PublicArticle::from(article)`。
    pub fn to_public(&self) -> PublicArticle {
        PublicArticle {
            id: self.id.clone(),
            link: if let Some(ref slug) = self.slug {
                if !slug.is_empty() {
                    format!("/{}/", slug)
                } else {
                    self.link.clone()
                }
            } else {
                self.link.clone()
            },
            slug: self.slug.clone(),
            external_link: self.external_link.clone(),
            title: self.title.clone(),
            content: self.content.clone(),
            content_rendered: self.content_rendered.clone(),
            created: self.created.clone(),
            has_video: self.video_filename.as_ref().map(|_| true),
            video_filename: self.video_filename.clone(),
            has_audio: self.audio_filename.as_ref().map(|_| true),
            audio_filename: self.audio_filename.clone(),
            audio_duration: None, // 简化：Phase 3 不处理音频时长
            audio_byte_length: None,
            attachments: self.attachments.clone(),
            hero_image: None,
            hero_image_width: None,
            hero_image_height: None,
            tags: self.tags.clone(),
        }
    }

    /// 文章的 public 目录路径
    pub fn public_base_path(&self, planet_public_path: &PathBuf) -> PathBuf {
        planet_public_path.join(&self.id)
    }

    /// 将文章渲染为静态 HTML 文件
    ///
    /// 对标 Swift MyArticleModel.savePublic()
    /// 
    /// **注意**：实际实现需要传入 `app: &AppHandle` 参数用于路径获取。
    pub fn save_public(
        &mut self,
        planet: &super::planet::MyPlanet,  // 注意：使用 MyPlanet，不是 Planet
        template: &Template,
        app: &tauri::AppHandle,  // 需要 app 参数
    ) -> Result<()> {
        let planet_public_path = planet.public_base_path(app);  // 需要传入 app
        let public_base = planet_public_path.join(self.id.to_string());  // id 是 Uuid，需要 to_string()

        // 1. 创建 public 目录
        fs::create_dir_all(&public_base)?;

        // 2. Markdown → HTML (如果尚未渲染)
        // 注意：MyArticle 结构体中没有 content_rendered 字段，需要在渲染时计算
        let content_html = if !self.content.is_empty() {
            render_markdown_html(&self.content)
        } else {
            String::new()
        };

        let content_html = self.content_rendered.clone().unwrap_or_default();

        // 3. 用模板渲染文章页面
        let mut context = tera::Context::new();
        let public_article = self.to_public();

        // 构建模板上下文（对标 Swift Template.render(article:) 的 context）
        let public_planet = PublicPlanet::from(planet);
        context.insert("planet", &public_planet);
        context.insert("planet_ipns", &planet.ipns);
        context.insert("assets_prefix", "../");
        context.insert("article_id", &self.id.to_string());  // Uuid 需要转换为 String
        context.insert("article", &public_article);
        context.insert("article_title", &self.title);
        context.insert("page_title", &self.title);
        context.insert("content_html", &content_html);
        context.insert("style_css_sha256", &template.style_css_hash().unwrap_or_default());
        context.insert("build_timestamp", &chrono::Utc::now().timestamp());

        // 渲染 blog.html → index.html
        let article_html = template.render_article(&context)?;
        let index_path = public_base.join("index.html");
        fs::write(&index_path, article_html.as_bytes())?;

        debug!("文章 '{}' 静态化完成: {:?}", self.title, index_path);

        // 4. 写入 article.json
        let info_path = public_base.join("article.json");
        let info_json = serde_json::to_string_pretty(&public_article)?;
        fs::write(&info_path, info_json.as_bytes())?;

        // 5. 保存 article.md (原始 Markdown)
        let md_path = public_base.join("article.md");
        let md_content = format!("{}\n\n{}", self.title, self.content);
        fs::write(&md_path, md_content.as_bytes())?;

        Ok(())
    }
}
```

### 6.3 ✅ Step 4 检查点

完成 Step 4 后，执行以下检查：

```bash
cd src-tauri

# 1. 编译检查
cargo check

# 2. 验证 article 模块可以编译
cargo check --lib 2>&1 | grep -i "article\|PublicArticle" || echo "✓ article 模块编译通过"

# 3. 检查方法签名
# 注意：如果 MyArticle 结构体中还没有 save_public 方法，这一步可能会失败
# 这是正常的，可以暂时跳过，等实现完成后再检查
```

**预期结果**：
- ✅ `cargo check` 无错误（或只有预期的未实现方法错误）
- ✅ `PublicArticle` 结构体可以正常序列化/反序列化
- ✅ `MyArticle::save_public` 方法签名正确

**如果遇到错误**：
- 检查 `PublicArticle` 结构体字段类型是否正确（`Uuid` vs `String`）
- 确认 `save_public` 方法签名包含 `app: &AppHandle` 参数
- 验证 `render_markdown_html` 调用是否正确
- 检查模板渲染上下文构建是否正确

---

## 7. Step 5：实现 savePublic — Planet 站点生成 (`models/planet.rs` 扩展)

### 7.1 对应 Swift 代码

```swift
// MyPlanetModel.swift
func savePublic() async throws {
    // 1. 构建 PublicPlanetModel
    // 2. 渲染 RSS
    // 3. 渲染 index.html (带分页)
    // 4. 渲染 tags 页面 (可选)
    // 5. 渲染 archive 页面 (可选)
    // 6. 写入 planet.json
    // 7. 复制模板 assets
}
```

### 7.2 PublicPlanet 结构体

在 `models/planet.rs` 中添加：

```rust
use std::collections::HashMap;

/// Planet 的公开数据（写入 planet.json，也传给模板渲染）
///
/// 对标 Swift PublicPlanetModel
/// 
/// **注意**：实际代码中 `PublicPlanet` 结构体已存在，字段略有不同。
/// 实际实现中 `id` 为 `Uuid` 类型，`created` 和 `updated` 为 `DateTime<Utc>` 类型。
/// 以下为文档示例，实际使用时请参考 `src-tauri/src/models/planet.rs` 中的实现。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicPlanet {
    pub id: Uuid,  // 注意：实际代码中使用 Uuid，不是 String
    pub name: String,
    pub about: String,
    pub created: DateTime<Utc>,  // 注意：实际代码中使用 DateTime，不是 String
    pub updated: DateTime<Utc>,
    // 注意：实际代码中可能没有 ipns 和 articles 字段，这些在模板渲染时单独传入
}

impl MyPlanet {  // 注意：实际代码中使用 MyPlanet，不是 Planet
    /// 生成公开 Planet 数据
    /// 
    /// **注意**：实际代码中已有 `From<&MyPlanet> for PublicPlanet` 实现，
    /// 可以直接使用 `PublicPlanet::from(planet)`。
    pub fn to_public(&self, articles: &[super::article::PublicArticle]) -> PublicPlanet {
        PublicPlanet {
            id: self.id.clone(),
            name: self.name.clone(),
            about: self.about.clone(),
            ipns: self.ipns.clone(),
            created: self.created.clone(),
            updated: self.updated.clone(),
            articles: articles.to_vec(),
            tags: self.tags.clone(),
        }
    }

    /// 生成 serde_json::Value 版本 (用于模板渲染 context)
    pub fn to_public_value(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "about": self.about,
            "ipns": self.ipns,
            "created": self.created,
            "updated": self.updated,
        })
    }

    /// Planet 的 public 根目录
    /// 
    /// **注意**：实际实现需要传入 `app: &AppHandle` 参数。
    pub fn public_base_path(&self, app: &tauri::AppHandle) -> PathBuf {
        let public_path = crate::helpers::paths::get_data_path(app).join("Public");
        std::fs::create_dir_all(&public_path).ok();
        public_path.join(self.id.to_string())  // id 是 Uuid，需要 to_string()
    }

    /// 获取所有文章的 public 版本
    fn get_public_articles(&self) -> Vec<super::article::PublicArticle> {
        self.articles.iter().map(|a| a.to_public()).collect()
    }
}
```

### 7.3 RSS 渲染

```rust
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

impl Planet {
    /// 渲染 RSS feed
    ///
    /// 对标 Swift MyPlanetModel.renderRSS(podcastOnly:)
    fn render_rss(&self, articles: &[super::article::PublicArticle]) -> Result<String> {
        let root_prefix = if let Some(ref domain) = self.domain {
            if !domain.is_empty() {
                format!("https://{}", domain)
            } else {
                format!("https://eth.sucks/ipns/{}", self.ipns)
            }
        } else {
            format!("https://eth.sucks/ipns/{}", self.ipns)
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
}
```

> **注意**：`register_filters` 函数需要从 `template/mod.rs` 中导出为 `pub fn`。

### 7.4 savePublic 完整实现

```rust
impl Planet {
    /// 生成静态站点到 public 目录
    ///
    /// 对标 Swift MyPlanetModel.savePublic()
    /// 
    /// **注意**：实际实现需要传入 `app: &AppHandle` 参数。
    pub fn save_public(&mut self, template: &Template, app: &tauri::AppHandle) -> Result<()> {
        let public_base = self.public_base_path(app);
        let public_assets = public_base.join("assets");

        // 确保 public 目录存在
        fs::create_dir_all(&public_base)?;

        // ============================================================
        // 1. 对每篇文章执行 save_public
        // ============================================================
        // 注意：需要先加载所有文章
        let articles = crate::models::article::MyArticle::load_all(self, app)?;
        for mut article in articles {
            if let Err(e) = article.save_public(self, template, app) {
                tracing::error!("文章 '{}' 静态化失败: {}", article.title, e);
            }
        }

        // ============================================================
        // 2. 构建公开文章列表
        // ============================================================
        let articles = crate::models::article::MyArticle::load_all(self, app)?;
        let public_articles: Vec<super::article::PublicArticle> =
            articles.iter().map(|a| PublicArticle::from(a)).collect();

        let public_planet = self.to_public(&public_articles);

        let about_html = render_markdown_html(&self.about);
        let css_hash = template.style_css_hash().unwrap_or_default();

        // ============================================================
        // 3. 渲染 RSS
        // ============================================================
        match self.render_rss(&public_articles) {
            Ok(rss_xml) => {
                let rss_path = public_base.join("rss.xml");
                fs::write(&rss_path, rss_xml.as_bytes())?;
                debug!("RSS 生成完成: {:?}", rss_path);
            }
            Err(e) => tracing::error!("RSS 渲染失败: {}", e),
        }

        // ============================================================
        // 4. 渲染 index.html (首页 + 分页)
        // ============================================================
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
                context.insert("planet_ipns", &self.ipns);
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

                let page_html = template.render_index(&context)?;
                let page_path = public_base.join(format!("page{}.html", page));
                fs::write(&page_path, page_html.as_bytes())?;

                // 第一页同时写入 index.html
                if page == 1 {
                    let index_path = public_base.join("index.html");
                    fs::write(&index_path, page_html.as_bytes())?;
                }
            }
        } else {
            // 不分页，直接渲染单个 index.html
            let mut context = tera::Context::new();
            context.insert("planet", &public_planet);
            context.insert("planet_ipns", &self.ipns);
            context.insert("has_avatar", &self.has_avatar());
            context.insert("page_title", &self.name);
            context.insert("page_description", &self.about);
            context.insert("page_description_html", &about_html);
            context.insert("articles", &public_articles);
            context.insert("assets_prefix", "./");
            context.insert("style_css_sha256", &css_hash);
            context.insert("build_timestamp", &chrono::Utc::now().timestamp());

            let index_html = template.render_index(&context)?;
            let index_path = public_base.join("index.html");
            fs::write(&index_path, index_html.as_bytes())?;

            let page1_path = public_base.join("page1.html");
            fs::write(&page1_path, index_html.as_bytes())?;
        }

        // ============================================================
        // 5. 渲染 tags 页面 (可选)
        // ============================================================
        if template.info.generate_tag_pages && template.has_tags_html() {
            let mut tag_articles: HashMap<String, Vec<&super::article::PublicArticle>> = HashMap::new();

            for article in &public_articles {
                if let Some(ref tags) = article.tags {
                    for key in tags.keys() {
                        tag_articles
                            .entry(key.clone())
                            .or_insert_with(Vec::new)
                            .push(article);
                    }
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
                context.insert("planet_ipns", &self.ipns);
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
            tags_context.insert("has_avatar", &self.has_avatar());
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
                let month_year = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&article.created) {
                    dt.format("%B %Y").to_string()
                } else {
                    "Unknown".to_string()
                };

                if !archive.contains_key(&month_year) {
                    archive_sections.push(month_year.clone());
                }
                archive.entry(month_year).or_insert_with(Vec::new).push(article);
            }

            let mut context = tera::Context::new();
            context.insert("planet", &public_planet);
            context.insert("planet_ipns", &self.ipns);
            context.insert("has_avatar", &self.has_avatar());
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
        fs::write(&info_path, planet_json.as_bytes())?;

        // ============================================================
        // 8. 复制模板 assets
        // ============================================================
        if public_assets.exists() {
            fs::remove_dir_all(&public_assets)?;
        }
        copy_dir_recursive(&template.assets_path(), &public_assets)?;

        // ============================================================
        // 9. 复制 avatar (如果存在)
        // ============================================================
        let avatar_src = self.avatar_path(app);  // 使用已有的 avatar_path 方法
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
        fs::write(&robots_path, robots_txt.as_bytes())?;

        debug!("Planet '{}' 站点生成完成", self.name);
        Ok(())
    }

    /// 检查是否有 avatar
    /// 
    /// **注意**：实际实现需要传入 `app: &AppHandle` 参数。
    fn has_avatar(&self, app: &tauri::AppHandle) -> bool {
        self.avatar_path(app).exists()  // 使用已有的 avatar_path 方法
    }

    /// Planet 的 base 目录
    /// 
    /// **注意**：实际代码中已有 `base_path(&self, app: &AppHandle)` 方法。
    fn base_path(&self, app: &tauri::AppHandle) -> PathBuf {
        self.base_path(app)  // 使用已有的方法
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
```

### 7.5 Planet 模型需要新增的字段

在 Phase 2 的 `Planet` 结构体中需要确保包含以下发布相关字段：

```rust
// 在 models/planet.rs 的 Planet 结构体中，确保包含：

#[serde(skip_serializing_if = "Option::is_none")]
pub domain: Option<String>,

#[serde(skip_serializing_if = "Option::is_none")]
pub last_published: Option<String>,       // ISO 8601

#[serde(skip_serializing_if = "Option::is_none")]
pub last_published_cid: Option<String>,

#[serde(skip_serializing_if = "Option::is_none")]
pub tags: Option<HashMap<String, String>>,

#[serde(skip_serializing_if = "Option::is_none")]
pub do_not_index: Option<bool>,

// Filebase 集成
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

    // 运行时状态 (不持久化)
    // 注意：实际代码中可能没有这些字段，需要在发布时临时管理状态
    // #[serde(skip)]
    // pub is_publishing: bool,
    // #[serde(skip)]
    // pub publish_started_at: Option<DateTime<Utc>>,  // 注意：实际代码中可能使用 DateTime，不是 String
```

### 7.6 ✅ Step 5 检查点

完成 Step 5 后，执行以下检查：

```bash
cd src-tauri

# 1. 编译检查
cargo check

# 2. 验证 planet 模块可以编译
cargo check --lib 2>&1 | grep -i "planet\|PublicPlanet" || echo "✓ planet 模块编译通过"

# 3. 检查关键方法是否存在
# 注意：如果方法还未实现，这一步可能会失败，这是正常的
```

**预期结果**：
- ✅ `cargo check` 无错误（或只有预期的未实现方法错误）
- ✅ `PublicPlanet` 结构体可以正常序列化/反序列化
- ✅ `MyPlanet::save_public` 方法签名正确
- ✅ `MyPlanet::public_base_path` 方法签名正确

**如果遇到错误**：
- 检查 `PublicPlanet` 结构体字段类型是否正确（`Uuid` vs `String`，`DateTime<Utc>` vs `String`）
- 确认所有方法都包含 `app: &AppHandle` 参数
- 验证 `render_rss` 方法是否正确使用 `register_filters`
- 检查 `copy_dir_recursive` 辅助函数是否正确实现
- 确认 `has_avatar` 方法签名包含 `app` 参数

---

## 8. Step 6：实现 Publish 发布到 IPNS (`models/planet.rs` 扩展)

### 8.1 对应 Swift 代码

```swift
// MyPlanetModel.swift
func publish() async throws {
    self.isPublishing = true
    self.publishStartedAt = Date()
    // 1. 检查 IPFS key 是否存在
    if try await !IPFSDaemon.shared.checkKeyExists(name: id.uuidString) {
        try KeychainHelper.shared.importKeyFromKeychain(forPlanetKeyName: id.uuidString)
    }
    // 2. add_directory → CID
    let cid = try await IPFSDaemon.shared.addDirectory(url: publicBasePath)
    // 3. name/publish → IPNS
    let result = try await IPFSDaemon.shared.api(
        path: "name/publish",
        args: ["arg": cid, "allow-offline": "1", "key": id.uuidString, ...]
    )
    // 4. Filebase pin (可选)
    // 5. Pinnable pin (可选)
    self.isPublishing = false
}
```

### 8.2 Rust 实现

```rust
use crate::ipfs::daemon::IpfsDaemon;
use crate::keystore::Keystore;

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

impl Planet {
    /// 完整发布流程
    ///
    /// 对标 Swift MyPlanetModel.publish()
    pub async fn publish(
        &mut self,
        daemon: &IpfsDaemon,
        template: &crate::template::Template,
        app_handle: &tauri::AppHandle,
    ) -> Result<()> {
        // 注意：如果 MyPlanet 结构体中没有 is_publishing 字段，
        // 需要在外部管理发布状态，或使用临时变量
        let publish_started_at = chrono::Utc::now();

        // 通知前端：发布开始
        self.emit_publish_state(app_handle, "saving", None, None, &publish_started_at);

        // ============================================================
        // 1. 检查 IPFS key 是否存在
        // ============================================================
        // 注意：check_key_exists 方法签名需要确认，可能需要传入 &str
        let key_name = self.id.to_string();  // Uuid 转换为 String
        let key_exists = daemon.check_key_exists(&key_name).await?;
        if !key_exists {
            info!("IPFS key '{}' 不存在，尝试从 Keystore 恢复", key_name);
            Keystore::import_key_from_keystore(daemon, &key_name, app_handle)?;
        }

        // ============================================================
        // 2. save_public() — 生成静态站点
        // ============================================================
        self.save_public(template, app_handle)?;

        self.emit_publish_state(app_handle, "uploading", None, None, &publish_started_at);

        // ============================================================
        // 3. add_directory → CID
        // ============================================================
        // 注意：add_directory 是同步方法，不是异步的
        let public_path = self.public_base_path(app_handle);
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
        // 注意：api 方法接受 Option<&HashMap<String, String>>，不是数组
        let mut args = std::collections::HashMap::new();
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
                tracing::error!("IPNS 发布失败: {}", e);
                // IPNS 发布失败不阻塞整体流程，CID 已经可用
            }
        }

        // ============================================================
        // 5. 更新 Planet 状态
        // ============================================================
        if self.last_published_cid.as_deref() != Some(&cid) {
            self.last_published = Some(chrono::Utc::now());
            self.last_published_cid = Some(cid.clone());
            self.save(app_handle)?;  // 使用已有的 save 方法
        }

        // ============================================================
        // 6. Filebase pin (可选, 异步)
        // ============================================================
        if let Some(true) = self.filebase_enabled {
            if let (Some(ref pin_name), Some(ref api_token)) =
                (&self.filebase_pin_name, &self.filebase_api_token)
            {
                let should_pin = match &self.filebase_pin_cid {
                    Some(existing_cid) => existing_cid.is_empty() || existing_cid != &cid,
                    None => true,
                };

                if should_pin {
                    self.emit_publish_state(app_handle, "pinning", Some(&cid), None, &publish_started_at);
                    let filebase = crate::integrations::filebase::Filebase::new(
                        pin_name.clone(),
                        api_token.clone(),
                    );
                    match filebase.pin(&cid).await {
                        Ok(Some(request_id)) => {
                            // 注意：MyPlanet 结构体中可能没有 filebase_request_id 字段
                            // 需要根据实际结构体定义调整
                            self.filebase_pin_cid = Some(cid.clone());
                            self.save(app_handle)?;  // 使用已有的 save 方法
                            info!("Filebase pin 成功");
                        }
                        Ok(None) => {
                            warn!("Filebase pin 未返回 request ID");
                        }
                        Err(e) => {
                            tracing::error!("Filebase pin 失败: {}", e);
                        }
                    }
                }
            }
        }

        // ============================================================
        // 7. Pinnable pin (可选, 异步)
        // ============================================================
        if let Some(true) = self.pinnable_enabled {
            if let Some(ref api_endpoint) = self.pinnable_api_endpoint {
                let pinnable = crate::integrations::pinnable::Pinnable::new(api_endpoint.clone());
                if let Err(e) = pinnable.pin().await {
                    tracing::error!("Pinnable pin 失败: {}", e);
                }
            }
        }

        // ============================================================
        // 8. 完成
        // ============================================================
        self.emit_publish_state(app_handle, "done", Some(&cid), None, &publish_started_at);

        info!("Planet '{}' 发布流程完成", self.name);
        Ok(())
    }

    /// 向前端发送发布状态更新
    fn emit_publish_state(
        &self,
        app_handle: &tauri::AppHandle,
        step: &str,
        cid: Option<&str>,
        error: Option<&str>,
        started_at: &chrono::DateTime<chrono::Utc>,
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
        let _ = app_handle.emit_all("publish-state-changed", &state);
    }
}
```

### 8.3 ✅ Step 6 检查点

完成 Step 6 后，执行以下检查：

```bash
cd src-tauri

# 1. 编译检查
cargo check

# 2. 验证 publish 相关代码可以编译
cargo check --lib 2>&1 | grep -i "publish\|PublishState" || echo "✓ publish 相关代码编译通过"

# 3. 检查关键方法签名
# 注意：如果方法还未实现，这一步可能会失败，这是正常的
```

**预期结果**：
- ✅ `cargo check` 无错误（或只有预期的未实现方法错误）
- ✅ `PublishState` 结构体可以正常序列化/反序列化
- ✅ `MyPlanet::publish` 方法签名正确（async，包含所有必需参数）
- ✅ `emit_publish_state` 方法签名正确

**如果遇到错误**：
- 检查 `PublishState` 结构体字段类型是否正确
- 确认 `publish` 方法是 `async` 的
- 验证 `add_directory` 调用是否正确（同步方法，不是异步）
- 检查 `api` 方法调用参数类型是否正确（`HashMap` 而不是数组）
- 确认所有 ID 类型转换正确（`Uuid` → `String`）
- 验证 `Keystore::import_key_from_keystore` 调用参数是否正确

---

## 9. Step 7：实现 Filebase 集成 (`integrations/filebase.rs`)

### 9.1 对应 Swift 代码

```swift
// Planet/Integrations/Filebase.swift
struct Filebase: Codable {
    var pinName: String
    var apiToken: String

    func pin(cid: String) async -> String? {
        // GET /v1/ipfs/pins → 查找已有的 requestID
        // POST /v1/ipfs/pins → 提交新的 pin 请求
    }

    func checkPinStatus(requestID: String) async -> (pin: FilebasePin?, message: String?) {
        // GET /v1/ipfs/pins/{requestID}
    }
}
```

### 9.2 Rust 实现

创建目录和文件 `src-tauri/src/integrations/mod.rs` 和 `src-tauri/src/integrations/filebase.rs`：

**`integrations/mod.rs`**:

```rust
pub mod filebase;
pub mod pinnable;
```

**`integrations/filebase.rs`**:

```rust
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// Filebase pinning 服务
///
/// API 文档: https://docs.filebase.com/api-documentation/ipfs-pinning-service-api
///
/// 对标 Swift Filebase
pub struct Filebase {
    pin_name: String,
    api_token: String,
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilebasePin {
    pub cid: String,
    pub request_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
struct PinListResponse {
    results: Option<Vec<PinListResult>>,
}

#[derive(Debug, Deserialize)]
struct PinListResult {
    requestid: Option<String>,
    pin: Option<PinInfo>,
}

#[derive(Debug, Deserialize)]
struct PinInfo {
    name: Option<String>,
    cid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PinResponse {
    requestid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PinStatusResponse {
    status: Option<String>,
    pin: Option<PinInfo>,
    error: Option<ErrorInfo>,
}

#[derive(Debug, Deserialize)]
struct ErrorInfo {
    reason: Option<String>,
}

impl Filebase {
    pub fn new(pin_name: String, api_token: String) -> Self {
        Filebase {
            pin_name,
            api_token,
            client: Client::new(),
        }
    }

    /// 提交 CID 进行 pinning
    ///
    /// 1. 先查询是否已有同名的 pin
    /// 2. 如果有，更新；如果没有，创建新的
    ///
    /// 返回 requestID
    ///
    /// 对标 Swift Filebase.pin(cid:)
    pub async fn pin(&self, cid: &str) -> Result<Option<String>> {
        let base_url = "https://api.filebase.io/v1/ipfs/pins";

        // Step 1: 查找已有的 request ID
        let existing_request_id = self.find_existing_request_id().await;

        // Step 2: 提交 pin 请求
        let url = if let Some(ref _request_id) = existing_request_id {
            // 已有 pin，使用 POST 创建新的（Filebase 会自动去重）
            base_url.to_string()
        } else {
            base_url.to_string()
        };

        let body = serde_json::json!({
            "name": self.pin_name,
            "cid": cid,
        });

        let resp = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!("Filebase pin 请求失败: {} {}", status, body);
            return Ok(None);
        }

        let pin_resp: PinResponse = resp.json().await?;
        if let Some(request_id) = pin_resp.requestid {
            info!("Filebase: pin 成功，requestID={}", request_id);
            return Ok(Some(request_id));
        }

        Ok(None)
    }

    /// 查找已有的同名 pin 的 request ID
    async fn find_existing_request_id(&self) -> Option<String> {
        let url = "https://api.filebase.io/v1/ipfs/pins";

        let resp = self.client
            .get(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .ok()?;

        let list: PinListResponse = resp.json().await.ok()?;
        if let Some(results) = list.results {
            for result in results {
                if let Some(ref pin) = result.pin {
                    if pin.name.as_deref() == Some(&self.pin_name) {
                        debug!("Filebase: 找到已有的 requestID for '{}'", self.pin_name);
                        return result.requestid;
                    }
                }
            }
        }

        debug!("Filebase: 未找到 '{}' 的 requestID", self.pin_name);
        None
    }

    /// 检查 pin 状态
    ///
    /// 对标 Swift Filebase.checkPinStatus(requestID:)
    pub async fn check_pin_status(&self, request_id: &str) -> Result<Option<FilebasePin>> {
        let url = format!("https://api.filebase.io/v1/ipfs/pins/{}", request_id);

        let resp = self.client
            .get(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?;

        let status_resp: PinStatusResponse = resp.json().await?;

        if let Some(ref error) = status_resp.error {
            if let Some(ref reason) = error.reason {
                warn!("Filebase check status 错误: {}", reason);
                return Ok(None);
            }
        }

        if let (Some(status), Some(pin)) = (status_resp.status, status_resp.pin) {
            if let Some(cid) = pin.cid {
                return Ok(Some(FilebasePin {
                    cid,
                    request_id: request_id.to_string(),
                    status,
                }));
            }
        }

        Ok(None)
    }
}
```

### 9.3 ✅ Step 7 检查点

完成 Step 7 后，执行以下检查：

```bash
cd src-tauri

# 1. 编译检查
cargo check

# 2. 验证 integrations 模块可以编译
cargo check --lib 2>&1 | grep -i "integrations\|filebase" || echo "✓ integrations/filebase 模块编译通过"

# 3. 检查模块文件是否存在
ls -la src-tauri/src/integrations/mod.rs && echo "✓ integrations/mod.rs 存在"
ls -la src-tauri/src/integrations/filebase.rs && echo "✓ integrations/filebase.rs 存在"
```

**预期结果**：
- ✅ `cargo check` 无错误
- ✅ `integrations::filebase::Filebase` 可以正常使用
- ✅ 所有方法签名正确（async 方法）

**如果遇到错误**：
- 检查 `main.rs` 中是否添加了 `mod integrations;`
- 确认 `integrations/mod.rs` 中已导出 `pub mod filebase;`
- 验证所有 async 方法签名正确
- 检查 `reqwest::Client` 使用是否正确

---

## 10. Step 8：实现 Pinnable 集成 (`integrations/pinnable.rs`)

### 10.1 对应 Swift 代码

```swift
// Planet/Integrations/Pinnable.swift
struct Pinnable {
    var api: String
    func pin() async { ... }           // GET /pin/:uuid, expect 202
    func status() async -> PinnablePinStatus? { ... }  // GET /pin/:uuid/status
}
```

### 10.2 Rust 实现

创建文件 `src-tauri/src/integrations/pinnable.rs`：

```rust
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// Pinnable.xyz pinning 服务
///
/// 对标 Swift Pinnable
pub struct Pinnable {
    api: String,
    client: Client,
}

/// Pin 状态
///
/// 对标 Swift PinnablePinStatus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnablePinStatus {
    pub status: String,
    pub last_known_ipns: Option<String>,
    pub last_known_cid: Option<String>,
    pub size: Option<i64>,
    pub created: Option<i64>,
    pub last_checked: Option<i64>,
    pub last_pinned: Option<i64>,
}

impl Pinnable {
    pub fn new(api: String) -> Self {
        Pinnable {
            api,
            client: Client::new(),
        }
    }

    /// 发送 pin 请求
    ///
    /// 对标 Swift Pinnable.pin()
    pub async fn pin(&self) -> Result<()> {
        let resp = self.client
            .get(&self.api)
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 202 {
            info!("Pinnable: pin 请求已接受 (202)");
            Ok(())
        } else {
            warn!("Pinnable: 意外的状态码 {}", status);
            anyhow::bail!("Pinnable pin 失败，状态码: {}", status);
        }
    }

    /// 查询 pin 状态
    ///
    /// 对标 Swift Pinnable.status()
    pub async fn status(&self) -> Result<Option<PinnablePinStatus>> {
        let url = format!("{}/status", self.api);
        debug!("Pinnable: 查询状态 {}", url);

        let resp = self.client
            .get(&url)
            .send()
            .await?;

        if !resp.status().is_success() {
            warn!("Pinnable: 查询状态失败 {}", resp.status());
            return Ok(None);
        }

        let status: PinnablePinStatus = resp.json().await?;
        Ok(Some(status))
    }
}
```

### 10.3 ✅ Step 8 检查点

完成 Step 8 后，执行以下检查：

```bash
cd src-tauri

# 1. 编译检查
cargo check

# 2. 验证 integrations 模块可以编译
cargo check --lib 2>&1 | grep -i "integrations\|pinnable" || echo "✓ integrations/pinnable 模块编译通过"

# 3. 检查模块文件是否存在
ls -la src-tauri/src/integrations/pinnable.rs && echo "✓ integrations/pinnable.rs 存在"
```

**预期结果**：
- ✅ `cargo check` 无错误
- ✅ `integrations::pinnable::Pinnable` 可以正常使用
- ✅ 所有方法签名正确（async 方法）

**如果遇到错误**：
- 确认 `integrations/mod.rs` 中已导出 `pub mod pinnable;`
- 验证所有 async 方法签名正确
- 检查 `PinnablePinStatus` 结构体字段类型是否正确

---

## 11. Step 9：注册 Tauri Commands (`commands/planet.rs` 扩展)

### 11.1 新增发布相关命令

在 `src-tauri/src/commands/planet.rs` 中添加：

```rust
use crate::integrations::filebase::Filebase;
use crate::integrations::pinnable::PinnablePinStatus;
use crate::models::planet::PublishState;

/// 触发发布
///
/// 对标 Swift MyPlanetModel.publish()
#[tauri::command]
pub async fn planet_publish(
    planet_id: String,
    app_handle: tauri::AppHandle,
    store: tauri::State<'_, crate::store::PlanetStoreHandle>,
    ipfs_state: tauri::State<'_, crate::ipfs::state::IpfsStateHandle>,
) -> Result<(), String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;

    let planet = store.my_planets
        .iter_mut()
        .find(|p| p.id == planet_id)
        .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;

    // 获取 daemon 引用
    let ipfs = ipfs_state.lock().map_err(|e| e.to_string())?;
    let daemon = ipfs.daemon.as_ref()
        .ok_or_else(|| "IPFS daemon 未启动".to_string())?;

    // 获取模板
    // 注意：需要确认模板路径的实际获取方法
    let templates_path = crate::helpers::paths::get_data_path(&app_handle).join("Templates");
    let template_store = crate::template::TemplateStore::load(&templates_path)
        .map_err(|e| e.to_string())?;

    let template = template_store.get(&planet.template_name)
        .ok_or_else(|| format!("模板 '{}' 不存在", planet.template_name))?;

    // 执行发布
    planet.publish(daemon, template, &app_handle)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 查询发布状态
#[tauri::command]
pub fn planet_get_publish_state(
    planet_id: String,
    store: tauri::State<'_, crate::store::PlanetStoreHandle>,
) -> Result<PublishState, String> {
    let store = store.lock().map_err(|e| e.to_string())?;

    // 注意：planet_id 是 String，需要转换为 Uuid
    let planet_uuid = uuid::Uuid::parse_str(&planet_id)
        .map_err(|_| format!("无效的 Planet ID: {}", planet_id))?;
    
    let planet = store.my_planets
        .iter()
        .find(|p| p.id == planet_uuid)
        .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;

    // 注意：MyPlanet 结构体中可能没有 is_publishing 和 publish_started_at 字段
    // 需要根据实际结构体定义调整，或使用临时状态管理
    Ok(PublishState {
        planet_id: planet.id.to_string(),  // Uuid 转换为 String
        is_publishing: false,  // 需要从外部状态获取
        step: "idle".into(),
        cid: planet.last_published_cid.clone(),
        error: None,
        started_at: None,  // 需要从外部状态获取
    })
}

/// 更新 Filebase 设置
#[tauri::command]
pub fn planet_update_filebase(
    planet_id: String,
    enabled: bool,
    pin_name: Option<String>,
    api_token: Option<String>,
    store: tauri::State<'_, crate::store::PlanetStoreHandle>,
) -> Result<(), String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;

    // 注意：planet_id 是 String，需要转换为 Uuid
    let planet_uuid = uuid::Uuid::parse_str(&planet_id)
        .map_err(|_| format!("无效的 Planet ID: {}", planet_id))?;
    
    let planet = store.my_planets
        .iter_mut()
        .find(|p| p.id == planet_uuid)
        .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;

    planet.filebase_enabled = Some(enabled);
    planet.filebase_pin_name = pin_name;
    planet.filebase_api_token = api_token;

    // 使用已有的 save 方法
    planet.save(&app_handle).map_err(|e| e.to_string())?;

    Ok(())
}

/// 更新 Pinnable 设置
#[tauri::command]
pub fn planet_update_pinnable(
    planet_id: String,
    enabled: bool,
    api_endpoint: Option<String>,
    store: tauri::State<'_, crate::store::PlanetStoreHandle>,
) -> Result<(), String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;

    // 注意：planet_id 是 String，需要转换为 Uuid
    let planet_uuid = uuid::Uuid::parse_str(&planet_id)
        .map_err(|_| format!("无效的 Planet ID: {}", planet_id))?;
    
    let planet = store.my_planets
        .iter_mut()
        .find(|p| p.id == planet_uuid)
        .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;

    // 注意：MyPlanet 结构体中可能没有 pinnable_enabled 和 pinnable_api_endpoint 字段
    // 需要根据实际结构体定义添加这些字段
    // planet.pinnable_enabled = Some(enabled);
    // planet.pinnable_api_endpoint = api_endpoint;

    // 使用已有的 save 方法
    planet.save(&app_handle).map_err(|e| e.to_string())?;

    Ok(())
}

/// 检查 Filebase pin 状态
#[tauri::command]
pub async fn planet_check_filebase_status(
    planet_id: String,
    store: tauri::State<'_, crate::store::PlanetStoreHandle>,
) -> Result<Option<crate::integrations::filebase::FilebasePin>, String> {
    let store = store.lock().map_err(|e| e.to_string())?;

    // 注意：planet_id 是 String，需要转换为 Uuid
    let planet_uuid = uuid::Uuid::parse_str(&planet_id)
        .map_err(|_| format!("无效的 Planet ID: {}", planet_id))?;
    
    let planet = store.my_planets
        .iter()
        .find(|p| p.id == planet_uuid)
        .ok_or_else(|| format!("Planet '{}' 不存在", planet_id))?;

    // 注意：MyPlanet 结构体中可能没有 filebase_request_id 字段
    if let (Some(ref api_token), Some(ref pin_name)) =
        (&planet.filebase_api_token, &planet.filebase_pin_name)
    {
        // 注意：需要从外部获取 request_id，或从 planet 的其他字段获取
        // 这里假设 request_id 作为参数传入，或从其他来源获取
        // let filebase = Filebase::new(pin_name.clone(), api_token.clone());
        // let result = filebase.check_pin_status(request_id)
        //     .await
        //     .map_err(|e| e.to_string())?;
        // return Ok(result);
        Ok(None)  // 临时返回，需要根据实际实现调整
    }

    Ok(None)
}
```

### 11.2 注册命令到 `main.rs`

```rust
// 在 main.rs 的 invoke_handler 中添加：
.invoke_handler(tauri::generate_handler![
    // ... Phase 1/2 已有命令 ...
    commands::planet::planet_publish,
    commands::planet::planet_get_publish_state,
    commands::planet::planet_update_filebase,
    commands::planet::planet_update_pinnable,
    commands::planet::planet_check_filebase_status,
])
```

### 11.3 更新 `main.rs` 模块声明

```rust
// 在 main.rs 顶部确保声明：
mod integrations;
```

### 11.4 ✅ Step 9 检查点

完成 Step 9 后，执行以下检查：

```bash
cd src-tauri

# 1. 编译检查
cargo check

# 2. 验证 commands 模块可以编译
cargo check --bin planet-windows 2>&1 | grep -i "command\|planet_publish" || echo "✓ commands 模块编译通过"

# 3. 检查命令是否已注册
# 查看 main.rs 中的 invoke_handler，确认所有新命令都已添加
grep -n "planet_publish\|planet_get_publish_state\|planet_update_filebase\|planet_update_pinnable\|planet_check_filebase_status" src-tauri/src/main.rs && echo "✓ 命令已注册"
```

**预期结果**：
- ✅ `cargo check` 无错误
- ✅ 所有 Tauri commands 可以正常编译
- ✅ 命令已在 `main.rs` 中注册

**如果遇到错误**：
- 检查 `main.rs` 中 `invoke_handler` 是否包含所有新命令
- 确认 `mod integrations;` 已在 `main.rs` 中声明
- 验证所有命令函数签名正确（注意参数类型，特别是 `planet_id: String` 需要转换为 `Uuid`）
- 检查 `store` 和 `ipfs_state` 的类型是否正确
- 确认 `app_handle` 参数传递正确

---

## 12. Step 10：前端实现

### 12.1 TypeScript 类型定义

在 `src/types/publish.ts` 中创建：

```typescript
export interface PublishState {
  planetId: string;
  isPublishing: boolean;
  step: 'idle' | 'saving' | 'uploading' | 'publishing' | 'pinning' | 'done' | 'error';
  cid: string | null;
  error: string | null;
  startedAt: string | null;
}

export interface FilebaseSettings {
  enabled: boolean;
  pinName: string;
  apiToken: string;
  requestId?: string;
  pinCid?: string;
}

export interface PinnableSettings {
  enabled: boolean;
  apiEndpoint: string;
  pinCid?: string;
}

export interface FilebasePin {
  cid: string;
  requestId: string;
  status: string;
}
```

### 12.2 Publish Hook

在 `src/hooks/usePublish.ts` 中创建：

```typescript
import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import type { PublishState } from '../types/publish';

export function usePublish(planetId: string | null) {
  const [publishState, setPublishState] = useState<PublishState | null>(null);

  // 监听发布状态变化事件
  useEffect(() => {
    const unlisten = listen<PublishState>('publish-state-changed', (event) => {
      if (event.payload.planetId === planetId) {
        setPublishState(event.payload);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [planetId]);

  // 触发发布
  const publish = useCallback(async () => {
    if (!planetId) return;
    try {
      await invoke('planet_publish', { planetId });
    } catch (error) {
      console.error('发布失败:', error);
      setPublishState({
        planetId,
        isPublishing: false,
        step: 'error',
        cid: null,
        error: String(error),
        startedAt: null,
      });
    }
  }, [planetId]);

  // 查询当前发布状态
  const refreshState = useCallback(async () => {
    if (!planetId) return;
    try {
      const state = await invoke<PublishState>('planet_get_publish_state', { planetId });
      setPublishState(state);
    } catch (error) {
      console.error('查询发布状态失败:', error);
    }
  }, [planetId]);

  return {
    publishState,
    publish,
    refreshState,
    isPublishing: publishState?.isPublishing ?? false,
  };
}
```

### 12.3 PublishButton 组件

在 `src/components/PublishButton.tsx` 中创建：

```tsx
import React from 'react';
import { usePublish } from '../hooks/usePublish';

interface PublishButtonProps {
  planetId: string;
}

const STEP_LABELS: Record<string, string> = {
  idle: '发布',
  saving: '生成站点...',
  uploading: '上传到 IPFS...',
  publishing: '发布到 IPNS...',
  pinning: '远程 Pinning...',
  done: '发布完成',
  error: '发布失败',
};

export const PublishButton: React.FC<PublishButtonProps> = ({ planetId }) => {
  const { publishState, publish, isPublishing } = usePublish(planetId);

  const step = publishState?.step ?? 'idle';
  const label = STEP_LABELS[step] ?? '发布';

  return (
    <div className="flex flex-col gap-2">
      <button
        onClick={publish}
        disabled={isPublishing}
        className={`px-4 py-2 rounded-lg text-white font-medium transition-colors ${
          isPublishing
            ? 'bg-blue-400 cursor-not-allowed'
            : 'bg-blue-600 hover:bg-blue-700'
        }`}
      >
        {isPublishing && (
          <span className="inline-block w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin mr-2" />
        )}
        {label}
      </button>

      {/* 发布状态详情 */}
      {publishState && step !== 'idle' && (
        <div className="text-sm">
          {step === 'done' && publishState.cid && (
            <div className="text-green-600 dark:text-green-400">
              <div>✅ 发布成功</div>
              <div className="font-mono text-xs mt-1 break-all">
                CID: {publishState.cid}
              </div>
            </div>
          )}

          {step === 'error' && (
            <div className="text-red-600 dark:text-red-400">
              ❌ {publishState.error}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
```

### 12.4 PublishInfo 组件（Planet 详情中展示发布信息）

在 `src/components/PublishInfo.tsx` 中创建：

```tsx
import React from 'react';
import type { Planet } from '../types/planet';

interface PublishInfoProps {
  planet: Planet;
  gatewayPort: number | null;
}

export const PublishInfo: React.FC<PublishInfoProps> = ({ planet, gatewayPort }) => {
  const ipnsUrl = gatewayPort
    ? `http://127.0.0.1:${gatewayPort}/ipns/${planet.ipns}`
    : null;

  return (
    <div className="space-y-2 text-sm">
      {/* IPNS */}
      <div>
        <span className="text-gray-500 dark:text-gray-400">IPNS: </span>
        <span className="font-mono text-xs break-all">{planet.ipns}</span>
      </div>

      {/* 上次发布时间 */}
      {planet.lastPublished && (
        <div>
          <span className="text-gray-500 dark:text-gray-400">上次发布: </span>
          <span>{new Date(planet.lastPublished).toLocaleString()}</span>
        </div>
      )}

      {/* 最新 CID */}
      {planet.lastPublishedCid && (
        <div>
          <span className="text-gray-500 dark:text-gray-400">CID: </span>
          <span className="font-mono text-xs break-all">{planet.lastPublishedCid}</span>
        </div>
      )}

      {/* 访问链接 */}
      {ipnsUrl && planet.lastPublished && (
        <div>
          <a
            href={ipnsUrl}
            target="_blank"
            rel="noreferrer"
            className="text-blue-600 dark:text-blue-400 hover:underline text-xs"
          >
            🌐 在浏览器中预览
          </a>
        </div>
      )}
    </div>
  );
};
```

### 12.5 FilebaseSettings 组件

在 `src/components/FilebaseSettings.tsx` 中创建：

```tsx
import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface FilebaseSettingsProps {
  planetId: string;
  initialEnabled: boolean;
  initialPinName: string;
  initialApiToken: string;
}

export const FilebaseSettings: React.FC<FilebaseSettingsProps> = ({
  planetId,
  initialEnabled,
  initialPinName,
  initialApiToken,
}) => {
  const [enabled, setEnabled] = useState(initialEnabled);
  const [pinName, setPinName] = useState(initialPinName);
  const [apiToken, setApiToken] = useState(initialApiToken);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    setSaving(true);
    try {
      await invoke('planet_update_filebase', {
        planetId,
        enabled,
        pinName: pinName || null,
        apiToken: apiToken || null,
      });
    } catch (error) {
      console.error('保存 Filebase 设置失败:', error);
    }
    setSaving(false);
  };

  return (
    <div className="space-y-3 p-4 border rounded-lg dark:border-gray-700">
      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => setEnabled(e.target.checked)}
          id="filebase-enabled"
        />
        <label htmlFor="filebase-enabled" className="font-medium">
          启用 Filebase Pinning
        </label>
      </div>

      {enabled && (
        <>
          <div>
            <label className="block text-sm text-gray-600 dark:text-gray-400 mb-1">Pin Name</label>
            <input
              type="text"
              value={pinName}
              onChange={(e) => setPinName(e.target.value)}
              className="w-full px-3 py-1.5 border rounded dark:bg-gray-800 dark:border-gray-600"
              placeholder="my-planet"
            />
          </div>
          <div>
            <label className="block text-sm text-gray-600 dark:text-gray-400 mb-1">API Token</label>
            <input
              type="password"
              value={apiToken}
              onChange={(e) => setApiToken(e.target.value)}
              className="w-full px-3 py-1.5 border rounded dark:bg-gray-800 dark:border-gray-600"
              placeholder="Bearer token"
            />
          </div>
        </>
      )}

      <button
        onClick={save}
        disabled={saving}
        className="px-3 py-1 bg-gray-200 dark:bg-gray-700 rounded hover:bg-gray-300 dark:hover:bg-gray-600 text-sm"
      >
        {saving ? '保存中...' : '保存设置'}
      </button>
    </div>
  );
};
```

### 12.6 PinnableSettings 组件

在 `src/components/PinnableSettings.tsx` 中创建：

```tsx
import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface PinnableSettingsProps {
  planetId: string;
  initialEnabled: boolean;
  initialApiEndpoint: string;
}

export const PinnableSettings: React.FC<PinnableSettingsProps> = ({
  planetId,
  initialEnabled,
  initialApiEndpoint,
}) => {
  const [enabled, setEnabled] = useState(initialEnabled);
  const [apiEndpoint, setApiEndpoint] = useState(initialApiEndpoint);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    setSaving(true);
    try {
      await invoke('planet_update_pinnable', {
        planetId,
        enabled,
        apiEndpoint: apiEndpoint || null,
      });
    } catch (error) {
      console.error('保存 Pinnable 设置失败:', error);
    }
    setSaving(false);
  };

  return (
    <div className="space-y-3 p-4 border rounded-lg dark:border-gray-700">
      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => setEnabled(e.target.checked)}
          id="pinnable-enabled"
        />
        <label htmlFor="pinnable-enabled" className="font-medium">
          启用 Pinnable.xyz
        </label>
      </div>

      {enabled && (
        <div>
          <label className="block text-sm text-gray-600 dark:text-gray-400 mb-1">
            API Endpoint
          </label>
          <input
            type="text"
            value={apiEndpoint}
            onChange={(e) => setApiEndpoint(e.target.value)}
            className="w-full px-3 py-1.5 border rounded dark:bg-gray-800 dark:border-gray-600"
            placeholder="https://dev.pinnable.xyz/pin/xxxxxxxx"
          />
        </div>
      )}

      <button
        onClick={save}
        disabled={saving}
        className="px-3 py-1 bg-gray-200 dark:bg-gray-700 rounded hover:bg-gray-300 dark:hover:bg-gray-600 text-sm"
      >
        {saving ? '保存中...' : '保存设置'}
      </button>
    </div>
  );
};
```

### 12.7 集成到 Planet 详情页

在现有的 Planet 详情页中添加发布功能：

```tsx
// 在 Planet 详情页组件中添加
import { PublishButton } from './PublishButton';
import { PublishInfo } from './PublishInfo';
import { FilebaseSettings } from './FilebaseSettings';
import { PinnableSettings } from './PinnableSettings';

// 在 JSX 中：
{selectedPlanet && (
  <div className="space-y-6">
    {/* 基本信息 */}
    <div>
      <h2 className="text-xl font-bold">{selectedPlanet.name}</h2>
      <p className="text-gray-600 dark:text-gray-400">{selectedPlanet.about}</p>
    </div>

    {/* 发布信息 */}
    <PublishInfo planet={selectedPlanet} gatewayPort={ipfsState?.gatewayPort ?? null} />

    {/* 发布按钮 */}
    <PublishButton planetId={selectedPlanet.id} />

    {/* Pinning 设置 (折叠面板) */}
    <details className="mt-4">
      <summary className="cursor-pointer text-sm font-medium text-gray-600 dark:text-gray-400">
        ⚙️ 远程 Pinning 设置
      </summary>
      <div className="mt-3 space-y-4">
        <FilebaseSettings
          planetId={selectedPlanet.id}
          initialEnabled={selectedPlanet.filebaseEnabled ?? false}
          initialPinName={selectedPlanet.filebasePinName ?? ''}
          initialApiToken={selectedPlanet.filebaseApiToken ?? ''}
        />
        <PinnableSettings
          planetId={selectedPlanet.id}
          initialEnabled={selectedPlanet.pinnableEnabled ?? false}
          initialApiEndpoint={selectedPlanet.pinnableApiEndpoint ?? ''}
        />
      </div>
    </details>
  </div>
)}
```

### 12.8 ✅ Step 10 检查点

完成 Step 10 后，执行以下检查：

```bash
# 1. TypeScript 类型检查（如果配置了）
cd src
npm run type-check 2>/dev/null || echo "⚠️ 如果项目配置了类型检查，请运行 npm run type-check"

# 2. 检查前端文件是否存在
ls -la src/types/publish.ts && echo "✓ publish.ts 类型定义存在"
ls -la src/hooks/usePublish.ts && echo "✓ usePublish.ts Hook 存在"
ls -la src/components/PublishButton.tsx && echo "✓ PublishButton.tsx 组件存在"
ls -la src/components/PublishInfo.tsx && echo "✓ PublishInfo.tsx 组件存在"
ls -la src/components/FilebaseSettings.tsx && echo "✓ FilebaseSettings.tsx 组件存在"
ls -la src/components/PinnableSettings.tsx && echo "✓ PinnableSettings.tsx 组件存在"

# 3. 验证组件导入路径
# 检查 App.tsx 或其他使用这些组件的地方，确认导入路径正确
```

**预期结果**：
- ✅ 所有前端文件存在
- ✅ TypeScript 类型定义正确（如果使用 TypeScript）
- ✅ 组件可以正常导入和使用

**如果遇到错误**：
- 检查文件路径是否正确
- 验证 TypeScript 类型定义是否与实际 Rust 结构体匹配
- 确认 `invoke` 调用的命令名称与 Rust 后端一致
- 检查事件监听器的事件名称是否正确（`publish-state-changed`）

---

## 13. Step 11：测试与调试

### 13.1 功能验收清单

| # | 测试项 | 验证方法 | 预期结果 |
|---|--------|----------|----------|
| 1 | Markdown → HTML | 创建包含标题、粗体、表格、任务列表的文章 | 正确渲染为 HTML |
| 2 | 模板加载 | 启动应用，检查控制台日志 | 显示 "加载模板: Plain (v1.0.0)" |
| 3 | 文章静态化 | 创建文章后查看 `public/{planet_id}/{article_id}/index.html` | HTML 文件存在且内容正确 |
| 4 | RSS 生成 | 发布后查看 `public/{planet_id}/rss.xml` | 有效的 RSS XML |
| 5 | planet.json | 发布后查看 `public/{planet_id}/planet.json` | 包含所有文章信息 |
| 6 | IPFS 上传 | 点击 Publish | 控制台显示 CID |
| 7 | IPNS 发布 | 点击 Publish | 前端显示发布完成 + CID |
| 8 | 网关访问 | 浏览器打开 `http://127.0.0.1:{port}/ipns/{ipns}` | 显示站点首页 |
| 9 | 文章页面 | 点击首页文章链接 | 跳转到文章详情页 |
| 10 | Key 备份 | 创建 Planet 后检查 Keystore | `keyring` 中有对应条目 |
| 11 | Key 恢复 | 删除 IPFS key → 重新发布 | 自动从 Keystore 恢复 |
| 12 | Filebase | 配置 Filebase → 发布 | 控制台显示 pin 成功 |
| 13 | 发布进度 | 发布过程中观察前端 | 按钮显示各阶段进度 |
| 14 | 重复发布 | CID 不变时再次发布 | 不重复更新 lastPublished |

### 13.2 手动验证步骤

```bash
# 1. 最终编译检查（应该已经通过前面的检查点）
cd src-tauri
cargo check

# 2. 运行所有单元测试
cargo test

# 3. 启动应用
cargo tauri dev

# 4. 在前端创建 Planet，创建文章，点击 Publish

# 5. 检查生成的文件
# Windows:
dir "%APPDATA%\planet-desktop\Planet\Public\{planet_id}\"
# macOS:
ls ~/Library/Application\ Support/planet-desktop/Planet/Public/{planet_id}/

# 6. 预期文件列表：
#   index.html
#   page1.html
#   rss.xml
#   planet.json
#   robots.txt
#   assets/
#   assets/style.css
#   {article_id}/
#   {article_id}/index.html
#   {article_id}/article.json
#   {article_id}/article.md

# 7. 通过 IPFS Gateway 访问
curl http://127.0.0.1:{gateway_port}/ipns/{ipns_key}
```

### 13.3 常见问题排查

| 问题 | 可能原因 | 解决方法 |
|------|----------|----------|
| 模板渲染报错 "variable not found" | Tera context 缺少变量 | 检查 context.insert() 是否遗漏 |
| RSS 中日期格式错误 | chrono 解析失败 | 确认日期字符串为 RFC 3339 格式 |
| IPNS 发布超时 | 网络问题或 DHT 传播慢 | 增加超时时间；添加 `allow-offline` |
| Filebase 返回 401 | API Token 无效 | 检查 Bearer token |
| Keystore 操作失败 | 平台 credential 服务不可用 | Windows: 检查 Credential Manager; Linux: 安装 gnome-keyring |
| 模板 assets 未复制 | `copy_dir_recursive` 路径错误 | 检查 template.assets_path() |
| IPFS key 不存在 | 之前的 key gen 失败或被删除 | 检查 Keystore 备份，手动恢复 |
| 发布后网关 404 | CID 还在传播 | 等待几秒后重试 |

---

## 14. 文件清单

### 14.1 新建文件

| 文件路径 | 说明 |
|----------|------|
| `src-tauri/src/helpers/markdown.rs` | Markdown → HTML 渲染器 |
| `src-tauri/src/template/mod.rs` | 模板引擎 + TemplateStore |
| `src-tauri/src/keystore/mod.rs` | 跨平台密钥安全存储 |
| `src-tauri/src/integrations/mod.rs` | 集成模块声明 |
| `src-tauri/src/integrations/filebase.rs` | Filebase pinning 集成 |
| `src-tauri/src/integrations/pinnable.rs` | Pinnable.xyz pinning 集成 |
| `src-tauri/resources/templates/Plain/template.json` | 内置默认模板元数据 |
| `src-tauri/resources/templates/Plain/templates/index.html` | 默认首页模板 |
| `src-tauri/resources/templates/Plain/templates/blog.html` | 默认文章页模板 |
| `src-tauri/resources/templates/Plain/assets/style.css` | 默认样式 |
| `src/types/publish.ts` | 发布相关 TypeScript 类型 |
| `src/hooks/usePublish.ts` | 发布 React Hook |
| `src/components/PublishButton.tsx` | 发布按钮组件 |
| `src/components/PublishInfo.tsx` | 发布信息展示组件 |
| `src/components/FilebaseSettings.tsx` | Filebase 设置面板 |
| `src/components/PinnableSettings.tsx` | Pinnable 设置面板 |

### 14.2 修改文件

| 文件路径 | 修改内容 |
|----------|----------|
| `src-tauri/Cargo.toml` | 添加 pulldown-cmark, tera, keyring, sha2, hex, regex |
| `src-tauri/src/helpers/mod.rs` | 添加 `pub mod markdown;` |
| `src-tauri/src/main.rs` | 添加 `mod integrations;`，注册新命令 |
| `src-tauri/src/commands/planet.rs` | 添加发布/Filebase/Pinnable 命令 |
| `src-tauri/src/models/planet.rs` | 添加 PublicPlanet, save_public, publish 等 |
| `src-tauri/src/models/article.rs` | 添加 PublicArticle, save_public 等 |
| `src-tauri/src/ipfs/daemon.rs` | 添加 export_key, import_key 方法 |
| `src/App.tsx` | 集成 PublishButton, PublishInfo 等组件 |

---

## 15. Swift → Rust 对照表

| 功能 | Swift 原代码 | Rust 实现 |
|------|-------------|-----------|
| Markdown → HTML | `CMarkRenderer.renderMarkdownHTML()` (libcmark_gfm) | `render_markdown_html()` (pulldown-cmark) |
| YouTube 替换 | `CMarkRenderer.replaceYouTubeLinks()` | `replace_youtube_links()` (regex) |
| 模板引擎 | `Stencil` (Django 风格) | `Tera` (Jinja2 风格) |
| 模板加载 | `Template.from(path:)` | `Template::from_path()` |
| 模板渲染 | `environment.renderTemplate(name:context:)` | `tera.render(name, &context)` |
| 自定义 filter | `ext.registerFilter("md2html")` | `tera.register_filter("md2html", fn)` |
| RSS 渲染 | `Stencil` + 内嵌 RSS.xml | `Tera` + 内嵌 `RSS_TEMPLATE` 常量 |
| Keychain 存储 | `SecItemAdd` / `SecItemCopyMatching` | `keyring::Entry::set_secret/get_secret` |
| Key 导出 | `IPFSCommand.exportKey()` → `KeychainHelper.saveData()` | `IpfsDaemon::export_key()` → `Keystore::save_data()` |
| Key 导入 | `KeychainHelper.loadData()` → `IPFSCommand.importKey()` | `Keystore::load_data()` → `IpfsDaemon::import_key()` |
| 发布流程 | `MyPlanetModel.publish()` async | `Planet::publish()` async |
| 生成静态站点 | `MyPlanetModel.savePublic()` | `Planet::save_public()` |
| 文章静态化 | `MyArticleModel.savePublic()` | `Article::save_public()` |
| PublicPlanet | `PublicPlanetModel` struct | `PublicPlanet` struct |
| PublicArticle | `PublicArticleModel` struct | `PublicArticle` struct |
| 分页逻辑 | `template.idealItemsPerPage` + ceil 分页 | `template.info.ideal_items_per_page` + ceil 分页 |
| Tags 渲染 | `template.generateTagPages` + tag 分组 | `template.info.generate_tag_pages` + HashMap 分组 |
| Archive 渲染 | `template.generateArchive` + 按月分组 | `template.info.generate_archive` + HashMap 分组 |
| style.css hash | `Data.sha256().toHexString()` | `Sha256::digest()` + `hex::encode()` |
| Filebase API | `Filebase.pin(cid:)` + URLSession | `Filebase::pin()` + reqwest |
| Pinnable API | `Pinnable.pin()` + URLSession | `Pinnable::pin()` + reqwest |
| 复制目录 | `FileManager.copyItem(at:to:)` | `copy_dir_recursive()` 递归实现 |
| robots.txt | `MyPlanetModel.saveRobotsTxt()` | 内联在 `save_public()` 中 |
| 发布状态通知 | `PlanetStatusManager.shared.updateStatus()` | `app_handle.emit_all("publish-state-changed", ...)` |

---

## 16. 执行顺序总结

```
Step 1: helpers/markdown.rs       ← 无依赖，可首先实现
    ↓
Step 2: template/mod.rs           ← 依赖 Step 1 (md2html filter)
    ↓
Step 3: keystore/mod.rs           ← 依赖 ipfs/daemon.rs (Phase 1)
    ↓
Step 4: models/article.rs 扩展    ← 依赖 Step 1 + Step 2
    ↓
Step 5: models/planet.rs 扩展     ← 依赖 Step 2 + Step 4
    ↓
Step 6: Publish (planet.rs)       ← 依赖 Step 3 + Step 5
    ↓
Step 7: integrations/filebase.rs  ← 无依赖，可并行
Step 8: integrations/pinnable.rs  ← 无依赖，可并行
    ↓
Step 9: commands/planet.rs 扩展   ← 依赖 Step 6 + 7 + 8
    ↓
Step 10: 前端实现                 ← 依赖 Step 9
    ↓
Step 11: 测试与调试
```

建议实际开发顺序：

1. **第一天**：Step 1 (Markdown) + Step 2 (Template) — 核心渲染引擎
2. **第二天**：Step 3 (Keystore) + Step 7 (Filebase) + Step 8 (Pinnable) — 独立模块
3. **第三天**：Step 4 (Article savePublic) + Step 5 (Planet savePublic) — 站点生成
4. **第四天**：Step 6 (Publish) + Step 9 (Commands) — 发布流程
5. **第五天**：Step 10 (前端) + Step 11 (测试) — 集成验证

> **验收标准再次确认**：创建 Planet → 写文章 → 点击 Publish，能看到发布进度，发布完成后显示 CID。通过 `http://127.0.0.1:{gateway_port}/ipns/{ipns_key}` 可以访问到发布的站点。重启应用后 IPNS key 仍然存在且可用。