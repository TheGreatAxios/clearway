//! Zero-dependency SARIF 2.1.0 serializer for GitHub Code Scanning and PR annotations.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::rule::{AuditReport, RuleLevel, Violation};
use crate::rules::BUILTIN_RULES;

fn build_result(v: &Violation, level: &str, target_uri: &str) -> Value {
    json!({
        "ruleId": v.rule_id,
        "level": level,
        "message": {
            "text": v.description
        },
        "locations": [
            {
                "physicalLocation": {
                    "artifactLocation": {
                        "uri": target_uri
                    }
                },
                "logicalLocations": [
                    {
                        "fullyQualifiedName": v.selector
                    }
                ]
            }
        ],
        "properties": {
            "htmlSnippet": v.html_snippet
        }
    })
}

/// Serializes an [`AuditReport`] to pretty-printed SARIF 2.1.0 JSON.
pub fn to_sarif_json(report: &AuditReport, target_uri: &str) -> Result<String, serde_json::Error> {
    let mut seen_rules = BTreeSet::new();
    let mut rules = Vec::new();

    for v in report.violations.iter().chain(&report.incomplete) {
        if seen_rules.insert(v.rule_id.as_str()) {
            let tags: &[&str] = BUILTIN_RULES
                .iter()
                .find(|meta| meta.id == v.rule_id)
                .map_or(&[], |meta| meta.tags);
            rules.push(json!({
                "id": v.rule_id,
                "shortDescription": {
                    "text": v.help
                },
                "helpUri": v.docs_url,
                "properties": {
                    "tags": tags
                }
            }));
        }
    }

    let mut results = Vec::with_capacity(report.violations.len() + report.incomplete.len());
    for v in &report.violations {
        let level = match v.level {
            RuleLevel::Error => "error",
            RuleLevel::Warn => "warning",
            RuleLevel::Off => "none",
        };
        results.push(build_result(v, level, target_uri));
    }
    for v in &report.incomplete {
        results.push(build_result(v, "note", target_uri));
    }

    let sarif = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "clearway",
                        "version": env!("CARGO_PKG_VERSION"),
                        "informationUri": "https://github.com/TheGreatAxios/clearway",
                        "rules": rules
                    }
                },
                "results": results
            }
        ]
    });

    serde_json::to_string_pretty(&sarif)
}
