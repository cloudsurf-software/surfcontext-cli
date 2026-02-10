//! Markdown degradation renderer.
//!
//! Converts a `SurfDoc` into standard CommonMark with no `::` directive markers.
//! Each block type is degraded to the nearest Markdown equivalent.

use crate::types::{Block, CalloutType, DecisionStatus, SurfDoc, Trend};

/// Render a `SurfDoc` as standard CommonMark markdown.
///
/// The output contains no `::` directive markers. Each SurfDoc block type is
/// degraded to its closest CommonMark equivalent.
pub fn to_markdown(doc: &SurfDoc) -> String {
    let mut parts: Vec<String> = Vec::new();

    for block in &doc.blocks {
        parts.push(render_block(block));
    }

    parts.join("\n\n")
}

fn render_block(block: &Block) -> String {
    match block {
        Block::Markdown { content, .. } => content.clone(),

        Block::Callout {
            callout_type,
            title,
            content,
            ..
        } => {
            let type_label = callout_type_label(*callout_type);
            let prefix = match title {
                Some(t) => format!("**{type_label}**: {t}"),
                None => format!("**{type_label}**"),
            };
            let mut lines = vec![format!("> {prefix}")];
            for line in content.lines() {
                lines.push(format!("> {line}"));
            }
            lines.join("\n")
        }

        Block::Data {
            headers, rows, ..
        } => {
            if headers.is_empty() {
                return String::new();
            }
            let mut lines = Vec::new();
            // Header row
            lines.push(format!("| {} |", headers.join(" | ")));
            // Separator
            let sep: Vec<&str> = headers.iter().map(|_| "---").collect();
            lines.push(format!("| {} |", sep.join(" | ")));
            // Data rows
            for row in rows {
                lines.push(format!("| {} |", row.join(" | ")));
            }
            lines.join("\n")
        }

        Block::Code {
            lang, content, ..
        } => {
            let lang_tag = lang.as_deref().unwrap_or("");
            format!("```{lang_tag}\n{content}\n```")
        }

        Block::Tasks { items, .. } => {
            let lines: Vec<String> = items
                .iter()
                .map(|item| {
                    let check = if item.done { "x" } else { " " };
                    match &item.assignee {
                        Some(a) => format!("- [{check}] {} @{a}", item.text),
                        None => format!("- [{check}] {}", item.text),
                    }
                })
                .collect();
            lines.join("\n")
        }

        Block::Decision {
            status,
            date,
            content,
            ..
        } => {
            let status_label = decision_status_label(*status);
            let date_part = match date {
                Some(d) => format!(" ({d})"),
                None => String::new(),
            };
            let mut lines = vec![format!("> **Decision** ({status_label}){date_part}")];
            for line in content.lines() {
                lines.push(format!("> {line}"));
            }
            lines.join("\n")
        }

        Block::Metric {
            label,
            value,
            trend,
            unit,
            ..
        } => {
            let trend_arrow = match trend {
                Some(Trend::Up) => " \u{2191}",
                Some(Trend::Down) => " \u{2193}",
                Some(Trend::Flat) => " \u{2192}",
                None => "",
            };
            let unit_part = match unit {
                Some(u) => format!(" {u}"),
                None => String::new(),
            };
            format!("**{label}**: {value}{unit_part}{trend_arrow}")
        }

        Block::Summary { content, .. } => {
            let lines: Vec<String> = content.lines().map(|l| format!("> *{l}*")).collect();
            lines.join("\n")
        }

        Block::Figure {
            src,
            caption,
            alt,
            ..
        } => {
            let alt_text = alt.as_deref().unwrap_or("");
            let img = format!("![{alt_text}]({src})");
            match caption {
                Some(c) => format!("{img}\n*{c}*"),
                None => img,
            }
        }

        Block::Tabs { tabs, .. } => {
            let parts: Vec<String> = tabs
                .iter()
                .map(|tab| format!("### {}\n\n{}", tab.label, tab.content))
                .collect();
            parts.join("\n\n")
        }

        Block::Columns { columns, .. } => {
            let parts: Vec<String> = columns
                .iter()
                .map(|col| col.content.clone())
                .collect();
            parts.join("\n\n---\n\n")
        }

        Block::Quote {
            content,
            attribution,
            ..
        } => {
            let mut lines: Vec<String> = content.lines().map(|l| format!("> {l}")).collect();
            if let Some(attr) = attribution {
                lines.push(format!(">\n> \u{2014} {attr}"));
            }
            lines.join("\n")
        }

        Block::Unknown {
            name,
            content,
            ..
        } => {
            let mut lines = Vec::new();
            lines.push(format!("<!-- ::{name} -->"));
            if !content.is_empty() {
                lines.push(content.clone());
            }
            lines.push("<!-- :: -->".to_string());
            lines.join("\n")
        }
    }
}

