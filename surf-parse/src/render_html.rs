//! HTML fragment renderer.
//!
//! Produces semantic HTML with `surfdoc-*` CSS classes. Markdown blocks are
//! rendered through `pulldown-cmark`. All other content is HTML-escaped to
//! prevent XSS.

use crate::types::{Block, CalloutType, DecisionStatus, StyleProperty, SurfDoc, Trend};

/// Render a markdown string to HTML using pulldown-cmark with GFM extensions.
fn render_markdown(content: &str) -> String {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_TABLES);
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    options.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
    let parser = pulldown_cmark::Parser::new_ext(content, options);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
}

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
/// If a `::site` block sets an accent color, a `<style>` override is injected.
pub fn to_html(doc: &SurfDoc) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut css_overrides = String::new();

    // Scan for site-level CSS variable overrides before rendering blocks.
    for block in &doc.blocks {
        if let Block::Site { properties, .. } = block {
            for prop in properties {
                match prop.key.as_str() {
                    "accent" => css_overrides.push_str(&format!(
                        "--accent: {};", escape_html(&prop.value)
                    )),
                    "font" => css_overrides.push_str(&format!(
                        "--font: {};", escape_html(&prop.value)
                    )),
                    _ => {}
                }
            }
        }
    }

    if !css_overrides.is_empty() {
        parts.push(format!("<style>:root {{ {} }}</style>", css_overrides));
    }

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

