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