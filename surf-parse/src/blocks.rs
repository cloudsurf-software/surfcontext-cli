//! Per-block-type content parsers (Pass 2 resolution).
//!
//! `resolve_block` converts a `Block::Unknown` into a typed variant based on
//! the block name. Unknown block names pass through unchanged.

use crate::types::{
    AttrValue, Attrs, Block, CalloutType, ColumnContent, DataFormat, DecisionStatus, Span,
    TabPanel, TaskItem, Trend,
};

/// Resolve a `Block::Unknown` into a typed variant, if the name matches a known
/// block type. Unrecognised names are returned unchanged.
pub fn resolve_block(block: Block) -> Block {
    let Block::Unknown {
        name,
        attrs,
        content,
        span,
    } = &block
    else {
        return block;
    };

    match name.as_str() {
        "callout" => parse_callout(attrs, content, *span),
        "data" => parse_data(attrs, content, *span),
        "code" => parse_code(attrs, content, *span),
        "tasks" => parse_tasks(content, *span),
        "decision" => parse_decision(attrs, content, *span),
        "metric" => parse_metric(attrs, *span),
        "summary" => parse_summary(content, *span),
        "figure" => parse_figure(attrs, *span),
        "tabs" => parse_tabs(content, *span),
        "columns" => parse_columns(content, *span),
        "quote" => parse_quote(attrs, content, *span),
        _ => block,
    }
}

// ------------------------------------------------------------------
// Attribute extraction helpers
// ------------------------------------------------------------------

fn attr_string(attrs: &Attrs, key: &str) -> Option<String> {
    attrs.get(key).and_then(|v| match v {
        AttrValue::String(s) => Some(s.clone()),
        AttrValue::Number(n) => Some(n.to_string()),
        AttrValue::Bool(b) => Some(b.to_string()),
        AttrValue::Null => None,
    })
}

fn attr_bool(attrs: &Attrs, key: &str) -> bool {
    attrs
        .get(key)
        .is_some_and(|v| matches!(v, AttrValue::Bool(true)))
}

// ------------------------------------------------------------------
// Per-block parsers
// ------------------------------------------------------------------

fn parse_callout(attrs: &Attrs, content: &str, span: Span) -> Block {
    let callout_type = attr_string(attrs, "type")
        .and_then(|s| match s.as_str() {
            "info" => Some(CalloutType::Info),
            "warning" => Some(CalloutType::Warning),
            "danger" => Some(CalloutType::Danger),
            "tip" => Some(CalloutType::Tip),
            "note" => Some(CalloutType::Note),
            "success" => Some(CalloutType::Success),
            _ => None,
        })
        .unwrap_or(CalloutType::Info);

    let title = attr_string(attrs, "title");

    Block::Callout {
        callout_type,
        title,
        content: content.to_string(),
        span,
    }
}

fn parse_data(attrs: &Attrs, content: &str, span: Span) -> Block {
    let id = attr_string(attrs, "id");
    let sortable = attr_bool(attrs, "sortable");

    let format = attr_string(attrs, "format")
        .and_then(|s| match s.as_str() {
            "table" => Some(DataFormat::Table),
            "csv" => Some(DataFormat::Csv),
            "json" => Some(DataFormat::Json),
            _ => None,
        })
        .unwrap_or(DataFormat::Table);

    let (headers, rows) = match format {
        DataFormat::Table => parse_table_content(content),
        DataFormat::Csv => parse_csv_content(content),
        DataFormat::Json => (Vec::new(), Vec::new()),
    };

    Block::Data {
        id,
        format,
        sortable,
        headers,
        rows,
        raw_content: content.to_string(),
        span,
    }
}

/// Parse pipe-delimited table content.
///
/// First non-empty line is headers. Lines that look like `|---|---|` are
/// separator rows and get skipped. Remaining lines are data rows.
fn parse_table_content(content: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let mut headers = Vec::new();
    let mut rows = Vec::new();
    let mut header_done = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Skip separator lines like |---|---| or | --- | --- |
        if is_table_separator(trimmed) {
            continue;
        }

        let cells: Vec<String> = split_pipe_row(trimmed);

        if !header_done {
            headers = cells;
            header_done = true;
        } else {
            rows.push(cells);
        }
    }

    (headers, rows)
}

