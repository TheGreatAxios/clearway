//! Universal `wasm-bindgen` bindings for `clearway`.
//!
//! All functions exchange JSON strings to stay tiny and work identically in
//! browsers, Chrome extensions, Cloudflare Workers, Node, Bun, and Deno.

use clearway::{AuditOptions, BUILTIN_RULES, Document};
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

fn parse_options(options_json: Option<String>) -> Result<AuditOptions, JsValue> {
    match options_json {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(&raw)
            .map_err(|err| JsValue::from_str(&format!("invalid options JSON: {err}"))),
        _ => Ok(AuditOptions::default()),
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, JsValue> {
    serde_json::to_string(value)
        .map_err(|err| JsValue::from_str(&format!("serialization failed: {err}")))
}

/// Parses an HTML5 string and returns the `AuditReport` serialized as JSON.
#[wasm_bindgen]
pub fn audit_html(html: &str, options_json: Option<String>) -> Result<String, JsValue> {
    let options = parse_options(options_json)?;
    let report = clearway::audit_html(html, &options);
    to_json(&report)
}

/// Audits a JSON-serialized browser-agnostic `Document` IR and returns the
/// `AuditReport` serialized as JSON.
#[wasm_bindgen]
pub fn audit_document(doc_json: &str, options_json: Option<String>) -> Result<String, JsValue> {
    let doc: Document = serde_json::from_str(doc_json)
        .map_err(|err| JsValue::from_str(&format!("invalid Document IR JSON: {err}")))?;
    let options = parse_options(options_json)?;
    let report = clearway::audit_document(&doc, &options);
    to_json(&report)
}

/// Returns metadata for all 113 built-in rules serialized as a JSON array.
#[wasm_bindgen]
pub fn builtin_rules() -> Result<String, JsValue> {
    to_json(&BUILTIN_RULES)
}