fn callout_type_label(ct: CalloutType) -> &'static str {
    match ct {
        CalloutType::Info => "Info",
        CalloutType::Warning => "Warning",
        CalloutType::Danger => "Danger",
        CalloutType::Tip => "Tip",
        CalloutType::Note => "Note",
        CalloutType::Success => "Success",
    }
}

fn decision_status_label(ds: DecisionStatus) -> &'static str {
    match ds {
        DecisionStatus::Proposed => "proposed",
        DecisionStatus::Accepted => "accepted",
        DecisionStatus::Rejected => "rejected",
        DecisionStatus::Superseded => "superseded",
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
    fn md_callout_warning() {
        let doc = doc_with(vec![Block::Callout {
            callout_type: CalloutType::Warning,
            title: Some("Watch out".into()),
            content: "Sharp edges ahead.".into(),
            span: span(),
        }]);
        let md = to_markdown(&doc);
        assert!(md.contains("> **Warning**: Watch out"));
        assert!(md.contains("> Sharp edges ahead."));
    }

    #[test]
    fn md_data_table() {
        let doc = doc_with(vec![Block::Data {
            id: None,
            format: DataFormat::Table,
            sortable: false,
            headers: vec!["Name".into(), "Age".into()],
            rows: vec![vec!["Alice".into(), "30".into()]],
            raw_content: String::new(),
            span: span(),
        }]);
        let md = to_markdown(&doc);
        assert!(md.contains("| Name | Age |"));
        assert!(md.contains("| --- | --- |"));
        assert!(md.contains("| Alice | 30 |"));
    }

    #[test]
    fn md_code_block() {
        let doc = doc_with(vec![Block::Code {
            lang: Some("rust".into()),
            file: None,
            highlight: vec![],
            content: "fn main() {}".into(),
            span: span(),
        }]);
        let md = to_markdown(&doc);
        assert!(md.contains("```rust"));
        assert!(md.contains("fn main() {}"));
        assert!(md.contains("```"));
    }

    #[test]
    fn md_tasks() {
        let doc = doc_with(vec![Block::Tasks {
            items: vec![
                TaskItem {
                    done: false,
                    text: "Write tests".into(),
                    assignee: None,
                },
                TaskItem {
                    done: true,
                    text: "Write parser".into(),
                    assignee: Some("brady".into()),
                },
            ],
            span: span(),
        }]);
        let md = to_markdown(&doc);
        assert!(md.contains("- [ ] Write tests"));
        assert!(md.contains("- [x] Write parser @brady"));
    }

    #[test]
    fn md_decision() {
        let doc = doc_with(vec![Block::Decision {
            status: DecisionStatus::Accepted,
            date: Some("2026-02-10".into()),
            deciders: vec![],
            content: "We chose Rust.".into(),
            span: span(),
        }]);
        let md = to_markdown(&doc);
        assert!(md.contains("> **Decision** (accepted) (2026-02-10)"));
        assert!(md.contains("> We chose Rust."));
    }

    #[test]
    fn md_metric() {
        let doc = doc_with(vec![Block::Metric {
            label: "MRR".into(),
            value: "$2K".into(),
            trend: Some(Trend::Up),
            unit: Some("USD".into()),
            span: span(),
        }]);
        let md = to_markdown(&doc);
        assert!(md.contains("**MRR**: $2K USD"));
        assert!(md.contains("\u{2191}")); // up arrow
    }

    #[test]
    fn md_summary() {
        let doc = doc_with(vec![Block::Summary {
            content: "Executive overview.".into(),
            span: span(),
        }]);
        let md = to_markdown(&doc);
        assert!(md.contains("> *Executive overview.*"));
    }

    #[test]
    fn md_figure() {
        let doc = doc_with(vec![Block::Figure {
            src: "diagram.png".into(),
            caption: Some("Architecture".into()),
            alt: Some("Diagram".into()),
            width: None,
            span: span(),
        }]);
        let md = to_markdown(&doc);
        assert!(md.contains("![Diagram](diagram.png)"));
        assert!(md.contains("*Architecture*"));
    }

    #[test]
    fn md_no_surfdoc_markers() {
        let doc = doc_with(vec![
            Block::Callout {
                callout_type: CalloutType::Info,
                title: None,
                content: "Hello".into(),
                span: span(),
            },
            Block::Code {
                lang: Some("rust".into()),
                file: None,
                highlight: vec![],
                content: "let x = 1;".into(),
                span: span(),
            },
            Block::Metric {
                label: "A".into(),
                value: "1".into(),
                trend: None,
                unit: None,
                span: span(),
            },
        ]);
        let md = to_markdown(&doc);
        // Ensure no :: markers exist (they belong to SurfDoc directives, not Markdown)
        assert!(
            !md.contains("::callout"),
            "Output should not contain ::callout markers"
        );
        assert!(
            !md.contains("::code"),
            "Output should not contain ::code markers"
        );
        assert!(
            !md.contains("::metric"),
            "Output should not contain ::metric markers"
        );
    }
}