/// Check whether a line is a markdown table separator (e.g. `|---|---|`).
fn is_table_separator(line: &str) -> bool {
    let stripped = line.trim().trim_matches('|').trim();
    if stripped.is_empty() {
        return false;
    }
    stripped
        .split('|')
        .all(|cell| cell.trim().chars().all(|c| c == '-' || c == ':'))
}

/// Split a pipe-delimited row into trimmed cell strings, stripping leading and
/// trailing pipes.
fn split_pipe_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    // Remove leading/trailing pipes.
    let inner = trimmed
        .strip_prefix('|')
        .unwrap_or(trimmed);
    let inner = inner
        .strip_suffix('|')
        .unwrap_or(inner);
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

/// Parse CSV content: newline-delimited, comma-separated.
fn parse_csv_content(content: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let mut headers = Vec::new();
    let mut rows = Vec::new();
    let mut header_done = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let cells: Vec<String> = trimmed.split(',').map(|c| c.trim().to_string()).collect();

        if !header_done {
            headers = cells;
            header_done = true;
        } else {
            rows.push(cells);
        }
    }

    (headers, rows)
}

fn parse_code(attrs: &Attrs, content: &str, span: Span) -> Block {
    let lang = attr_string(attrs, "lang");
    let file = attr_string(attrs, "file");
    let highlight = attr_string(attrs, "highlight")
        .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
        .unwrap_or_default();

    Block::Code {
        lang,
        file,
        highlight,
        content: content.to_string(),
        span,
    }
}

fn parse_tasks(content: &str, span: Span) -> Block {
    let mut items = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        let (done, rest) = if let Some(rest) = trimmed.strip_prefix("- [x] ") {
            (true, rest)
        } else if let Some(rest) = trimmed.strip_prefix("- [X] ") {
            (true, rest)
        } else if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            (false, rest)
        } else {
            continue;
        };

        // Extract optional @assignee from end of text.
        let (text, assignee) = extract_assignee(rest);

        items.push(TaskItem {
            done,
            text,
            assignee,
        });
    }

    Block::Tasks { items, span }
}

/// Extract a trailing `@username` from the end of a task text.
///
/// Returns `(text_without_assignee, Option<assignee>)`.
fn extract_assignee(text: &str) -> (String, Option<String>) {
    let trimmed = text.trim_end();
    if let Some(at_pos) = trimmed.rfind(" @") {
        let candidate = &trimmed[at_pos + 2..];
        // Assignee must be a single word (no spaces).
        if !candidate.is_empty() && !candidate.contains(' ') {
            let main_text = trimmed[..at_pos].trim_end().to_string();
            return (main_text, Some(candidate.to_string()));
        }
    }
    (text.to_string(), None)
}

fn parse_decision(attrs: &Attrs, content: &str, span: Span) -> Block {
    let status = attr_string(attrs, "status")
        .and_then(|s| match s.as_str() {
            "proposed" => Some(DecisionStatus::Proposed),
            "accepted" => Some(DecisionStatus::Accepted),
            "rejected" => Some(DecisionStatus::Rejected),
            "superseded" => Some(DecisionStatus::Superseded),
            _ => None,
        })
        .unwrap_or(DecisionStatus::Proposed);

    let date = attr_string(attrs, "date");

    let deciders = attr_string(attrs, "deciders")
        .map(|s| s.split(',').map(|d| d.trim().to_string()).collect())
        .unwrap_or_default();

    Block::Decision {
        status,
        date,
        deciders,
        content: content.to_string(),
        span,
    }
}

fn parse_metric(attrs: &Attrs, span: Span) -> Block {
    let label = attr_string(attrs, "label").unwrap_or_default();
    let value = attr_string(attrs, "value").unwrap_or_default();

    let trend = attr_string(attrs, "trend").and_then(|s| match s.as_str() {
        "up" => Some(Trend::Up),
        "down" => Some(Trend::Down),
        "flat" => Some(Trend::Flat),
        _ => None,
    });

    let unit = attr_string(attrs, "unit");

    Block::Metric {
        label,
        value,
        trend,
        unit,
        span,
    }
}

fn parse_summary(content: &str, span: Span) -> Block {
    Block::Summary {
        content: content.to_string(),
        span,
    }
}