/* CTA buttons */
.surfdoc-cta { display: inline-block; padding: 0.625rem 1.5rem; margin: 0.5rem 0.5rem 0.5rem 0; border-radius: 8px; font-weight: 600; font-size: 0.95rem; text-decoration: none; transition: all 0.15s; cursor: pointer; }
.surfdoc-cta-primary { background: var(--accent); color: #fff; border: 1px solid var(--accent); }
.surfdoc-cta-primary:hover { background: #2563eb; text-decoration: none; }
.surfdoc-cta-secondary { background: transparent; color: var(--accent); border: 1px solid var(--border); }
.surfdoc-cta-secondary:hover { background: var(--bg-hover); text-decoration: none; }

/* Hero image */
.surfdoc-hero-image { margin: 2rem 0; text-align: center; }
.surfdoc-hero-image img { max-width: 100%; border-radius: 12px; box-shadow: 0 8px 32px rgba(0,0,0,0.3); border: 1px solid var(--border-subtle); }

/* Testimonials */
.surfdoc-testimonial { padding: 1.25rem 1.5rem; margin: 1rem 0; background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: 12px; position: relative; }
.surfdoc-testimonial blockquote { border: none; background: none; padding: 0; margin: 0 0 0.75rem; font-size: 1rem; font-style: italic; color: var(--text-dim); line-height: 1.6; }
.surfdoc-testimonial .author { font-weight: 600; color: var(--text); font-size: 0.9rem; }
.surfdoc-testimonial .role { color: var(--text-muted); font-size: 0.8rem; }

/* Style blocks — invisible, metadata only */
.surfdoc-style { display: none; }

/* FAQ accordion */
.surfdoc-faq { margin: 1rem 0; }
.surfdoc-faq details { border: 1px solid var(--border-subtle); border-radius: 8px; margin: 0.5rem 0; overflow: hidden; }
.surfdoc-faq summary { padding: 0.75rem 1rem; font-weight: 600; cursor: pointer; background: var(--bg-card); color: var(--text); font-size: 0.95rem; }
.surfdoc-faq summary:hover { background: var(--bg-hover); }
.surfdoc-faq .faq-answer { padding: 0.75rem 1rem; color: var(--text-dim); line-height: 1.6; border-top: 1px solid var(--border-subtle); }

/* Pricing table */
.surfdoc-pricing { width: 100%; border-collapse: collapse; margin: 1rem 0; font-size: 0.875rem; border: 1px solid var(--border-subtle); border-radius: 8px; overflow: hidden; }
.surfdoc-pricing thead { background: var(--bg-card); }
.surfdoc-pricing th { text-align: center; padding: 0.75rem; font-weight: 600; color: var(--text); border-bottom: 2px solid var(--border); font-size: 0.95rem; }
.surfdoc-pricing th:first-child { text-align: left; color: var(--text-muted); font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.03em; }
.surfdoc-pricing td { padding: 0.5rem 0.75rem; border-bottom: 1px solid var(--border-subtle); text-align: center; }
.surfdoc-pricing td:first-child { text-align: left; font-weight: 500; color: var(--text-dim); }
.surfdoc-pricing tr:hover td { background: rgba(255,255,255,0.02); }
.surfdoc-pricing tr:last-child td { border-bottom: none; }

/* Site config — invisible, metadata only */
.surfdoc-site { display: none; }

/* Page sections */
.surfdoc-page { margin: 2rem 0; padding: 2rem 0; border-top: 2px solid var(--border-subtle); }
.surfdoc-page[data-layout="hero"] { text-align: center; padding: 4rem 0; }
.surfdoc-page[data-layout="hero"] h1 { font-size: 2.5rem; margin-bottom: 1rem; }
.surfdoc-page[data-layout="hero"] p { font-size: 1.15rem; color: var(--text-dim); max-width: 36rem; margin: 0 auto 1.5rem; }
.surfdoc-page[data-layout="hero"] .surfdoc-cta { margin: 0.5rem; }
.surfdoc-page[data-layout="cards"] { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; }
.surfdoc-page[data-layout="split"] { display: grid; grid-template-columns: 1fr 1fr; gap: 2rem; align-items: center; }
@media (max-width: 640px) {
    .surfdoc-page[data-layout="split"] { grid-template-columns: 1fr; }
    .surfdoc-page[data-layout="hero"] h1 { font-size: 1.75rem; }
}
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
        Block::Markdown { content, .. } => render_markdown(content),

        Block::Callout {
            callout_type,
            title,
            content,
            ..
        } => {
            let type_str = callout_type_str(*callout_type);
            let role = if matches!(callout_type, CalloutType::Danger) { "alert" } else { "note" };
            let heading = match title {
                Some(t) => format!("{}: {}", capitalize(type_str), escape_html(t)),
                None => capitalize(type_str).to_string(),
            };
            format!(
                "<div class=\"surfdoc-callout surfdoc-callout-{type_str}\" role=\"{role}\"><strong>{heading}</strong><p>{}</p></div>",
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
                    html.push_str(&format!("<th scope=\"col\">{}</th>", escape_html(h)));
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
            let aria = match lang {
                Some(l) => format!(" aria-label=\"{} code\"", escape_html(l)),
                None => String::new(),
            };
            format!(
                "<pre class=\"surfdoc-code\"{}><code{}>{}</code></pre>",
                aria,
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
                    "<li><label><input type=\"checkbox\"{checked} disabled> {}</label>{assignee_html}</li>",
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
                "<div class=\"surfdoc-decision surfdoc-decision-{status_str}\" role=\"note\" aria-label=\"Decision: {status_str}\"><span class=\"status\">{status_str}</span>{date_html}<p>{}</p></div>",
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
            let trend_text = match trend {
                Some(Trend::Up) => ", trending up",
                Some(Trend::Down) => ", trending down",
                Some(Trend::Flat) => ", flat",
                None => "",
            };
            let unit_text = match unit {
                Some(u) => format!(" {}", u),
                None => String::new(),
            };
            let aria_label = format!("{}: {}{}{}", label, value, unit_text, trend_text);
            format!(
                "<div class=\"surfdoc-metric\" role=\"group\" aria-label=\"{}\"><span class=\"label\">{}</span><span class=\"value\">{}</span>{unit_html}{trend_html}</div>",
                escape_html(&aria_label),
                escape_html(label),
                escape_html(value),
            )
        }

        Block::Summary { content, .. } => {
            format!(
                "<div class=\"surfdoc-summary\" role=\"doc-abstract\"><p>{}</p></div>",
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
            let mut html = String::from("<div class=\"surfdoc-tabs\">");
            html.push_str("<nav role=\"tablist\">");
            for (i, tab) in tabs.iter().enumerate() {
                let selected = if i == 0 { "true" } else { "false" };
                let tabindex = if i == 0 { "0" } else { "-1" };
                html.push_str(&format!(
                    "<button class=\"tab-btn{}\" role=\"tab\" aria-selected=\"{}\" aria-controls=\"surfdoc-panel-{}\" id=\"surfdoc-tab-{}\" tabindex=\"{}\">{}</button>",
                    if i == 0 { " active" } else { "" },
                    selected,
                    i,
                    i,
                    tabindex,
                    escape_html(&tab.label)
                ));
            }
            html.push_str("</nav>");
            for (i, tab) in tabs.iter().enumerate() {
                let active = if i == 0 { " active" } else { "" };
                let hidden = if i == 0 { "" } else { " hidden" };
                let content_html = render_markdown(&tab.content);
                html.push_str(&format!(
                    "<div class=\"tab-panel{}\" role=\"tabpanel\" id=\"surfdoc-panel-{}\" aria-labelledby=\"surfdoc-tab-{}\" tabindex=\"0\"{}>{}</div>",
                    active, i, i, hidden, content_html
                ));
            }
            html.push_str(r#"<script>document.querySelectorAll('.surfdoc-tabs').forEach(t=>{t.querySelectorAll('[role="tab"]').forEach(b=>{b.onclick=()=>{t.querySelectorAll('[role="tab"]').forEach(e=>{e.classList.remove('active');e.setAttribute('aria-selected','false');e.tabIndex=-1});b.classList.add('active');b.setAttribute('aria-selected','true');b.tabIndex=0;t.querySelectorAll('[role="tabpanel"]').forEach(p=>{p.classList.remove('active');p.hidden=true});var panel=document.getElementById(b.getAttribute('aria-controls'));if(panel){panel.classList.add('active');panel.hidden=false}}})})</script>"#);
            html.push_str("</div>");
            html
        }

        Block::Columns { columns, .. } => {
            let count = columns.len();
            let mut html = format!(
                "<div class=\"surfdoc-columns\" role=\"group\" data-cols=\"{}\">",
                count
            );
            for col in columns {
                let col_html = render_markdown(&col.content);
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

        Block::Cta {
            label,
            href,
            primary,
            ..
        } => {
            let class = if *primary { "surfdoc-cta surfdoc-cta-primary" } else { "surfdoc-cta surfdoc-cta-secondary" };
            format!(
                "<a class=\"{}\" href=\"{}\">{}</a>",
                class,
                escape_html(href),
                escape_html(label),
            )
        }

        Block::HeroImage { src, alt, .. } => {
            let alt_attr = alt.as_deref().unwrap_or("");
            let role_attr = if !alt_attr.is_empty() {
                format!(" role=\"img\" aria-label=\"{}\"", escape_html(alt_attr))
            } else {
                String::new()
            };
            format!(
                "<div class=\"surfdoc-hero-image\"{}><img src=\"{}\" alt=\"{}\" /></div>",
                role_attr,
                escape_html(src),
                escape_html(alt_attr),
            )
        }

        Block::Testimonial {
            content,
            author,
            role,
            company,
            ..
        } => {
            let aria_label = match author {
                Some(a) => format!(" aria-label=\"Testimonial from {}\"", escape_html(a)),
                None => " aria-label=\"Testimonial\"".to_string(),
            };
            let mut html = format!("<div class=\"surfdoc-testimonial\" role=\"figure\"{}><blockquote>", aria_label);
            html.push_str(&escape_html(content));
            html.push_str("</blockquote>");
            if author.is_some() || role.is_some() || company.is_some() {
                html.push_str("<div class=\"author\">");
                if let Some(a) = author {
                    html.push_str(&escape_html(a));
                }
                let details: Vec<&str> = [role.as_deref(), company.as_deref()]
                    .iter()
                    .filter_map(|v| *v)
                    .collect();
                if !details.is_empty() {
                    html.push_str(&format!(
                        " <span class=\"role\">{}</span>",
                        escape_html(&details.join(", "))
                    ));
                }
                html.push_str("</div>");
            }
            html.push_str("</div>");
            html
        }

        Block::Style { properties, .. } => {
            // Style blocks are metadata — rendered as a hidden data element
            let pairs: Vec<String> = properties
                .iter()
                .map(|p| format!("{}={}", escape_html(&p.key), escape_html(&p.value)))
                .collect();
            format!(
                "<div class=\"surfdoc-style\" aria-hidden=\"true\" data-properties=\"{}\"></div>",
                escape_html(&pairs.join(";"))
            )
        }

        Block::Faq { items, .. } => {
            let mut html = String::from("<div class=\"surfdoc-faq\">");
            for item in items {
                html.push_str(&format!(
                    "<details><summary>{}</summary><div class=\"faq-answer\">{}</div></details>",
                    escape_html(&item.question),
                    escape_html(&item.answer),
                ));
            }
            html.push_str("</div>");
            html
        }

        Block::PricingTable {
            headers, rows, ..
        } => {
            let mut html = String::from("<table class=\"surfdoc-pricing\" aria-label=\"Pricing comparison\">");
            if !headers.is_empty() {
                html.push_str("<thead><tr>");
                for h in headers {
                    html.push_str(&format!("<th scope=\"col\">{}</th>", escape_html(h)));
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

        Block::Site { properties, domain, .. } => {
            // Site config is metadata — hidden element with data attributes
            let domain_attr = match domain {
                Some(d) => format!(" data-domain=\"{}\"", escape_html(d)),
                None => String::new(),
            };
            let pairs: Vec<String> = properties
                .iter()
                .map(|p| format!("{}={}", escape_html(&p.key), escape_html(&p.value)))
                .collect();
            format!(
                "<div class=\"surfdoc-site\" aria-hidden=\"true\"{} data-properties=\"{}\"></div>",
                domain_attr,
                escape_html(&pairs.join(";")),
            )
        }

        Block::Page {
            route, layout, title, children, ..
        } => {
            let layout_attr = match layout {
                Some(l) => format!(" data-layout=\"{}\"", escape_html(l)),
                None => String::new(),
            };
            let aria_label = match title {
                Some(t) => format!(" aria-label=\"{}\"", escape_html(t)),
                None => format!(" aria-label=\"Page: {}\"", escape_html(route)),
            };
            let mut html = format!("<section class=\"surfdoc-page\"{layout_attr}{aria_label}>");
            for child in children {
                html.push_str(&render_block(child));
            }
            html.push_str("</section>");
            html
        }

        Block::Unknown {
            name, content, ..
        } => {
            format!(
                "<div class=\"surfdoc-unknown\" role=\"note\" data-name=\"{}\">{}</div>",
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

// -- Multi-page site extraction and rendering --------------------------

/// Extracted site-level configuration from a `::site` block.
#[derive(Debug, Clone, Default)]
pub struct SiteConfig {
    pub domain: Option<String>,
    pub name: Option<String>,
    pub tagline: Option<String>,
    pub theme: Option<String>,
    pub accent: Option<String>,
    pub font: Option<String>,
    pub properties: Vec<StyleProperty>,
}

/// A single page extracted from a `::page` block.
#[derive(Debug, Clone)]
pub struct PageEntry {
    pub route: String,
    pub layout: Option<String>,
    pub title: Option<String>,
    pub sidebar: bool,
    pub children: Vec<Block>,
}

/// Extract site config and page list from a parsed SurfDoc.
///
/// Returns `(site_config, pages, loose_blocks)` where `loose_blocks` are
/// top-level blocks that are neither `Site` nor `Page`.
pub fn extract_site(doc: &SurfDoc) -> (Option<SiteConfig>, Vec<PageEntry>, Vec<Block>) {
    let mut site_config: Option<SiteConfig> = None;
    let mut pages: Vec<PageEntry> = Vec::new();
    let mut loose: Vec<Block> = Vec::new();

    for block in &doc.blocks {
        match block {
            Block::Site {
                domain,
                properties,
                ..
            } => {
                let mut config = SiteConfig {
                    domain: domain.clone(),
                    properties: properties.clone(),
                    ..Default::default()
                };
                for prop in properties {
                    match prop.key.as_str() {
                        "name" => config.name = Some(prop.value.clone()),
                        "tagline" => config.tagline = Some(prop.value.clone()),
                        "theme" => config.theme = Some(prop.value.clone()),
                        "accent" => config.accent = Some(prop.value.clone()),
                        "font" => config.font = Some(prop.value.clone()),
                        _ => {}
                    }
                }
                site_config = Some(config);
            }
            Block::Page {
                route,
                layout,
                title,
                sidebar,
                children,
                ..
            } => {
                pages.push(PageEntry {
                    route: route.clone(),
                    layout: layout.clone(),
                    title: title.clone(),
                    sidebar: *sidebar,
                    children: children.clone(),
                });
            }
            other => {
                loose.push(other.clone());
            }
        }
    }

    (site_config, pages, loose)
}

/// CSS for site-level navigation and footer.
const SITE_NAV_CSS: &str = r#"
/* Site navigation */
.surfdoc-site-nav { display: flex; align-items: center; gap: 1.5rem; padding: 0.75rem 1.5rem; background: var(--bg-card); border-bottom: 1px solid var(--border-subtle); max-width: 100%; position: sticky; top: 0; z-index: 100; }
.surfdoc-site-nav .site-name { font-weight: 700; color: #fff; font-size: 1rem; text-decoration: none; margin-right: auto; }
.surfdoc-site-nav a { color: var(--text-dim); text-decoration: none; font-size: 0.875rem; padding: 0.25rem 0.5rem; border-radius: 4px; transition: color 0.15s, background 0.15s; }
.surfdoc-site-nav a:hover { color: var(--text); background: var(--bg-hover); }
.surfdoc-site-nav a.active { color: var(--accent); font-weight: 600; }

/* Site footer */
.surfdoc-site-footer { margin-top: 4rem; padding: 1.5rem; border-top: 1px solid var(--border-subtle); text-align: center; color: var(--text-muted); font-size: 0.8rem; }
"#;

/// Render a full HTML page for one route within a multi-page site.
///
/// Produces a `<!DOCTYPE html>` page with site-level `<nav>`, page content,
/// and a footer. Theme and accent from `SiteConfig` are applied via CSS variables.
pub fn render_site_page(
    page: &PageEntry,
    site: &SiteConfig,
    nav_items: &[(String, String)], // (route, title) pairs
    config: &PageConfig,
) -> String {
    // Render page children as HTML
    let mut body_parts: Vec<String> = Vec::new();
    for child in &page.children {
        body_parts.push(render_block(child));
    }
    let body = body_parts.join("\n");

    let lang = config.lang.as_deref().unwrap_or("en");
    let site_name = site
        .name
        .as_deref()
        .unwrap_or("SurfDoc Site");

    // Title: page title > site name + route
    let title = match &page.title {
        Some(t) => format!("{} — {}", t, site_name),
        None if page.route == "/" => site_name.to_string(),
        None => format!("{} — {}", page.route.trim_start_matches('/'), site_name),
    };

    let source_path = escape_html(&config.source_path);

    // Build navigation HTML
    let mut nav_html = format!(
        "<nav class=\"surfdoc-site-nav\" role=\"navigation\" aria-label=\"Site navigation\">\n  <a href=\"/index.html\" class=\"site-name\">{}</a>\n",
        escape_html(site_name)
    );
    for (route, nav_title) in nav_items {
        let href = if route == "/" {
            "/index.html".to_string()
        } else {
            format!("{}/index.html", route)
        };
        let active = if *route == page.route { " active" } else { "" };
        nav_html.push_str(&format!(
            "  <a href=\"{}\"{}>{}</a>\n",
            escape_html(&href),
            if active.is_empty() {
                String::new()
            } else {
                format!(" class=\"active\"")
            },
            escape_html(nav_title),
        ));
    }
    nav_html.push_str("</nav>");

    // Build footer
    let footer_html = format!(
        "<footer class=\"surfdoc-site-footer\">{}</footer>",
        escape_html(site_name),
    );

    // Build optional CSS variable overrides from site config
    let mut css_overrides = String::new();
    if let Some(accent) = &site.accent {
        css_overrides.push_str(&format!("--accent: {};\n", escape_html(accent)));
    }
    let override_block = if css_overrides.is_empty() {
        String::new()
    } else {
        format!("\n:root {{\n{}}}", css_overrides)
    };

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
    <style>{css}{nav_css}{override_block}</style>
</head>
<body>
{nav}
<article class="surfdoc">
{body}
</article>
{footer}
</body>
</html>"#,
        source_path = source_path,
        lang = escape_html(lang),
        title = escape_html(&title),
        meta_extra = meta_extra,
        css = SURFDOC_CSS,
        nav_css = SITE_NAV_CSS,
        override_block = override_block,
        nav = nav_html,
        body = body,
        footer = footer_html,
    )
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
        assert!(html.contains("<strong>Warning: Caution</strong>"));
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
        assert!(html.contains("<th scope=\"col\">Name</th>"));
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
        assert!(html.contains("<pre class=\"surfdoc-code\" aria-label=\"rust code\">"));
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

    // -- Web blocks (cta, hero-image, testimonial, style) ---------------

    #[test]
    fn html_cta_primary() {
        let doc = doc_with(vec![Block::Cta {
            label: "Get Started".into(),
            href: "/signup".into(),
            primary: true,
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-cta surfdoc-cta-primary\""));
        assert!(html.contains("href=\"/signup\""));
        assert!(html.contains("Get Started"));
    }

    #[test]
    fn html_cta_secondary() {
        let doc = doc_with(vec![Block::Cta {
            label: "Learn More".into(),
            href: "https://example.com".into(),
            primary: false,
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("surfdoc-cta-secondary"));
        assert!(html.contains("Learn More"));
    }

    #[test]
    fn html_hero_image() {
        let doc = doc_with(vec![Block::HeroImage {
            src: "screenshot.png".into(),
            alt: Some("App screenshot".into()),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-hero-image\""));
        assert!(html.contains("src=\"screenshot.png\""));
        assert!(html.contains("alt=\"App screenshot\""));
    }

    #[test]
    fn html_testimonial() {
        let doc = doc_with(vec![Block::Testimonial {
            content: "Amazing product!".into(),
            author: Some("Jane Dev".into()),
            role: Some("Engineer".into()),
            company: Some("Acme".into()),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-testimonial\""));
        assert!(html.contains("Amazing product!"));
        assert!(html.contains("Jane Dev"));
        assert!(html.contains("Engineer, Acme"));
    }

    #[test]
    fn html_testimonial_anonymous() {
        let doc = doc_with(vec![Block::Testimonial {
            content: "Great tool.".into(),
            author: None,
            role: None,
            company: None,
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("Great tool."));
        assert!(!html.contains("class=\"author\""));
    }

    #[test]
    fn html_style_hidden() {
        let doc = doc_with(vec![Block::Style {
            properties: vec![
                crate::types::StyleProperty { key: "accent".into(), value: "#6366f1".into() },
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-style\""));
    }

    #[test]
    fn html_cta_escapes_xss() {
        let doc = doc_with(vec![Block::Cta {
            label: "<script>alert('xss')</script>".into(),
            href: "javascript:alert(1)".into(),
            primary: true,
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn html_faq() {
        let doc = doc_with(vec![Block::Faq {
            items: vec![
                crate::types::FaqItem {
                    question: "Is it free?".into(),
                    answer: "Yes, the free tier is forever.".into(),
                },
                crate::types::FaqItem {
                    question: "Can I self-host?".into(),
                    answer: "Docker image available.".into(),
                },
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-faq\""));
        assert!(html.contains("<summary>Is it free?</summary>"));
        assert!(html.contains("<summary>Can I self-host?</summary>"));
        assert!(html.contains("class=\"faq-answer\""));
        assert!(html.contains("Yes, the free tier is forever."));
    }

    #[test]
    fn html_pricing_table() {
        let doc = doc_with(vec![Block::PricingTable {
            headers: vec!["".into(), "Free".into(), "Pro".into()],
            rows: vec![
                vec!["Price".into(), "$0".into(), "$9/mo".into()],
                vec!["Storage".into(), "1GB".into(), "100GB".into()],
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-pricing\""));
        assert!(html.contains("<th scope=\"col\">Free</th>"));
        assert!(html.contains("<th scope=\"col\">Pro</th>"));
        assert!(html.contains("<td>$9/mo</td>"));
    }

    #[test]
    fn html_faq_escapes_xss() {
        let doc = doc_with(vec![Block::Faq {
            items: vec![crate::types::FaqItem {
                question: "<script>alert('q')</script>".into(),
                answer: "<img onerror=alert(1)>".into(),
            }],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn html_site_hidden() {
        let doc = doc_with(vec![Block::Site {
            domain: Some("notesurf.io".into()),
            properties: vec![
                crate::types::StyleProperty { key: "name".into(), value: "NoteSurf".into() },
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-site\""));
        assert!(html.contains("data-domain=\"notesurf.io\""));
    }

    #[test]
    fn html_page_hero_layout() {
        let doc = doc_with(vec![Block::Page {
            route: "/".into(),
            layout: Some("hero".into()),
            title: None,
            sidebar: false,
            content: "# Welcome".into(),
            children: vec![
                Block::Markdown {
                    content: "# Welcome".into(),
                    span: span(),
                },
                Block::Cta {
                    label: "Get Started".into(),
                    href: "/signup".into(),
                    primary: true,
                    span: span(),
                },
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("class=\"surfdoc-page\""));
        assert!(html.contains("data-layout=\"hero\""));
        assert!(html.contains("Get Started")); // CTA rendered
        assert!(html.contains("surfdoc-cta")); // CTA has class
    }

    #[test]
    fn html_page_renders_children() {
        let doc = doc_with(vec![Block::Page {
            route: "/pricing".into(),
            layout: None,
            title: Some("Pricing".into()),
            sidebar: false,
            content: String::new(),
            children: vec![
                Block::Markdown {
                    content: "# Pricing".into(),
                    span: span(),
                },
                Block::HeroImage {
                    src: "pricing.png".into(),
                    alt: Some("Plans".into()),
                    span: span(),
                },
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("<section class=\"surfdoc-page\" aria-label=\"Pricing\">"));
        assert!(html.contains("<h1>Pricing</h1>")); // Markdown rendered
        assert!(html.contains("surfdoc-hero-image")); // Hero image rendered
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

    // -- ARIA accessibility tests -----------------------------------------

    #[test]
    fn aria_callout_danger_role_alert() {
        let doc = doc_with(vec![Block::Callout {
            callout_type: CalloutType::Danger,
            title: None,
            content: "Critical error.".into(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("role=\"alert\""));
    }

    #[test]
    fn aria_callout_info_role_note() {
        let doc = doc_with(vec![Block::Callout {
            callout_type: CalloutType::Info,
            title: None,
            content: "FYI.".into(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("role=\"note\""));
    }

    #[test]
    fn aria_data_table_scope_col() {
        let doc = doc_with(vec![Block::Data {
            id: None,
            format: DataFormat::Table,
            sortable: false,
            headers: vec!["Col1".into()],
            rows: vec![],
            raw_content: String::new(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("scope=\"col\""));
    }

    #[test]
    fn aria_code_label() {
        let doc = doc_with(vec![Block::Code {
            lang: Some("python".into()),
            file: None,
            highlight: vec![],
            content: "print()".into(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("aria-label=\"python code\""));
    }

    #[test]
    fn aria_tasks_label_wraps_checkbox() {
        let doc = doc_with(vec![Block::Tasks {
            items: vec![TaskItem {
                done: false,
                text: "Do thing".into(),
                assignee: None,
            }],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("<label><input type=\"checkbox\" disabled> Do thing</label>"));
    }

    #[test]
    fn aria_decision_role_note() {
        let doc = doc_with(vec![Block::Decision {
            status: DecisionStatus::Accepted,
            date: None,
            deciders: vec![],
            content: "We decided.".into(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("role=\"note\""));
        assert!(html.contains("aria-label=\"Decision: accepted\""));
    }

    #[test]
    fn aria_metric_group_label() {
        let doc = doc_with(vec![Block::Metric {
            label: "MRR".into(),
            value: "$5K".into(),
            trend: Some(Trend::Up),
            unit: Some("USD".into()),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("role=\"group\""));
        assert!(html.contains("aria-label=\"MRR: $5K USD, trending up\""));
    }

    #[test]
    fn aria_summary_doc_abstract() {
        let doc = doc_with(vec![Block::Summary {
            content: "TL;DR.".into(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("role=\"doc-abstract\""));
    }

    #[test]
    fn aria_tabs_tablist_pattern() {
        let doc = doc_with(vec![Block::Tabs {
            tabs: vec![
                TabPanel { label: "A".into(), content: "First.".into() },
                TabPanel { label: "B".into(), content: "Second.".into() },
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("role=\"tablist\""));
        assert!(html.contains("role=\"tab\""));
        assert!(html.contains("role=\"tabpanel\""));
        assert!(html.contains("aria-selected=\"true\""));
        assert!(html.contains("aria-selected=\"false\""));
        assert!(html.contains("aria-controls=\"surfdoc-panel-0\""));
        assert!(html.contains("aria-labelledby=\"surfdoc-tab-0\""));
    }

    #[test]
    fn aria_hero_image_role_img() {
        let doc = doc_with(vec![Block::HeroImage {
            src: "hero.png".into(),
            alt: Some("Product shot".into()),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("role=\"img\""));
        assert!(html.contains("aria-label=\"Product shot\""));
    }

    #[test]
    fn aria_testimonial_role_figure() {
        let doc = doc_with(vec![Block::Testimonial {
            content: "Great!".into(),
            author: Some("Ada".into()),
            role: None,
            company: None,
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("role=\"figure\""));
        assert!(html.contains("aria-label=\"Testimonial from Ada\""));
    }

    #[test]
    fn aria_style_hidden() {
        let doc = doc_with(vec![Block::Style {
            properties: vec![],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("aria-hidden=\"true\""));
    }

    #[test]
    fn aria_site_hidden() {
        let doc = doc_with(vec![Block::Site {
            domain: None,
            properties: vec![],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("aria-hidden=\"true\""));
    }

    #[test]
    fn aria_page_label_from_title() {
        let doc = doc_with(vec![Block::Page {
            route: "/about".into(),
            layout: None,
            title: Some("About Us".into()),
            sidebar: false,
            content: String::new(),
            children: vec![],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("aria-label=\"About Us\""));
    }

    #[test]
    fn aria_page_label_from_route() {
        let doc = doc_with(vec![Block::Page {
            route: "/pricing".into(),
            layout: None,
            title: None,
            sidebar: false,
            content: String::new(),
            children: vec![],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("aria-label=\"Page: /pricing\""));
    }

    #[test]
    fn aria_unknown_role_note() {
        let doc = doc_with(vec![Block::Unknown {
            name: "custom".into(),
            attrs: Default::default(),
            content: "stuff".into(),
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("role=\"note\""));
    }

    #[test]
    fn aria_pricing_table_scope() {
        let doc = doc_with(vec![Block::PricingTable {
            headers: vec!["".into(), "Basic".into()],
            rows: vec![vec!["Price".into(), "$0".into()]],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("scope=\"col\""));
        assert!(html.contains("aria-label=\"Pricing comparison\""));
    }

    #[test]
    fn aria_columns_role_group() {
        let doc = doc_with(vec![Block::Columns {
            columns: vec![
                ColumnContent { content: "A".into() },
                ColumnContent { content: "B".into() },
            ],
            span: span(),
        }]);
        let html = to_html(&doc);
        assert!(html.contains("role=\"group\""));
    }

    // -- extract_site() unit tests -----------------------------------------

    #[test]
    fn extract_site_separates_blocks() {
        let doc = doc_with(vec![
            Block::Site {
                domain: Some("example.com".into()),
                properties: vec![
                    StyleProperty { key: "name".into(), value: "My Site".into() },
                    StyleProperty { key: "accent".into(), value: "#ff0000".into() },
                ],
                span: span(),
            },
            Block::Markdown {
                content: "Loose block".into(),
                span: span(),
            },
            Block::Page {
                route: "/".into(),
                layout: Some("hero".into()),
                title: Some("Home".into()),
                sidebar: false,
                content: "# Welcome".into(),
                children: vec![Block::Markdown {
                    content: "# Welcome".into(),
                    span: span(),
                }],
                span: span(),
            },
            Block::Page {
                route: "/about".into(),
                layout: None,
                title: Some("About".into()),
                sidebar: false,
                content: "# About".into(),
                children: vec![Block::Markdown {
                    content: "# About".into(),
                    span: span(),
                }],
                span: span(),
            },
        ]);

        let (site, pages, loose) = extract_site(&doc);

        // Site config extracted
        let site = site.expect("should have site config");
        assert_eq!(site.domain.as_deref(), Some("example.com"));
        assert_eq!(site.name.as_deref(), Some("My Site"));
        assert_eq!(site.accent.as_deref(), Some("#ff0000"));

        // Pages extracted
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].route, "/");
        assert_eq!(pages[0].title.as_deref(), Some("Home"));
        assert_eq!(pages[1].route, "/about");

        // Loose blocks
        assert_eq!(loose.len(), 1);
    }

    #[test]
    fn extract_site_no_site_block() {
        let doc = doc_with(vec![
            Block::Markdown {
                content: "Just markdown".into(),
                span: span(),
            },
        ]);

        let (site, pages, loose) = extract_site(&doc);
        assert!(site.is_none());
        assert!(pages.is_empty());
        assert_eq!(loose.len(), 1);
    }

    #[test]
    fn extract_site_config_fields() {
        let doc = doc_with(vec![Block::Site {
            domain: Some("test.io".into()),
            properties: vec![
                StyleProperty { key: "name".into(), value: "Test".into() },
                StyleProperty { key: "tagline".into(), value: "A tagline".into() },
                StyleProperty { key: "theme".into(), value: "dark".into() },
                StyleProperty { key: "accent".into(), value: "#00ff00".into() },
                StyleProperty { key: "font".into(), value: "inter".into() },
                StyleProperty { key: "custom".into(), value: "value".into() },
            ],
            span: span(),
        }]);

        let (site, _, _) = extract_site(&doc);
        let site = site.unwrap();
        assert_eq!(site.name.as_deref(), Some("Test"));
        assert_eq!(site.tagline.as_deref(), Some("A tagline"));
        assert_eq!(site.theme.as_deref(), Some("dark"));
        assert_eq!(site.accent.as_deref(), Some("#00ff00"));
        assert_eq!(site.font.as_deref(), Some("inter"));
        assert_eq!(site.properties.len(), 6); // all properties preserved
    }

    // -- render_site_page() unit tests ------------------------------------

    #[test]
    fn render_site_page_produces_valid_html() {
        let site = SiteConfig {
            name: Some("Test Site".into()),
            accent: Some("#3b82f6".into()),
            ..Default::default()
        };
        let page = PageEntry {
            route: "/".into(),
            layout: None,
            title: Some("Home".into()),
            sidebar: false,
            children: vec![Block::Markdown {
                content: "# Hello World".into(),
                span: span(),
            }],
        };
        let nav_items = vec![
            ("/".into(), "Home".into()),
            ("/about".into(), "About".into()),
        ];
        let config = PageConfig::default();

        let html = render_site_page(&page, &site, &nav_items, &config);

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("surfdoc-site-nav"));
        assert!(html.contains("Test Site"));
        assert!(html.contains("Hello World"));
        assert!(html.contains("surfdoc-site-footer"));
        assert!(html.contains("#3b82f6")); // accent override
    }

    #[test]
    fn render_site_page_has_nav_links() {
        let site = SiteConfig {
            name: Some("Nav Test".into()),
            ..Default::default()
        };
        let page = PageEntry {
            route: "/about".into(),
            layout: None,
            title: Some("About".into()),
            sidebar: false,
            children: vec![],
        };
        let nav_items = vec![
            ("/".into(), "Home".into()),
            ("/about".into(), "About".into()),
            ("/pricing".into(), "Pricing".into()),
        ];
        let config = PageConfig::default();

        let html = render_site_page(&page, &site, &nav_items, &config);

        assert!(html.contains("/index.html"));
        assert!(html.contains("/about/index.html"));
        assert!(html.contains("/pricing/index.html"));
        // Active link for about page
        assert!(html.contains("class=\"active\">About</a>"));
    }

    #[test]
    fn render_site_page_title_format() {
        let site = SiteConfig {
            name: Some("My Site".into()),
            ..Default::default()
        };

        // Page with title
        let page = PageEntry {
            route: "/about".into(),
            layout: None,
            title: Some("About Us".into()),
            sidebar: false,
            children: vec![],
        };
        let html = render_site_page(&page, &site, &[], &PageConfig::default());
        assert!(html.contains("<title>About Us — My Site</title>"));

        // Home page without title
        let home = PageEntry {
            route: "/".into(),
            layout: None,
            title: None,
            sidebar: false,
            children: vec![],
        };
        let html = render_site_page(&home, &site, &[], &PageConfig::default());
        assert!(html.contains("<title>My Site</title>"));
    }
}
