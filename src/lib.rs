//! Pure-Rust, browser-agnostic accessibility engine for HTML5 and DOM IR snapshots.
//!
//! The library core is 100% synchronous with zero filesystem, process, or network
//! I/O so it compiles cleanly for native Rust, WASM, `napi-rs`, `PyO3`, and Ruby FFI.

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
