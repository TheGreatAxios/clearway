//! Load-bearing tests for provable CSS gradient contrast and the 8 modular
//! Clearway extension rules (`src/rules/extensions.rs`).

use std::collections::BTreeMap;

use clearway::contrast::apca_lc;
use clearway::{
    AuditOptions, ComputedStyle, Document, Rgba, RuleLevel, audit_document, audit_html, parse_html,
};

fn options_for(rule_id: &str) -> AuditOptions {
    let mut rules = BTreeMap::new();
    rules.insert(rule_id.to_owned(), RuleLevel::Error);
    AuditOptions {
        rules,
        include_disabled: true,
        ..AuditOptions::default()
    }
}

fn rule_violations(doc: &Document, rule_id: &str) -> usize {
    audit_document(doc, &options_for(rule_id))
        .violations
        .iter()
        .filter(|v| v.rule_id == rule_id)
        .count()
}

fn html_rule_violations(html: &str, rule_id: &str) -> usize {
    audit_html(html, &options_for(rule_id))
        .violations
        .iter()
        .filter(|v| v.rule_id == rule_id)
        .count()
}

fn set_computed_for_tag(doc: &mut Document, tag: &str, style: ComputedStyle) {
    if let Some(node) = doc.nodes.iter_mut().find(|n| n.tag == tag) {
        node.computed = Some(style);
    }
}

#[test]
fn gradient_contrast_all_stops_fail_emits_hard_violation() {
    let mut doc = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><p>Low contrast on light gradient</p></body></html>",
    );
    set_computed_for_tag(
        &mut doc,
        "p",
        ComputedStyle {
            color: Some(Rgba::opaque(180, 180, 180)),
            font_size_px: Some(16.0),
            font_weight: Some(400),
            background_gradient_stops: vec![
                Rgba::opaque(255, 255, 255),
                Rgba::opaque(235, 235, 235),
            ],
            ..ComputedStyle::default()
        },
    );

    let report = audit_document(&doc, &AuditOptions::default());
    let violations: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.rule_id == "color-contrast")
        .collect();
    let incomplete: Vec<_> = report
        .incomplete
        .iter()
        .filter(|v| v.rule_id == "color-contrast")
        .collect();
    assert_eq!(
        violations.len(),
        1,
        "all-failing gradient must emit hard violation"
    );
    assert_eq!(incomplete.len(), 0);
}

#[test]
fn gradient_contrast_all_stops_pass_emits_nothing() {
    let mut doc = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><p>High contrast on dark gradient</p></body></html>",
    );
    set_computed_for_tag(
        &mut doc,
        "p",
        ComputedStyle {
            color: Some(Rgba::opaque(255, 255, 255)),
            font_size_px: Some(16.0),
            font_weight: Some(400),
            background_gradient_stops: vec![Rgba::opaque(10, 10, 40), Rgba::opaque(30, 20, 60)],
            ..ComputedStyle::default()
        },
    );

    let report = audit_document(&doc, &AuditOptions::default());
    assert!(
        !report
            .violations
            .iter()
            .any(|v| v.rule_id == "color-contrast")
    );
    assert!(
        !report
            .incomplete
            .iter()
            .any(|v| v.rule_id == "color-contrast")
    );
}

#[test]
fn gradient_contrast_mixed_stops_routes_to_incomplete() {
    let mut doc = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><p>Mixed contrast gradient</p></body></html>",
    );
    set_computed_for_tag(
        &mut doc,
        "p",
        ComputedStyle {
            color: Some(Rgba::opaque(255, 255, 255)),
            font_size_px: Some(16.0),
            font_weight: Some(400),
            background_gradient_stops: vec![Rgba::opaque(0, 0, 0), Rgba::opaque(255, 255, 255)],
            ..ComputedStyle::default()
        },
    );

    let report = audit_document(&doc, &AuditOptions::default());
    assert!(
        !report
            .violations
            .iter()
            .any(|v| v.rule_id == "color-contrast")
    );
    assert_eq!(
        report
            .incomplete
            .iter()
            .filter(|v| v.rule_id == "color-contrast")
            .count(),
        1
    );
}

#[test]
fn accessible_authentication_pass_and_fail() {
    let bad_return = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><input type=\"password\" onpaste=\"return false\"></body></html>";
    let bad_prevent = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><input type=\"password\" onpaste=\"event.preventDefault()\"></body></html>";
    let good = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><input type=\"password\"></body></html>";

    assert_eq!(
        html_rule_violations(bad_return, "accessible-authentication"),
        1
    );
    assert_eq!(
        html_rule_violations(bad_prevent, "accessible-authentication"),
        1
    );
    assert_eq!(html_rule_violations(good, "accessible-authentication"), 0);
}

#[test]
fn focus_outline_none_pass_and_fail() {
    let bad = "<!doctype html><html lang=\"en\"><head><title>T</title><style>a:focus { outline: none; }</style></head><body><a href=\"/\">Home</a></body></html>";
    let good_replacement = "<!doctype html><html lang=\"en\"><head><title>T</title><style>a:focus { outline: 0; box-shadow: 0 0 0 2px blue; }</style></head><body><a href=\"/\">Home</a></body></html>";
    let good_focus_visible = "<!doctype html><html lang=\"en\"><head><title>T</title><style>a:focus-visible { outline: none; }</style></head><body><a href=\"/\">Home</a></body></html>";

    assert_eq!(html_rule_violations(bad, "focus-outline-none"), 1);
    assert_eq!(
        html_rule_violations(good_replacement, "focus-outline-none"),
        0
    );
    assert_eq!(
        html_rule_violations(good_focus_visible, "focus-outline-none"),
        0
    );
}

