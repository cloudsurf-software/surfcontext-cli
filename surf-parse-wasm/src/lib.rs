//! WASM bindings for `surf-parse`.
//!
//! Exposes the SurfDoc parser to JavaScript via wasm-bindgen.
//! Call `parse()` with a string to get a JSON AST, or use
//! `render_html()` / `render_markdown()` for rendered output.

use wasm_bindgen::prelude::*;

/// Parse a SurfDoc string and return the document as a JSON AST.
///
/// Returns a JSON object with `{ doc, diagnostics }`.
/// The `doc` contains `front_matter` and `blocks`.
/// `diagnostics` is an array of parse warnings/errors.
#[wasm_bindgen]
pub fn parse(input: &str) -> String {
    let result = surf_parse::parse(input);
    serde_json::json!({
        "doc": result.doc,
        "diagnostics": result.diagnostics,
    })
    .to_string()
}

/// Parse a SurfDoc string and return an HTML fragment.
///
/// The output uses `surfdoc-*` CSS classes. Wrap in an element
/// with class `surfdoc` and include the SurfDoc stylesheet.
#[wasm_bindgen]
pub fn render_html(input: &str) -> String {
    let result = surf_parse::parse(input);
    result.doc.to_html()
}

/// Parse a SurfDoc string and return a complete styled HTML page.
///
/// Includes embedded CSS, discovery metadata, and viewport tags.
/// The result is a standalone page that can be displayed in an iframe
/// or written to a file.
#[wasm_bindgen]
pub fn render_html_page(input: &str, title: Option<String>) -> String {
    let result = surf_parse::parse(input);
    let config = surf_parse::PageConfig {
        title,
        ..Default::default()
    };
    result.doc.to_html_page(&config)
}

/// Parse a SurfDoc string and return degraded CommonMark markdown.
///
/// Strips all `::` directives and renders each block as its closest
/// standard markdown equivalent.
#[wasm_bindgen]
pub fn render_markdown(input: &str) -> String {
    let result = surf_parse::parse(input);
    result.doc.to_markdown()
}

/// Validate a SurfDoc string and return diagnostics as JSON.
///
/// Returns a JSON array of `{ severity, message, span, code }` objects.
/// An empty array means the document is valid.
#[wasm_bindgen]
pub fn validate(input: &str) -> String {
    let result = surf_parse::parse(input);
    let mut all = result.diagnostics;
    all.extend(result.doc.validate());
    serde_json::to_string(&all).unwrap_or_else(|_| "[]".to_string())
}
