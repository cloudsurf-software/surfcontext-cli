//! HTML fragment renderer.
//!
//! Produces semantic HTML with `surfdoc-*` CSS classes. Markdown blocks are
//! rendered through `pulldown-cmark`. All other content is HTML-escaped to
//! prevent XSS.

use crate::types::{Block, CalloutType, DecisionStatus, SurfDoc, Trend};

/// Configuration for full-page HTML rendering with SurfDoc discovery metadata.
#[derive(Debug, Clone)]
pub struct PageConfig {
    /// Path to the original `.surf` source file (served alongside the built site).
    /// Used in `<link rel="alternate">` and the HTML comment.
    pub source_path: String,
    /// Page title. Falls back to front matter `title`, then "SurfDoc".
    pub title: Option<String>,
    /// Optional canonical URL for `<link rel="canonical">`.
    pub canonical_url: Option<String>,
    /// Optional meta description.
    pub description: Option<String>,
    /// Optional language code (default: "en").
    pub lang: Option<String>,
}

impl Default for PageConfig {
    fn default() -> Self {
        Self {
            source_path: "source.surf".to_string(),
            title: None,
            canonical_url: None,
            description: None,
            lang: None,
        }
    }
}

/// Render a `SurfDoc` as an HTML fragment.
///
/// The output is a sequence of semantic HTML elements with `surfdoc-*` CSS
/// classes. No `<html>`, `<head>`, or `<body>` wrapper is added.
pub fn to_html(doc: &SurfDoc) -> String {
    let mut parts: Vec<String> = Vec::new();

    for block in &doc.blocks {
        parts.push(render_block(block));
    }

    parts.join("\n")
}

/// Render a `SurfDoc` as a complete HTML page with SurfDoc discovery metadata.
///
/// Produces a full `<!DOCTYPE html>` document with:
/// - `<meta name="generator" content="SurfDoc v0.1">`
/// - `<link rel="alternate" type="text/surfdoc" href="...">` pointing to source
/// - HTML comment identifying the source file
/// - Standard viewport and charset meta tags
/// - Embedded dark-theme CSS for all SurfDoc block types
pub fn to_html_page(doc: &SurfDoc, config: &PageConfig) -> String {
    let body = to_html(doc);
    let lang = config.lang.as_deref().unwrap_or("en");

    // Resolve title: explicit config > front matter > fallback
    let title = config
        .title
        .clone()
        .or_else(|| {
            doc.front_matter
                .as_ref()
                .and_then(|fm| fm.title.clone())
        })
        .unwrap_or_else(|| "SurfDoc".to_string());

    let source_path = escape_html(&config.source_path);

    // Build optional meta tags
    let mut meta_extra = String::new();
    if let Some(desc) = &config.description {
        meta_extra.push_str(&format!(
            "\n    <meta name=\"description\" content=\"{}\">",
            escape_html(desc)
        ));
    }
    if let Some(url) = &config.canonical_url {
        meta_extra.push_str(&format!(
            "\n    <link rel=\"canonical\" href=\"{}\">",
            escape_html(url)
        ));
    }

    format!(
        r#"<!-- Built with SurfDoc — source: {source_path} -->
<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="generator" content="SurfDoc v0.1">
    <link rel="alternate" type="text/surfdoc" href="{source_path}">
    <title>{title}</title>{meta_extra}
    <style>{css}</style>
</head>
<body>
<article class="surfdoc">
{body}
</article>
</body>
</html>"#,
        source_path = source_path,
        lang = escape_html(lang),
        title = escape_html(&title),
        meta_extra = meta_extra,
        css = SURFDOC_CSS,
        body = body,
    )
}

/// Embedded CSS for standalone SurfDoc pages.
///
/// Dark theme ported from the CloudSurf strategy-web reference implementation.
/// Covers base typography, markdown elements, and all SurfDoc block types.
const SURFDOC_CSS: &str = r#"
:root {
    --bg: #0a0a0f;
    --bg-card: #12121a;
    --bg-hover: #1a1a26;
    --border: #2a2a3a;
    --border-subtle: #1e1e2e;
    --text: #e8e8f0;
    --text-dim: #8888a0;
    --text-muted: #5a5a72;
    --accent: #3b82f6;
}

