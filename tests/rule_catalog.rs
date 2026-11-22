//! Built-in rule catalog integrity tests.

use std::collections::BTreeSet;

use clearway::BUILTIN_RULES;
use clearway::rules::builtin_rules;

const KNOWN_PACK_TAGS: &[&str] = &[
    "wcag2a",
    "wcag2aa",
    "wcag21a",
    "wcag21aa",
    "wcag22aa",
    "wcag2aaa",
    "wcag2a-obsolete",
    "best-practice",
    "ACT",
    "experimental",
    "clearway",
    "wcag3-draft",
];

const ALLOWED_DOCS_PREFIXES: &[&str] = &[
    "https://www.w3.org/",
    "https://html.spec.whatwg.org/",
    "https://developer.mozilla.org/",
];

#[test]
fn builtin_rule_catalog_has_113_unique_consistent_rules() {
    assert_eq!(
        BUILTIN_RULES.len(),
        113,
        "expected 113 built-in rules (105 core + 8 clearway extensions)"
    );

    let meta_ids: BTreeSet<&str> = BUILTIN_RULES.iter().map(|r| r.id).collect();
    let impl_ids: BTreeSet<&str> = builtin_rules().iter().map(|r| r.meta().id).collect();

    assert_eq!(
        BUILTIN_RULES.len(),
        meta_ids.len(),
        "BUILTIN_RULES contains duplicate rule IDs"
    );
    assert_eq!(
        meta_ids, impl_ids,
        "BUILTIN_RULES and builtin_rules() must contain the exact same rule IDs"
    );

    let extension_count = BUILTIN_RULES
        .iter()
        .filter(|r| r.tags.contains(&"clearway"))
        .count();
    assert_eq!(
        extension_count, 8,
        "expected 8 modular clearway extension rules"
    );
}

#[test]
fn builtin_rules_have_valid_metadata_docs_and_pack_tags() {
    let known_packs: BTreeSet<&str> = KNOWN_PACK_TAGS.iter().copied().collect();

    for rule in BUILTIN_RULES {
        assert!(!rule.id.trim().is_empty(), "rule ID must not be empty");
        assert!(
            !rule.help.trim().is_empty(),
            "rule `{}` must have non-empty help text",
            rule.id
        );
        assert!(
            !rule.description.trim().is_empty(),
            "rule `{}` must have non-empty description text",
            rule.id
        );
        assert!(
            ALLOWED_DOCS_PREFIXES
                .iter()
                .any(|prefix| rule.docs_url.starts_with(prefix)),
            "rule `{}` has non-standard docs_url: {}",
            rule.id,
            rule.docs_url
        );
        assert!(
            rule.tags.iter().any(|tag| known_packs.contains(tag)),
            "rule `{}` must carry at least one recognized pack tag (found: {:?})",
            rule.id,
            rule.tags
        );
    }
}