#[test]
fn popover_target_valid_pass_and_fail() {
    let missing_target = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><button popovertarget=\"menu\">Open</button></body></html>";
    let missing_attr = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><button popovertarget=\"menu\">Open</button><div id=\"menu\">Hi</div></body></html>";
    let good = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><button popovertarget=\"menu\">Open</button><div id=\"menu\" popover>Hi</div></body></html>";

    assert_eq!(
        html_rule_violations(missing_target, "popover-target-valid"),
        1
    );
    assert_eq!(
        html_rule_violations(missing_attr, "popover-target-valid"),
        1
    );
    assert_eq!(html_rule_violations(good, "popover-target-valid"), 0);
}

#[test]
fn pointer_events_focusable_pass_and_fail() {
    let mut bad = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><button>Click</button></body></html>",
    );
    set_computed_for_tag(
        &mut bad,
        "button",
        ComputedStyle {
            pointer_events: Some("none".to_owned()),
            ..ComputedStyle::default()
        },
    );
    assert_eq!(rule_violations(&bad, "pointer-events-focusable"), 1);

    let mut good = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><button>Click</button></body></html>",
    );
    set_computed_for_tag(
        &mut good,
        "button",
        ComputedStyle {
            pointer_events: Some("auto".to_owned()),
            ..ComputedStyle::default()
        },
    );
    assert_eq!(rule_violations(&good, "pointer-events-focusable"), 0);
}

#[test]
fn text_clipping_overflow_pass_and_fail() {
    let mut bad = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><p>Clipped text content</p></body></html>",
    );
    set_computed_for_tag(
        &mut bad,
        "p",
        ComputedStyle {
            overflow_y: Some("hidden".to_owned()),
            scroll_height: Some(80.0),
            client_height: Some(24.0),
            ..ComputedStyle::default()
        },
    );
    assert_eq!(rule_violations(&bad, "text-clipping-overflow"), 1);

    let mut good = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><p>Fits fine</p></body></html>",
    );
    set_computed_for_tag(
        &mut good,
        "p",
        ComputedStyle {
            overflow_y: Some("hidden".to_owned()),
            scroll_height: Some(24.0),
            client_height: Some(24.0),
            ..ComputedStyle::default()
        },
    );
    assert_eq!(rule_violations(&good, "text-clipping-overflow"), 0);
}

#[test]
fn reading_order_reverse_pass_and_fail() {
    let mut bad = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><div><button>First</button><button>Second</button></div></body></html>",
    );
    set_computed_for_tag(
        &mut bad,
        "div",
        ComputedStyle {
            flex_direction: Some("row-reverse".to_owned()),
            ..ComputedStyle::default()
        },
    );
    assert_eq!(rule_violations(&bad, "reading-order-reverse"), 1);

    let mut good_single = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><div><button>Only one</button></div></body></html>",
    );
    set_computed_for_tag(
        &mut good_single,
        "div",
        ComputedStyle {
            flex_direction: Some("row-reverse".to_owned()),
            ..ComputedStyle::default()
        },
    );
    assert_eq!(rule_violations(&good_single, "reading-order-reverse"), 0);
}

#[test]
fn apca_contrast_math_and_rule_pass_and_fail() {
    let black = Rgba::opaque(0, 0, 0);
    let white = Rgba::opaque(255, 255, 255);
    let bow = apca_lc(black, white);
    let wob = apca_lc(white, black);
    assert!((bow - 106.04).abs() < 0.2, "unexpected BoW Lc: {bow}");
    assert!((wob + 107.88).abs() < 0.2, "unexpected WoB Lc: {wob}");
    assert!((apca_lc(white, white)).abs() < f64::EPSILON);

    let mut bad = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><p>Low APCA</p></body></html>",
    );
    set_computed_for_tag(
        &mut bad,
        "p",
        ComputedStyle {
            color: Some(Rgba::opaque(160, 160, 160)),
            font_size_px: Some(16.0),
            font_weight: Some(400),
            ..ComputedStyle::default()
        },
    );
    assert_eq!(rule_violations(&bad, "apca-contrast"), 1);

    let mut good = parse_html(
        "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><p>High APCA</p></body></html>",
    );
    set_computed_for_tag(
        &mut good,
        "p",
        ComputedStyle {
            color: Some(Rgba::opaque(20, 20, 20)),
            font_size_px: Some(16.0),
            font_weight: Some(400),
            ..ComputedStyle::default()
        },
    );
    assert_eq!(rule_violations(&good, "apca-contrast"), 0);
}

#[test]
fn prefers_reduced_motion_pass_and_fail() {
    let bad = "<!doctype html><html lang=\"en\"><head><title>T</title><style>@keyframes spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }</style></head><body><div>Spin</div></body></html>";
    let good = "<!doctype html><html lang=\"en\"><head><title>T</title><style>@keyframes spin { from { transform: rotate(0); } to { transform: rotate(360deg); } } @media (prefers-reduced-motion: reduce) { div { animation: none; } }</style></head><body><div>Spin</div></body></html>";

    assert_eq!(html_rule_violations(bad, "prefers-reduced-motion"), 1);
    assert_eq!(html_rule_violations(good, "prefers-reduced-motion"), 0);
}
