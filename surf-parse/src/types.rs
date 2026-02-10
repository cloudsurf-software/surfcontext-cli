use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

/// A parsed SurfDoc document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfDoc {
    /// Parsed YAML front matter, if present.
    pub front_matter: Option<FrontMatter>,
    /// Ordered sequence of blocks in the document body.
    pub blocks: Vec<Block>,
    /// Original source text that was parsed.
    pub source: String,
}

/// YAML front matter fields.
///
/// Known fields are typed; unknown fields are captured in `extra`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FrontMatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<DocType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<DocStatus>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub related: Option<Vec<Related>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributors: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,

    /// Any front matter fields not covered by typed fields above.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

/// A cross-reference to another document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Related {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<Relationship>,
}

/// Relationship type for cross-references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Relationship {
    Produces,
    Consumes,
    References,
    Supersedes,
}

/// SurfDoc document types (front matter `type` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocType {
    Doc,
    Guide,
    Conversation,
    Plan,
    Agent,
    Preference,
    Report,
    Proposal,
    Incident,
    Review,
}

/// Document lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocStatus {
    Draft,
    Active,
    Closed,
    Archived,
}

/// Visibility/access scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    Personal,
    WorkspacePrivate,
    Workspace,
    Repo,
    Public,
}

/// Confidence level for guides and estimates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// A parsed block in the document body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Block {
    /// A block directive that has not yet been typed (Chunk 1 catch-all).
    Unknown {
        name: String,
        attrs: Attrs,
        content: String,
        span: Span,
    },
    /// Plain markdown content between directives.
    Markdown {
        content: String,
        span: Span,
    },
    /// Callout/admonition box.
    Callout {
        callout_type: CalloutType,
        title: Option<String>,
        content: String,
        span: Span,
    },
    /// Structured data table (CSV/JSON/inline rows).
    Data {
        id: Option<String>,
        format: DataFormat,
        sortable: bool,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        raw_content: String,
        span: Span,
    },
    /// Code block with optional language and file path.
    Code {
        lang: Option<String>,
        file: Option<String>,
        highlight: Vec<String>,
        content: String,
        span: Span,
    },
    /// Task list with checkbox items.
    Tasks {
        items: Vec<TaskItem>,
        span: Span,
    },
    /// Decision record.
    Decision {
        status: DecisionStatus,
        date: Option<String>,
        deciders: Vec<String>,
        content: String,
        span: Span,
    },
    /// Single metric display.
    Metric {
        label: String,
        value: String,
        trend: Option<Trend>,
        unit: Option<String>,
        span: Span,
    },
    /// Executive summary block.
    Summary {
        content: String,
        span: Span,
    },
    /// Figure with image source and caption.
    Figure {
        src: String,
        caption: Option<String>,
        alt: Option<String>,
        width: Option<String>,
        span: Span,
    },
}

/// Callout/admonition type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalloutType {
    Info,
    Warning,
    Danger,
    Tip,
    Note,
    Success,
}

/// Data block format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataFormat {
    Table,
    Csv,
    Json,
}

/// A single task item within a `Tasks` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub done: bool,
    pub text: String,
    pub assignee: Option<String>,
}

/// Decision record status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionStatus {
    Proposed,
    Accepted,
    Rejected,
    Superseded,
}

/// Metric trend direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trend {
    Up,
    Down,
    Flat,
}

/// Inline extension found within text content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InlineExt {
    Evidence {
        tier: Option<u8>,
        source: Option<String>,
        text: String,
    },
    Status {
        value: String,
    },
}

/// Ordered map of attribute key-value pairs.
pub type Attrs = BTreeMap<String, AttrValue>;

/// A value inside a block directive attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttrValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

/// Source location of a block in the original document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// 1-based starting line number.
    pub start_line: usize,
    /// 1-based ending line number (inclusive).
    pub end_line: usize,
    /// 0-based byte offset of the first character.
    pub start_offset: usize,
    /// 0-based byte offset past the last character.
    pub end_offset: usize,
}
