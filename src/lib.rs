//! Fast, browser-agnostic WCAG 2.1/2.2 accessibility engine for HTML5 documents
//! and live browser DOM IR snapshots.
//!
//! `clearway` ships with 113 built-in rules covering WCAG 2.0/2.1/2.2,
//! WAI-ARIA 1.2/1.3, HTML-AAM, `AccName` 1.2, and W3C ACT Rules. The library core
//! is synchronous with zero filesystem, process, or network I/O, so it works
//! across native Rust, WebAssembly, and FFI targets.
//!
//! # Quickstart
//!
//! ```
//! use clearway::{AuditOptions, audit_html};
//!
//! let html = r#"<!doctype html><html lang="en"><head><title>Home</title></head><body><main><h1>Hello</h1></main></body></html>"#;
//! let report = audit_html(html, &AuditOptions::default());
//! assert_eq!(report.summary.violations, 0);
//! ```
//!
//! # Entry points
//!
//! - [`audit_html`]: parse an HTML5 string and run all enabled rules.
//! - [`audit_document`]: audit a pre-built [`Document`] IR (e.g., from a live
//!   browser snapshot).
//! - [`to_sarif_json`]: serialize an [`AuditReport`] to SARIF 2.1.0 for GitHub
//!   Code Scanning.

pub mod accname;
pub mod aria;
pub mod contrast;
pub mod html;
pub mod ir;
pub mod rule;
pub mod rules;
pub mod sarif;

pub use crate::html::parse_html;
pub use crate::ir::{ComputedStyle, Document, Node, NodeId, Rect, Rgba};
pub use crate::rule::{
    AuditOptions, AuditReport, AuditSummary, ConformanceProfile, Impact, Rule, RuleLevel, RuleMeta,
    Violation,
};
pub use crate::rules::BUILTIN_RULES;
pub use crate::sarif::to_sarif_json;

/// Parses an HTML5 string into a [`Document`] and audits it against all enabled
/// built-in rules.
pub fn audit_html(html: &str, options: &AuditOptions) -> AuditReport {
    let doc = parse_html(html);
    audit_document(&doc, options)
}

/// Audits a pre-built browser-agnostic [`Document`] IR against all enabled
/// built-in rules.
pub fn audit_document(doc: &Document, options: &AuditOptions) -> AuditReport {
    let all_rules = rules::builtin_rules();
    let mut violations = Vec::new();
    let mut incomplete = Vec::new();
    let mut rule_out = Vec::new();
    let mut rules_run = 0usize;
    let mut error_count = 0usize;
    let mut warning_count = 0usize;

    for rule in &all_rules {
        let meta = rule.meta();
        let level = options.effective_level(&meta);
        if level == RuleLevel::Off {
            continue;
        }
        rules_run += 1;
        let explicit_error = options.rules.get(meta.id).copied() == Some(RuleLevel::Error);
        rule_out.clear();
        rule.run_with_options(doc, options, &mut rule_out);
        for mut v in rule_out.drain(..) {
            v.level = level;
            if !explicit_error && (meta.needs_review || v.is_incomplete) {
                incomplete.push(v);
            } else {
                match level {
                    RuleLevel::Error => error_count += 1,
                    RuleLevel::Warn => warning_count += 1,
                    RuleLevel::Off => {}
                }
                violations.push(v);
            }
        }
    }

    let incomplete_count = incomplete.len();
    let nodes_checked = doc.elements().count();
    AuditReport {
        violations,
        incomplete,
        summary: AuditSummary {
            violations: error_count,
            warnings: warning_count,
            incomplete: incomplete_count,
            rules_run,
            nodes_checked,
        },
    }
}
