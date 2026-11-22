//! Rule trait, violation model, audit options, and report types.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ir::{Document, NodeId};

/// User-facing severity impact of an accessibility violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Impact {
    Minor,
    Moderate,
    Serious,
    Critical,
}

/// Configurable enforcement level for a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleLevel {
    Off,
    Warn,
    #[default]
    Error,
}

/// Static metadata describing a built-in accessibility rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RuleMeta {
    pub id: &'static str,
    pub impact: Impact,
    pub wcag: &'static str,
    pub tags: &'static [&'static str],
    pub enabled_by_default: bool,
    pub needs_review: bool,
    pub help: &'static str,
    pub description: &'static str,
    pub docs_url: &'static str,
}

/// A single accessibility finding reported against a node in a [`Document`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Violation {
    pub rule_id: String,
    pub impact: Impact,
    pub level: RuleLevel,
    pub help: String,
    pub description: String,
    pub docs_url: String,
    pub selector: String,
    pub html_snippet: String,
    #[serde(skip)]
    pub is_incomplete: bool,
}

impl Violation {
    /// Constructs a [`Violation`] for `node_id` using the rule's default description.
    pub fn from_node(meta: &RuleMeta, doc: &Document, node_id: NodeId) -> Self {
        Self::with_detail(meta, doc, node_id, meta.description)
    }

    /// Constructs a [`Violation`] for `node_id` with a customized description.
    pub fn with_detail(
        meta: &RuleMeta,
        doc: &Document,
        node_id: NodeId,
        description: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: meta.id.to_owned(),
            impact: meta.impact,
            level: RuleLevel::Error,
            help: meta.help.to_owned(),
            description: description.into(),
            docs_url: meta.docs_url.to_owned(),
            selector: doc.css_selector(node_id),
            html_snippet: doc.html_snippet(node_id),
            is_incomplete: false,
        }
    }

    /// Marks this finding as requiring manual/visual review (`incomplete`).
    #[must_use]
    pub const fn into_incomplete(mut self) -> Self {
        self.is_incomplete = true;
        self
    }
}

/// Conformance profile controlling how the `label` rule handles
/// `placeholder`-only form inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConformanceProfile {
    /// Default profile: flags `placeholder`-only form inputs under the `label` rule.
    #[default]
    Standard,
    /// Accepts `placeholder` as a fallback accessible name on the `label` rule
    /// per W3C `AccName`.
    #[serde(alias = "act")]
    W3cAct,
}

/// Options controlling which rules run and at what level (`off`, `warn`, `error`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditOptions {
    #[serde(default)]
    pub rules: BTreeMap<String, RuleLevel>,
    /// Optional tag filter (e.g. `["wcag2a", "wcag2aa"]`). When `Some`, only rules
    /// matching at least one requested tag are executed.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// When `true`, also run rules with `enabled_by_default == false`.
    #[serde(default)]
    pub include_disabled: bool,
    /// Conformance profile (`Standard` by default; `W3cAct` accepts `placeholder`
    /// as a fallback accessible name on the `label` rule).
    #[serde(default)]
    pub profile: ConformanceProfile,
}

impl AuditOptions {
    /// Returns the configured [`RuleLevel`] override for `rule_id` (defaults to [`RuleLevel::Error`]).
    pub fn level_for(&self, rule_id: &str) -> RuleLevel {
        self.rules.get(rule_id).copied().unwrap_or(RuleLevel::Error)
    }

    /// Returns the effective [`RuleLevel`] for a rule given its metadata, tag
    /// filter, `include_disabled` setting, and per-rule overrides.
    pub fn effective_level(&self, meta: &RuleMeta) -> RuleLevel {
        let override_level = self.rules.get(meta.id).copied();
        if override_level == Some(RuleLevel::Off) {
            return RuleLevel::Off;
        }
        if !meta.enabled_by_default
            && !self.include_disabled
            && !matches!(override_level, Some(RuleLevel::Warn | RuleLevel::Error))
        {
            return RuleLevel::Off;
        }
        if let Some(filter) = &self.tags {
            let matches_tag = meta
                .tags
                .iter()
                .any(|tag| filter.iter().any(|wanted| tag.eq_ignore_ascii_case(wanted)));
            if !matches_tag {
                return RuleLevel::Off;
            }
        }
        override_level.unwrap_or(RuleLevel::Error)
    }
}

/// High-level summary counts for an [`AuditReport`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditSummary {
    pub violations: usize,
    pub warnings: usize,
    #[serde(default)]
    pub incomplete: usize,
    pub rules_run: usize,
    pub nodes_checked: usize,
}

/// Complete output of auditing a [`Document`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditReport {
    pub violations: Vec<Violation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incomplete: Vec<Violation>,
    pub summary: AuditSummary,
}

/// A deterministic accessibility rule evaluated against a [`Document`].
pub trait Rule: Send + Sync {
    /// Returns static metadata for this rule.
    fn meta(&self) -> RuleMeta;

    /// Evaluates the rule against `doc` and appends any findings to `out`.
    fn run(&self, doc: &Document, out: &mut Vec<Violation>);

    /// Evaluates the rule against `doc` with [`AuditOptions`] (e.g. [`ConformanceProfile`]).
    fn run_with_options(&self, doc: &Document, _options: &AuditOptions, out: &mut Vec<Violation>) {
        self.run(doc, out);
    }
}