*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body { background: var(--bg); color: var(--text); font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, sans-serif; -webkit-font-smoothing: antialiased; }
::-webkit-scrollbar { width: 6px; height: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }

/* Layout */
.surfdoc { max-width: 48rem; margin: 0 auto; padding: 2rem 1.5rem 4rem; line-height: 1.7; }

/* Typography */
.surfdoc h1 { font-size: 1.875rem; font-weight: 700; margin: 2rem 0 1rem; letter-spacing: -0.025em; }
.surfdoc h2 { font-size: 1.5rem; font-weight: 600; margin: 1.75rem 0 0.75rem; padding-bottom: 0.5rem; border-bottom: 1px solid var(--border-subtle); }
.surfdoc h3 { font-size: 1.25rem; font-weight: 600; margin: 1.5rem 0 0.5rem; }
.surfdoc h4 { font-size: 1.1rem; font-weight: 600; margin: 1.25rem 0 0.5rem; color: var(--text-dim); }
.surfdoc p { margin: 0.75rem 0; }
.surfdoc a { color: var(--accent); text-decoration: none; }
.surfdoc a:hover { text-decoration: underline; }
.surfdoc strong { font-weight: 600; color: #fff; }
.surfdoc em { color: var(--text-dim); }
.surfdoc ul, .surfdoc ol { margin: 0.5rem 0; padding-left: 1.5rem; }
.surfdoc li { margin: 0.25rem 0; }
.surfdoc li::marker { color: var(--text-muted); }
.surfdoc blockquote { border-left: 3px solid var(--accent); padding: 0.5rem 1rem; margin: 1rem 0; background: rgba(59,130,246,0.05); border-radius: 0 6px 6px 0; }
.surfdoc blockquote p { margin: 0.25rem 0; }
.surfdoc code { font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace; font-size: 0.85em; background: rgba(255,255,255,0.06); padding: 0.15em 0.4em; border-radius: 4px; }
.surfdoc pre { background: #0d1117 !important; border: 1px solid var(--border-subtle); border-radius: 8px; padding: 1rem; overflow-x: auto; margin: 1rem 0; }
.surfdoc pre code { background: transparent; padding: 0; font-size: 0.8rem; line-height: 1.6; }
.surfdoc table { width: 100%; border-collapse: collapse; margin: 1rem 0; font-size: 0.875rem; }
.surfdoc th { text-align: left; padding: 0.5rem 0.75rem; border-bottom: 2px solid var(--border); font-weight: 600; color: var(--text-dim); font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.03em; }
.surfdoc td { padding: 0.5rem 0.75rem; border-bottom: 1px solid var(--border-subtle); }
.surfdoc tr:hover td { background: rgba(255,255,255,0.02); }
.surfdoc hr { border: none; border-top: 1px solid var(--border-subtle); margin: 2rem 0; }
.surfdoc img { max-width: 100%; border-radius: 8px; }

/* Callout blocks */
.surfdoc-callout { border-left: 3px solid; padding: 0.75rem 1rem; margin: 1rem 0; border-radius: 0 8px 8px 0; background: var(--bg-card); }
.surfdoc-callout strong { display: block; margin-bottom: 0.25rem; font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.04em; }
.surfdoc-callout p { margin: 0; }
.surfdoc-callout-info { border-color: #3b82f6; }
.surfdoc-callout-info strong { color: #3b82f6; }
.surfdoc-callout-warning { border-color: #f59e0b; }
.surfdoc-callout-warning strong { color: #f59e0b; }
.surfdoc-callout-danger { border-color: #ef4444; }
.surfdoc-callout-danger strong { color: #ef4444; }
.surfdoc-callout-tip { border-color: #22c55e; }
.surfdoc-callout-tip strong { color: #22c55e; }
.surfdoc-callout-note { border-color: #06b6d4; }
.surfdoc-callout-note strong { color: #06b6d4; }
.surfdoc-callout-success { border-color: #22c55e; background: rgba(34,197,94,0.05); }
.surfdoc-callout-success strong { color: #22c55e; }

/* Data tables */
.surfdoc-data { width: 100%; border-collapse: collapse; margin: 1rem 0; font-size: 0.875rem; border: 1px solid var(--border-subtle); border-radius: 8px; overflow: hidden; }
.surfdoc-data thead { background: var(--bg-card); }
.surfdoc-data th { text-align: left; padding: 0.625rem 0.75rem; font-weight: 600; color: var(--text-dim); font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.03em; border-bottom: 2px solid var(--border); }
.surfdoc-data td { padding: 0.5rem 0.75rem; border-bottom: 1px solid var(--border-subtle); }
.surfdoc-data tr:hover td { background: rgba(255,255,255,0.02); }
.surfdoc-data tr:last-child td { border-bottom: none; }

/* Code blocks */
.surfdoc-code { background: #0d1117; border: 1px solid var(--border-subtle); border-radius: 8px; padding: 1rem; overflow-x: auto; margin: 1rem 0; font-size: 0.8rem; line-height: 1.6; }
.surfdoc-code code { background: transparent; padding: 0; font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace; }

/* Task lists */
.surfdoc-tasks { list-style: none; padding-left: 0; margin: 1rem 0; }
.surfdoc-tasks li { display: flex; align-items: center; gap: 0.5rem; padding: 0.375rem 0.75rem; margin: 0.125rem 0; border-radius: 6px; font-size: 0.9rem; }
.surfdoc-tasks li:hover { background: var(--bg-hover); }
.surfdoc-tasks input[type="checkbox"] { accent-color: var(--accent); width: 16px; height: 16px; }
.surfdoc-tasks .assignee { color: var(--accent); font-size: 0.8rem; margin-left: auto; }

/* Decision records */
.surfdoc-decision { border-left: 3px solid; padding: 0.75rem 1rem; margin: 1rem 0; border-radius: 0 8px 8px 0; background: var(--bg-card); }
.surfdoc-decision .status { display: inline-block; padding: 0.15rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; margin-right: 0.5rem; }
.surfdoc-decision .date { color: var(--text-muted); font-size: 0.8rem; }
.surfdoc-decision p { margin: 0.5rem 0 0; }
.surfdoc-decision-accepted { border-color: #22c55e; }
.surfdoc-decision-accepted .status { background: rgba(34,197,94,0.15); color: #22c55e; }
.surfdoc-decision-rejected { border-color: #ef4444; }
.surfdoc-decision-rejected .status { background: rgba(239,68,68,0.15); color: #ef4444; }
.surfdoc-decision-proposed { border-color: #f59e0b; }
.surfdoc-decision-proposed .status { background: rgba(245,158,11,0.15); color: #f59e0b; }
.surfdoc-decision-superseded { border-color: var(--text-muted); }
.surfdoc-decision-superseded .status { background: rgba(90,90,114,0.15); color: var(--text-muted); }

/* Metric displays */
.surfdoc-metric { display: inline-flex; align-items: baseline; gap: 0.5rem; padding: 0.625rem 1rem; margin: 0.5rem 0.5rem 0.5rem 0; background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: 8px; }
.surfdoc-metric .label { color: var(--text-dim); font-size: 0.8rem; font-weight: 500; }
.surfdoc-metric .value { font-size: 1.25rem; font-weight: 700; color: #fff; }
.surfdoc-metric .unit { color: var(--text-muted); font-size: 0.8rem; }
.surfdoc-metric .trend { font-size: 1rem; }
.surfdoc-metric .trend.up { color: #22c55e; }
.surfdoc-metric .trend.down { color: #ef4444; }
.surfdoc-metric .trend.flat { color: var(--text-muted); }

/* Summary blocks */
.surfdoc-summary { border-left: 3px solid var(--accent); padding: 0.75rem 1rem; margin: 1rem 0; background: rgba(59,130,246,0.04); border-radius: 0 8px 8px 0; font-style: italic; color: var(--text-dim); }
.surfdoc-summary p { margin: 0; }

/* Figure blocks */
.surfdoc-figure { margin: 1.5rem 0; text-align: center; }
.surfdoc-figure img { max-width: 100%; border-radius: 8px; border: 1px solid var(--border-subtle); }
.surfdoc-figure figcaption { margin-top: 0.5rem; font-size: 0.85rem; color: var(--text-muted); font-style: italic; }

/* Unknown blocks */
.surfdoc-unknown { padding: 0.75rem 1rem; margin: 1rem 0; background: var(--bg-card); border: 1px dashed var(--border); border-radius: 8px; color: var(--text-dim); font-size: 0.875rem; }

/* Tabs blocks */
.surfdoc-tabs { margin: 1rem 0; border: 1px solid var(--border-subtle); border-radius: 8px; overflow: hidden; }
.surfdoc-tabs nav { display: flex; background: var(--bg-card); border-bottom: 1px solid var(--border-subtle); }
.surfdoc-tabs nav button { padding: 0.5rem 1rem; background: none; border: none; color: var(--text-muted); font-size: 0.85rem; cursor: pointer; border-bottom: 2px solid transparent; transition: all 0.15s; }
.surfdoc-tabs nav button:hover { color: var(--text); background: var(--bg-hover); }
.surfdoc-tabs nav button.active { color: var(--accent); border-bottom-color: var(--accent); }
.surfdoc-tabs .tab-panel { padding: 1rem; display: none; }
.surfdoc-tabs .tab-panel.active { display: block; }

/* Columns layout */
.surfdoc-columns { display: grid; gap: 1.5rem; margin: 1rem 0; }
.surfdoc-columns[data-cols="2"] { grid-template-columns: repeat(2, 1fr); }
.surfdoc-columns[data-cols="3"] { grid-template-columns: repeat(3, 1fr); }
.surfdoc-columns[data-cols="4"] { grid-template-columns: repeat(4, 1fr); }
.surfdoc-column { min-width: 0; }
@media (max-width: 640px) {
    .surfdoc-columns { grid-template-columns: 1fr !important; }
}

/* Quote blocks */
.surfdoc-quote { border-left: 3px solid var(--text-muted); padding: 0.75rem 1.25rem; margin: 1.5rem 0; }
.surfdoc-quote blockquote { border: none; padding: 0; margin: 0; background: none; font-size: 1.1rem; font-style: italic; color: var(--text-dim); line-height: 1.6; }
.surfdoc-quote .attribution { margin-top: 0.5rem; font-size: 0.85rem; color: var(--text-muted); font-style: normal; }
.surfdoc-quote .attribution::before { content: "— "; }
"#;

/// Escape HTML special characters to prevent XSS.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_block(block: &Block) -> String {
    match block {
        Block::Markdown { content, .. } => {
            let parser = pulldown_cmark::Parser::new(content);
            let mut html_output = String::new();
            pulldown_cmark::html::push_html(&mut html_output, parser);
            html_output
        }

        Block::Callout {
            callout_type,
            title,
            content,
            ..
        } => {
            let type_str = callout_type_str(*callout_type);
            let title_html = match title {
                Some(t) => format!(": {}", escape_html(t)),
                None => String::new(),
            };
            format!(
                "<div class=\"surfdoc-callout surfdoc-callout-{type_str}\"><strong>{}</strong>{title_html}<p>{}</p></div>",
                capitalize(type_str),
                escape_html(content),
            )
        }

        Block::Data {
            headers, rows, ..
        } => {
            let mut html = String::from("<table class=\"surfdoc-data\">");
            if !headers.is_empty() {
                html.push_str("<thead><tr>");
                for h in headers {
                    html.push_str(&format!("<th>{}</th>", escape_html(h)));
                }
                html.push_str("</tr></thead>");
            }
            html.push_str("<tbody>");
            for row in rows {
                html.push_str("<tr>");
                for cell in row {
                    html.push_str(&format!("<td>{}</td>", escape_html(cell)));
                }
                html.push_str("</tr>");
            }
            html.push_str("</tbody></table>");
            html
        }

        Block::Code {
            lang, content, ..
        } => {
            let class = match lang {
                Some(l) => format!(" class=\"language-{}\"", escape_html(l)),
                None => String::new(),
            };
            format!(
                "<pre class=\"surfdoc-code\"><code{}>{}</code></pre>",
                class,
                escape_html(content),
            )
        }

        Block::Tasks { items, .. } => {
            let mut html = String::from("<ul class=\"surfdoc-tasks\">");
            for item in items {
                let checked = if item.done { " checked" } else { "" };
                let assignee_html = match &item.assignee {
                    Some(a) => format!(" <span class=\"assignee\">@{}</span>", escape_html(a)),
                    None => String::new(),
                };
                html.push_str(&format!(
                    "<li><input type=\"checkbox\"{checked} disabled> {}{assignee_html}</li>",
                    escape_html(&item.text),
                ));
            }
            html.push_str("</ul>");
            html
        }

        Block::Decision {
            status,
            date,
            content,
            ..
        } => {
            let status_str = decision_status_str(*status);
            let date_html = match date {
                Some(d) => format!("<span class=\"date\">{}</span>", escape_html(d)),
                None => String::new(),
            };
            format!(
                "<div class=\"surfdoc-decision surfdoc-decision-{status_str}\"><span class=\"status\">{status_str}</span>{date_html}<p>{}</p></div>",
                escape_html(content),
            )
        }

        Block::Metric {
            label,
            value,
            trend,
            unit,
            ..
        } => {
            let trend_html = match trend {
                Some(Trend::Up) => "<span class=\"trend up\">\u{2191}</span>".to_string(),
                Some(Trend::Down) => "<span class=\"trend down\">\u{2193}</span>".to_string(),
                Some(Trend::Flat) => "<span class=\"trend flat\">\u{2192}</span>".to_string(),
                None => String::new(),
            };
            let unit_html = match unit {
                Some(u) => format!("<span class=\"unit\">{}</span>", escape_html(u)),
                None => String::new(),
            };
            format!(
                "<div class=\"surfdoc-metric\"><span class=\"label\">{}</span><span class=\"value\">{}</span>{unit_html}{trend_html}</div>",
                escape_html(label),
                escape_html(value),
            )
        }

        Block::Summary { content, .. } => {
            format!(
                "<div class=\"surfdoc-summary\"><p>{}</p></div>",
                escape_html(content),
            )
        }

        Block::Figure {
            src,
            caption,
            alt,
            ..
        } => {
            let alt_attr = alt.as_deref().unwrap_or("");
            let caption_html = match caption {
                Some(c) => format!("<figcaption>{}</figcaption>", escape_html(c)),
                None => String::new(),
            };
            format!(
                "<figure class=\"surfdoc-figure\"><img src=\"{}\" alt=\"{}\" />{caption_html}</figure>",
                escape_html(src),
                escape_html(alt_attr),
            )
        }

        Block::Tabs { tabs, .. } => {
            let mut html = String::from("<div class=\"surfdoc-tabs\"><nav>");
            for (i, tab) in tabs.iter().enumerate() {
                let active = if i == 0 { " active" } else { "" };
                html.push_str(&format!(
                    "<button class=\"tab-btn{}\" data-tab=\"tab-{}\">{}</button>",
                    active,
                    i,
                    escape_html(&tab.label)
                ));
            }
            html.push_str("</nav>");
            for (i, tab) in tabs.iter().enumerate() {
                let active = if i == 0 { " active" } else { "" };
                let parser = pulldown_cmark::Parser::new(&tab.content);
                let mut content_html = String::new();
                pulldown_cmark::html::push_html(&mut content_html, parser);
                html.push_str(&format!(
                    "<div class=\"tab-panel{}\" data-tab=\"tab-{}\">{}</div>",
                    active, i, content_html
                ));
            }
            // Inline script for tab switching
            html.push_str(r#"<script>document.querySelectorAll('.surfdoc-tabs').forEach(t=>{t.querySelectorAll('.tab-btn').forEach(b=>{b.onclick=()=>{t.querySelectorAll('.tab-btn,.tab-panel').forEach(e=>e.classList.remove('active'));b.classList.add('active');t.querySelector('.tab-panel[data-tab="'+b.dataset.tab+'"]').classList.add('active')}})})</script>"#);
            html.push_str("</div>");
            html
        }

        Block::Columns { columns, .. } => {
            let count = columns.len();
            let mut html = format!(
                "<div class=\"surfdoc-columns\" data-cols=\"{}\">",
                count
            );
            for col in columns {
                let parser = pulldown_cmark::Parser::new(&col.content);
                let mut col_html = String::new();
                pulldown_cmark::html::push_html(&mut col_html, parser);
                html.push_str(&format!(
                    "<div class=\"surfdoc-column\">{}</div>",
                    col_html
                ));
            }
            html.push_str("</div>");
            html
        }

        Block::Quote {
            content,
            attribution,
            cite,
            ..
        } => {
            let mut html = String::from("<div class=\"surfdoc-quote\"><blockquote>");
            html.push_str(&escape_html(content));
            html.push_str("</blockquote>");
            if let Some(attr) = attribution {
                let cite_part = match cite {
                    Some(c) => format!(", <cite>{}</cite>", escape_html(c)),
                    None => String::new(),
                };
                html.push_str(&format!(
                    "<div class=\"attribution\">{}{}</div>",
                    escape_html(attr),
                    cite_part,
                ));
            }
            html.push_str("</div>");
            html
        }

        Block::Unknown {
            name, content, ..
        } => {
            format!(
                "<div class=\"surfdoc-unknown\" data-name=\"{}\">{}</div>",
                escape_html(name),
                escape_html(content),
            )
        }
    }
}

fn callout_type_str(ct: CalloutType) -> &'static str {
    match ct {
        CalloutType::Info => "info",
        CalloutType::Warning => "warning",
        CalloutType::Danger => "danger",
        CalloutType::Tip => "tip",
        CalloutType::Note => "note",
        CalloutType::Success => "success",
    }
}

fn decision_status_str(ds: DecisionStatus) -> &'static str {
    match ds {
        DecisionStatus::Proposed => "proposed",
        DecisionStatus::Accepted => "accepted",
        DecisionStatus::Rejected => "rejected",
        DecisionStatus::Superseded => "superseded",
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn span() -> Span {
        Span {
            start_line: 1,
            end_line: 1,
            start_offset: 0,
            end_offset: 0,
        }
    }

    fn doc_with(blocks: Vec<Block>) -> SurfDoc {
        SurfDoc {
            front_matter: None,
            blocks,
            source: String::new(),
        }
    }

    #[test]
    fn html_callout() {
        let doc = doc_with(vec![Block::Callout {
            callout_type: CalloutType::Warning,
            title: Some("Caution".into()),
            content: "Be careful.".into(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-callout surfdoc-callout-warning\""));
        assert!(html.contains("<strong>Warning</strong>"));
        assert!(html.contains("Be careful."));
    }

    #[test]
    fn html_data_table() {
        let doc = doc_with(vec![Block::Data {
            id: None,
            format: DataFormat::Table,
            sortable: false,
            headers: vec!["Name".into(), "Age".into()],
            rows: vec![vec!["Alice".into(), "30".into()]],
            raw_content: String::new(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("<table class=\"surfdoc-data\">"));
        assert!(html.contains("<thead>"));
        assert!(html.contains("<tbody>"));
        assert!(html.contains("<th>Name</th>"));
        assert!(html.contains("<td>Alice</td>"));
    }

    #[test]
    fn html_code() {
        let doc = doc_with(vec![Block::Code {
            lang: Some("rust".into()),
            file: None,
            highlight: vec![],
            content: "fn main() { println!(\"<hello>\"); }".into(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("<pre class=\"surfdoc-code\">"));
        assert!(html.contains("class=\"language-rust\""));
        assert!(html.contains("&lt;hello&gt;"), "Angle brackets should be escaped");
    }

    #[test]
    fn html_tasks() {
        let doc = doc_with(vec![Block::Tasks {
            items: vec![
                TaskItem {
                    done: true,
                    text: "Done item".into(),
                    assignee: None,
                },
                TaskItem {
                    done: false,
                    text: "Pending item".into(),
                    assignee: None,
                },
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("<input type=\"checkbox\" checked disabled>"));
        assert!(html.contains("<input type=\"checkbox\" disabled>"));
    }

    #[test]
    fn html_metric() {
        let doc = doc_with(vec![Block::Metric {
            label: "Revenue".into(),
            value: "$10K".into(),
            trend: Some(Trend::Up),
            unit: None,
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-metric\""));
        assert!(html.contains("<span class=\"label\">Revenue</span>"));
        assert!(html.contains("<span class=\"value\">$10K</span>"));
        assert!(html.contains("class=\"trend up\""));
    }

    #[test]
    fn html_figure() {
        let doc = doc_with(vec![Block::Figure {
            src: "arch.png".into(),
            caption: Some("Architecture diagram".into()),
            alt: Some("System architecture".into()),
            width: None,
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("<figure class=\"surfdoc-figure\">"));
        assert!(html.contains("<img src=\"arch.png\" alt=\"System architecture\" />"));
        assert!(html.contains("<figcaption>Architecture diagram</figcaption>"));
    }

    #[test]
    fn html_markdown_rendered() {
        let doc = doc_with(vec![Block::Markdown {
            content: "# Hello\n\nWorld".into(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn html_escaping() {
        let doc = doc_with(vec![Block::Callout {
            callout_type: CalloutType::Info,
            title: None,
            content: "<script>alert('xss')</script>".into(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(
            !html.contains("<script>"),
            "Script tags must be escaped"
        );
        assert!(html.contains("&lt;script&gt;"));
    }

    // -- New block types (tabs, columns, quote) -------------------------

    #[test]
    fn html_tabs() {
        let doc = doc_with(vec![Block::Tabs {
            tabs: vec![
                crate::types::TabPanel {
                    label: "Overview".into(),
                    content: "Intro text.".into(),
                },
                crate::types::TabPanel {
                    label: "Details".into(),
                    content: "Technical info.".into(),
                },
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-tabs\""));
        assert!(html.contains("Overview"));
        assert!(html.contains("Details"));
        assert!(html.contains("Intro text."));
        assert!(html.contains("Technical info."));
        assert!(html.contains("tab-btn"));
        assert!(html.contains("tab-panel"));
    }

    #[test]
    fn html_columns() {
        let doc = doc_with(vec![Block::Columns {
            columns: vec![
                crate::types::ColumnContent {
                    content: "Left side.".into(),
                },
                crate::types::ColumnContent {
                    content: "Right side.".into(),
                },
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-columns\""));
        assert!(html.contains("data-cols=\"2\""));
        assert!(html.contains("class=\"surfdoc-column\""));
        assert!(html.contains("Left side."));
        assert!(html.contains("Right side."));
    }

    #[test]
    fn html_quote_with_attribution() {
        let doc = doc_with(vec![Block::Quote {
            content: "The best way to predict the future is to invent it.".into(),
            attribution: Some("Alan Kay".into()),
            cite: Some("ACM 1971".into()),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-quote\""));
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("class=\"attribution\""));
        assert!(html.contains("Alan Kay"));
        assert!(html.contains("<cite>ACM 1971</cite>"));
    }

    #[test]
    fn html_quote_no_attribution() {
        let doc = doc_with(vec![Block::Quote {
            content: "Anonymous wisdom.".into(),
            attribution: None,
            cite: None,
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-quote\""));
        assert!(html.contains("Anonymous wisdom."));
        assert!(!html.contains("attribution"));
    }

    // -- Full-page discovery mechanism ---------------------------------

    #[test]
    fn html_page_has_generator_meta() {
        let doc = doc_with(vec![Block::Markdown {
            content: "# Hello".into(),
            span: span(),
        }]);
        let config = PageConfig::default();
        let html = to_html_page(&doc, &config);
        assert!(html.contains("<meta name=\"generator\" content=\"SurfDoc v0.1\">"));
    }

    #[test]
    fn html_page_has_link_alternate() {
        let doc = doc_with(vec![]);
        let config = PageConfig::default();
        let html = to_html_page(&doc, &config);
        assert!(html.contains(
            "<link rel=\"alternate\" type=\"text/surfdoc\" href=\"source.surf\">"
        ));
    }

    #[test]
    fn html_page_has_source_comment() {
        let doc = doc_with(vec![]);
        let config = PageConfig {
            source_path: "site.surf".to_string(),
            ..Default::default()
        };
        let html = to_html_page(&doc, &config);
        assert!(html.starts_with("<!-- Built with SurfDoc — source: site.surf -->"));
    }

    #[test]
    fn html_page_uses_front_matter_title() {
        let doc = SurfDoc {
            front_matter: Some(FrontMatter {
                title: Some("My Site".to_string()),
                ..Default::default()
            }),
            blocks: vec![],
            source: String::new(),
        };
        let config = PageConfig::default();
        let html = to_html_page(&doc, &config);
        assert!(html.contains("<title>My Site</title>"));
    }

    #[test]
    fn html_page_config_title_overrides_front_matter() {
        let doc = SurfDoc {
            front_matter: Some(FrontMatter {
                title: Some("FM Title".to_string()),
                ..Default::default()
            }),
            blocks: vec![],
            source: String::new(),
        };
        let config = PageConfig {
            title: Some("Override Title".to_string()),
            ..Default::default()
        };
        let html = to_html_page(&doc, &config);
        assert!(html.contains("<title>Override Title</title>"));
        assert!(!html.contains("FM Title"));
    }

    #[test]
    fn html_page_has_doctype_and_structure() {
        let doc = doc_with(vec![Block::Markdown {
            content: "Hello".into(),
            span: span(),
        }]);
        let config = PageConfig::default();
        let html = to_html_page(&doc, &config);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("<meta charset=\"utf-8\">"));
        assert!(html.contains("<meta name=\"viewport\""));
        assert!(html.contains("<body>"));
        assert!(html.contains("</body>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn html_page_includes_description_and_canonical() {
        let doc = doc_with(vec![]);
        let config = PageConfig {
            description: Some("A test page".to_string()),
            canonical_url: Some("https://example.com/page".to_string()),
            ..Default::default()
        };
        let html = to_html_page(&doc, &config);
        assert!(html.contains("<meta name=\"description\" content=\"A test page\">"));
        assert!(html.contains(
            "<link rel=\"canonical\" href=\"https://example.com/page\">"
        ));
    }

    #[test]
    fn html_page_custom_source_path() {
        let doc = doc_with(vec![]);
        let config = PageConfig {
            source_path: "/docs/readme.surf".to_string(),
            ..Default::default()
        };
        let html = to_html_page(&doc, &config);
        assert!(html.contains("href=\"/docs/readme.surf\""));
        assert!(html.contains("source: /docs/readme.surf"));
    }

    #[test]
    fn html_page_escapes_title_xss() {
        let doc = doc_with(vec![]);
        let config = PageConfig {
            title: Some("<script>alert('xss')</script>".to_string()),
            ..Default::default()
        };
        let html = to_html_page(&doc, &config);
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;script&gt;"));
    }
}