fn parse_tabs(content: &str, span: Span) -> Block {
    let mut tabs = Vec::new();
    let mut current_label: Option<String> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        // Tab labels: `## Label` or `### Label` inside tabs block
        if let Some(rest) = trimmed.strip_prefix("## ") {
            // Flush previous tab
            if let Some(label) = current_label.take() {
                tabs.push(TabPanel {
                    label,
                    content: current_lines.join("\n").trim().to_string(),
                });
                current_lines.clear();
            }
            current_label = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            if let Some(label) = current_label.take() {
                tabs.push(TabPanel {
                    label,
                    content: current_lines.join("\n").trim().to_string(),
                });
                current_lines.clear();
            }
            current_label = Some(rest.trim().to_string());
        } else {
            current_lines.push(line);
        }
    }

    // Flush final tab
    if let Some(label) = current_label {
        tabs.push(TabPanel {
            label,
            content: current_lines.join("\n").trim().to_string(),
        });
    } else if !current_lines.is_empty() {
        // No headers found — single unnamed tab
        let text = current_lines.join("\n").trim().to_string();
        if !text.is_empty() {
            tabs.push(TabPanel {
                label: "Tab 1".to_string(),
                content: text,
            });
        }
    }

    Block::Tabs { tabs, span }
}

fn parse_columns(content: &str, span: Span) -> Block {
    let mut columns = Vec::new();
    let mut current_lines: Vec<&str> = Vec::new();
    let mut found_separator = false;

    for line in content.lines() {
        let trimmed = line.trim();
        // Nested :::column directives serve as separators
        if trimmed.starts_with(":::column") {
            if !current_lines.is_empty() {
                columns.push(ColumnContent {
                    content: current_lines.join("\n").trim().to_string(),
                });
                current_lines.clear();
            }
            found_separator = true;
        } else if trimmed == ":::" {
            // Close a :::column — flush content
            if found_separator {
                columns.push(ColumnContent {
                    content: current_lines.join("\n").trim().to_string(),
                });
                current_lines.clear();
            }
        } else if trimmed == "---" && !found_separator {
            // Horizontal rule as column separator (simpler syntax)
            columns.push(ColumnContent {
                content: current_lines.join("\n").trim().to_string(),
            });
            current_lines.clear();
            found_separator = true;
        } else {
            current_lines.push(line);
        }
    }

    // Flush remaining content
    let remaining = current_lines.join("\n").trim().to_string();
    if !remaining.is_empty() {
        columns.push(ColumnContent {
            content: remaining,
        });
    }

    // If no separators were found, treat the whole thing as one column
    if columns.is_empty() {
        columns.push(ColumnContent {
            content: content.trim().to_string(),
        });
    }

    Block::Columns { columns, span }
}

fn parse_quote(attrs: &Attrs, content: &str, span: Span) -> Block {
    let attribution = attr_string(attrs, "by")
        .or_else(|| attr_string(attrs, "attribution"))
        .or_else(|| attr_string(attrs, "author"));
    let cite = attr_string(attrs, "cite")
        .or_else(|| attr_string(attrs, "source"));

    Block::Quote {
        content: content.to_string(),
        attribution,
        cite,
        span,
    }
}

fn parse_figure(attrs: &Attrs, span: Span) -> Block {
    let src = attr_string(attrs, "src").unwrap_or_default();
    let caption = attr_string(attrs, "caption");
    let alt = attr_string(attrs, "alt");
    let width = attr_string(attrs, "width");

    Block::Figure {
        src,
        caption,
        alt,
        width,
        span,
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AttrValue;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;

    /// Helper: build a `Block::Unknown` for testing.
    fn unknown(name: &str, attrs: Attrs, content: &str) -> Block {
        Block::Unknown {
            name: name.to_string(),
            attrs,
            content: content.to_string(),
            span: Span {
                start_line: 1,
                end_line: 3,
                start_offset: 0,
                end_offset: 100,
            },
        }
    }

    /// Helper: quick attrs builder.
    fn attrs(pairs: &[(&str, AttrValue)]) -> Attrs {
        let mut map = BTreeMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.clone());
        }
        map
    }

    // -- Callout ---------------------------------------------------

    #[test]
    fn resolve_callout_warning() {
        let block = unknown(
            "callout",
            attrs(&[("type", AttrValue::String("warning".into()))]),
            "Watch out!",
        );
        match resolve_block(block) {
            Block::Callout {
                callout_type,
                content,
                ..
            } => {
                assert_eq!(callout_type, CalloutType::Warning);
                assert_eq!(content, "Watch out!");
            }
            other => panic!("Expected Callout, got {other:?}"),
        }
    }

    #[test]
    fn resolve_callout_with_title() {
        let block = unknown(
            "callout",
            attrs(&[
                ("type", AttrValue::String("tip".into())),
                ("title", AttrValue::String("Pro Tip".into())),
            ]),
            "Use Rust.",
        );
        match resolve_block(block) {
            Block::Callout {
                callout_type,
                title,
                ..
            } => {
                assert_eq!(callout_type, CalloutType::Tip);
                assert_eq!(title, Some("Pro Tip".to_string()));
            }
            other => panic!("Expected Callout, got {other:?}"),
        }
    }

    #[test]
    fn resolve_callout_default_type() {
        let block = unknown("callout", Attrs::new(), "No type attr.");
        match resolve_block(block) {
            Block::Callout { callout_type, .. } => {
                assert_eq!(callout_type, CalloutType::Info);
            }
            other => panic!("Expected Callout, got {other:?}"),
        }
    }

    // -- Data ------------------------------------------------------

    #[test]
    fn resolve_data_table() {
        let content = "| Name | Age |\n|---|---|\n| Alice | 30 |\n| Bob | 25 |";
        let block = unknown("data", Attrs::new(), content);
        match resolve_block(block) {
            Block::Data {
                headers,
                rows,
                format,
                ..
            } => {
                assert_eq!(format, DataFormat::Table);
                assert_eq!(headers, vec!["Name", "Age"]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0], vec!["Alice", "30"]);
                assert_eq!(rows[1], vec!["Bob", "25"]);
            }
            other => panic!("Expected Data, got {other:?}"),
        }
    }

    #[test]
    fn resolve_data_with_separator() {
        let content = "| H1 | H2 |\n| --- | --- |\n| v1 | v2 |";
        let block = unknown("data", Attrs::new(), content);
        match resolve_block(block) {
            Block::Data { headers, rows, .. } => {
                assert_eq!(headers, vec!["H1", "H2"]);
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0], vec!["v1", "v2"]);
            }
            other => panic!("Expected Data, got {other:?}"),
        }
    }

    #[test]
    fn resolve_data_sortable() {
        let block = unknown(
            "data",
            attrs(&[("sortable", AttrValue::Bool(true))]),
            "| A |\n| 1 |",
        );
        match resolve_block(block) {
            Block::Data { sortable, .. } => {
                assert!(sortable);
            }
            other => panic!("Expected Data, got {other:?}"),
        }
    }

    #[test]
    fn resolve_data_csv() {
        let content = "Name, Age\nAlice, 30\nBob, 25";
        let block = unknown(
            "data",
            attrs(&[("format", AttrValue::String("csv".into()))]),
            content,
        );
        match resolve_block(block) {
            Block::Data {
                format,
                headers,
                rows,
                ..
            } => {
                assert_eq!(format, DataFormat::Csv);
                assert_eq!(headers, vec!["Name", "Age"]);
                assert_eq!(rows.len(), 2);
            }
            other => panic!("Expected Data, got {other:?}"),
        }
    }

    // -- Code ------------------------------------------------------

    #[test]
    fn resolve_code_with_lang() {
        let block = unknown(
            "code",
            attrs(&[("lang", AttrValue::String("rust".into()))]),
            "fn main() {}",
        );
        match resolve_block(block) {
            Block::Code { lang, content, .. } => {
                assert_eq!(lang, Some("rust".to_string()));
                assert_eq!(content, "fn main() {}");
            }
            other => panic!("Expected Code, got {other:?}"),
        }
    }

    #[test]
    fn resolve_code_with_file() {
        let block = unknown(
            "code",
            attrs(&[
                ("lang", AttrValue::String("rust".into())),
                ("file", AttrValue::String("main.rs".into())),
            ]),
            "fn main() {}",
        );
        match resolve_block(block) {
            Block::Code { lang, file, .. } => {
                assert_eq!(lang, Some("rust".to_string()));
                assert_eq!(file, Some("main.rs".to_string()));
            }
            other => panic!("Expected Code, got {other:?}"),
        }
    }

    // -- Tasks -----------------------------------------------------

    #[test]
    fn resolve_tasks_mixed() {
        let content = "- [ ] Write tests\n- [x] Write parser";
        let block = unknown("tasks", Attrs::new(), content);
        match resolve_block(block) {
            Block::Tasks { items, .. } => {
                assert_eq!(items.len(), 2);
                assert!(!items[0].done);
                assert_eq!(items[0].text, "Write tests");
                assert!(items[1].done);
                assert_eq!(items[1].text, "Write parser");
            }
            other => panic!("Expected Tasks, got {other:?}"),
        }
    }

    #[test]
    fn resolve_tasks_with_assignee() {
        let content = "- [ ] Fix bug @brady";
        let block = unknown("tasks", Attrs::new(), content);
        match resolve_block(block) {
            Block::Tasks { items, .. } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].text, "Fix bug");
                assert_eq!(items[0].assignee, Some("brady".to_string()));
            }
            other => panic!("Expected Tasks, got {other:?}"),
        }
    }

    // -- Decision --------------------------------------------------

    #[test]
    fn resolve_decision_accepted() {
        let block = unknown(
            "decision",
            attrs(&[
                ("status", AttrValue::String("accepted".into())),
                ("date", AttrValue::String("2026-02-10".into())),
            ]),
            "We chose Rust.",
        );
        match resolve_block(block) {
            Block::Decision {
                status,
                date,
                content,
                ..
            } => {
                assert_eq!(status, DecisionStatus::Accepted);
                assert_eq!(date, Some("2026-02-10".to_string()));
                assert_eq!(content, "We chose Rust.");
            }
            other => panic!("Expected Decision, got {other:?}"),
        }
    }

    #[test]
    fn resolve_decision_with_deciders() {
        let block = unknown(
            "decision",
            attrs(&[
                ("status", AttrValue::String("proposed".into())),
                ("deciders", AttrValue::String("Brady, Claude".into())),
            ]),
            "Consider options.",
        );
        match resolve_block(block) {
            Block::Decision { deciders, .. } => {
                assert_eq!(deciders, vec!["Brady", "Claude"]);
            }
            other => panic!("Expected Decision, got {other:?}"),
        }
    }

    // -- Metric ----------------------------------------------------

    #[test]
    fn resolve_metric_basic() {
        let block = unknown(
            "metric",
            attrs(&[
                ("label", AttrValue::String("MRR".into())),
                ("value", AttrValue::String("$2K".into())),
            ]),
            "",
        );
        match resolve_block(block) {
            Block::Metric { label, value, .. } => {
                assert_eq!(label, "MRR");
                assert_eq!(value, "$2K");
            }
            other => panic!("Expected Metric, got {other:?}"),
        }
    }

    #[test]
    fn resolve_metric_with_trend() {
        let block = unknown(
            "metric",
            attrs(&[
                ("label", AttrValue::String("Users".into())),
                ("value", AttrValue::String("500".into())),
                ("trend", AttrValue::String("up".into())),
            ]),
            "",
        );
        match resolve_block(block) {
            Block::Metric { trend, .. } => {
                assert_eq!(trend, Some(Trend::Up));
            }
            other => panic!("Expected Metric, got {other:?}"),
        }
    }

    // -- Summary ---------------------------------------------------

    #[test]
    fn resolve_summary() {
        let block = unknown("summary", Attrs::new(), "This is the executive summary.");
        match resolve_block(block) {
            Block::Summary { content, .. } => {
                assert_eq!(content, "This is the executive summary.");
            }
            other => panic!("Expected Summary, got {other:?}"),
        }
    }

    // -- Figure ----------------------------------------------------

    #[test]
    fn resolve_figure_basic() {
        let block = unknown(
            "figure",
            attrs(&[
                ("src", AttrValue::String("img.png".into())),
                ("caption", AttrValue::String("Photo".into())),
            ]),
            "",
        );
        match resolve_block(block) {
            Block::Figure {
                src,
                caption,
                alt,
                width,
                ..
            } => {
                assert_eq!(src, "img.png");
                assert_eq!(caption, Some("Photo".to_string()));
                assert!(alt.is_none());
                assert!(width.is_none());
            }
            other => panic!("Expected Figure, got {other:?}"),
        }
    }

    // -- Tabs ------------------------------------------------------

    #[test]
    fn resolve_tabs_with_headers() {
        let content = "## Overview\nIntro text.\n\n## Details\nTechnical info.\n\n## FAQ\nQ&A here.";
        let block = unknown("tabs", Attrs::new(), content);
        match resolve_block(block) {
            Block::Tabs { tabs, .. } => {
                assert_eq!(tabs.len(), 3);
                assert_eq!(tabs[0].label, "Overview");
                assert!(tabs[0].content.contains("Intro text."));
                assert_eq!(tabs[1].label, "Details");
                assert!(tabs[1].content.contains("Technical info."));
                assert_eq!(tabs[2].label, "FAQ");
                assert!(tabs[2].content.contains("Q&A here."));
            }
            other => panic!("Expected Tabs, got {other:?}"),
        }
    }

    #[test]
    fn resolve_tabs_single_no_header() {
        let content = "Just some text without any tab headers.";
        let block = unknown("tabs", Attrs::new(), content);
        match resolve_block(block) {
            Block::Tabs { tabs, .. } => {
                assert_eq!(tabs.len(), 1);
                assert_eq!(tabs[0].label, "Tab 1");
                assert!(tabs[0].content.contains("Just some text"));
            }
            other => panic!("Expected Tabs, got {other:?}"),
        }
    }

    // -- Columns ---------------------------------------------------

    #[test]
    fn resolve_columns_with_nested_directives() {
        let content = ":::column\nLeft content.\n:::\n:::column\nRight content.\n:::";
        let block = unknown("columns", Attrs::new(), content);
        match resolve_block(block) {
            Block::Columns { columns, .. } => {
                assert_eq!(columns.len(), 2);
                assert_eq!(columns[0].content, "Left content.");
                assert_eq!(columns[1].content, "Right content.");
            }
            other => panic!("Expected Columns, got {other:?}"),
        }
    }

    #[test]
    fn resolve_columns_with_hr_separator() {
        let content = "Left side.\n---\nRight side.";
        let block = unknown("columns", Attrs::new(), content);
        match resolve_block(block) {
            Block::Columns { columns, .. } => {
                assert_eq!(columns.len(), 2);
                assert_eq!(columns[0].content, "Left side.");
                assert_eq!(columns[1].content, "Right side.");
            }
            other => panic!("Expected Columns, got {other:?}"),
        }
    }

    #[test]
    fn resolve_columns_single() {
        let content = "All in one column.";
        let block = unknown("columns", Attrs::new(), content);
        match resolve_block(block) {
            Block::Columns { columns, .. } => {
                assert_eq!(columns.len(), 1);
                assert_eq!(columns[0].content, "All in one column.");
            }
            other => panic!("Expected Columns, got {other:?}"),
        }
    }

    // -- Quote -----------------------------------------------------

    #[test]
    fn resolve_quote_with_attribution() {
        let block = unknown(
            "quote",
            attrs(&[
                ("by", AttrValue::String("Alan Kay".into())),
                ("cite", AttrValue::String("ACM 1971".into())),
            ]),
            "The best way to predict the future is to invent it.",
        );
        match resolve_block(block) {
            Block::Quote {
                content,
                attribution,
                cite,
                ..
            } => {
                assert_eq!(content, "The best way to predict the future is to invent it.");
                assert_eq!(attribution, Some("Alan Kay".to_string()));
                assert_eq!(cite, Some("ACM 1971".to_string()));
            }
            other => panic!("Expected Quote, got {other:?}"),
        }
    }

    #[test]
    fn resolve_quote_no_attribution() {
        let block = unknown("quote", Attrs::new(), "Anonymous wisdom.");
        match resolve_block(block) {
            Block::Quote {
                content,
                attribution,
                ..
            } => {
                assert_eq!(content, "Anonymous wisdom.");
                assert!(attribution.is_none());
            }
            other => panic!("Expected Quote, got {other:?}"),
        }
    }

    #[test]
    fn resolve_quote_author_alias() {
        let block = unknown(
            "quote",
            attrs(&[("author", AttrValue::String("Knuth".into()))]),
            "Premature optimization.",
        );
        match resolve_block(block) {
            Block::Quote { attribution, .. } => {
                assert_eq!(attribution, Some("Knuth".to_string()));
            }
            other => panic!("Expected Quote, got {other:?}"),
        }
    }

    // -- Passthrough -----------------------------------------------

    #[test]
    fn resolve_unknown_passthrough() {
        let block = unknown("custom_block", Attrs::new(), "whatever");
        match resolve_block(block) {
            Block::Unknown { name, .. } => {
                assert_eq!(name, "custom_block");
            }
            other => panic!("Expected Unknown passthrough, got {other:?}"),
        }
    }
}
