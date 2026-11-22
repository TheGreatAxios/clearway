//! W3C ACT-inspired conformance tests covering pass, fail, and inapplicable
//! cases across all rule categories, `AccName` 1.2 precedence, WCAG 2.1 contrast
//! with alpha compositing, and WCAG 2.2 target size.

use std::collections::BTreeMap;

use clearway::accname::accessible_name;
use clearway::contrast::{composite_over, contrast_ratio, relative_luminance};
use clearway::rules::builtin_rules;
use clearway::{
    AuditOptions, BUILTIN_RULES, ComputedStyle, Document, Rect, Rgba, RuleLevel, audit_document,
    audit_html, parse_html,
};

fn rule_violations(html: &str, rule_id: &str) -> Vec<clearway::Violation> {
    audit_html(html, &AuditOptions::default())
        .violations
        .into_iter()
        .filter(|v| v.rule_id == rule_id)
        .collect()
}

fn page(body: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><title>Test</title></head><body><main><h1>Title</h1>{body}</main></body></html>"
    )
}

#[test]
fn registry_matches_builtin_metadata() {
    let rules = builtin_rules();
    assert_eq!(rules.len(), BUILTIN_RULES.len());
    assert!(rules.len() >= 25);
    for (rule, meta) in rules.iter().zip(BUILTIN_RULES.iter()) {
        assert_eq!(rule.meta().id, meta.id);
    }
}

#[test]
fn accname_precedence_and_hidden_rules() {
    let html = r#"<!doctype html><html lang="en"><head><title>AccName</title></head><body>
      <span id="ref1" hidden>Hidden LabelledBy</span>
      <button id="b1" aria-labelledby="ref1" aria-label="Aria Label" title="Tooltip">Subtree</button>
      <button id="b2" aria-label="Aria Label" title="Tooltip">Subtree</button>
      <label for="i1">Native Label</label>
      <input id="i1" type="text" title="Input Tooltip">
      <button id="b3" title="Tooltip">Visible <span aria-hidden="true">Ignored</span> Text</button>
      <button id="b4" title="Only Tooltip"></button>
      <button id="cycle" aria-labelledby="cycle">Fallback</button>
    </body></html>"#;
    let doc = parse_html(html);

    let name_of = |id: &str| {
        let nid = doc.find_by_id(id).expect("element should exist");
        accessible_name(&doc, nid)
    };

    assert_eq!(name_of("b1"), "Hidden LabelledBy");
    assert_eq!(name_of("b2"), "Aria Label");
    assert_eq!(name_of("i1"), "Native Label");
    assert_eq!(name_of("b3"), "Visible Text");
    assert_eq!(name_of("b4"), "Only Tooltip");
    assert_eq!(name_of("cycle"), "Fallback");
}

#[test]
fn document_rules_pass_fail_inapplicable() {
    // Pass
    let clean = page("<p>Hello</p>");
    let report = audit_html(&clean, &AuditOptions::default());
    assert_eq!(report.summary.violations, 0);

    // Fail: missing title & lang
    let missing = "<html><head></head><body><main><h1>Hi</h1></main></body></html>";
    assert_eq!(rule_violations(missing, "document-title").len(), 1);
    assert_eq!(rule_violations(missing, "html-has-lang").len(), 1);
    // html-lang-valid is inapplicable when lang is missing
    assert_eq!(rule_violations(missing, "html-lang-valid").len(), 0);

    // Fail: invalid lang
    let bad_lang = "<html lang=\"123-bad\"><head><title>T</title></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(rule_violations(bad_lang, "html-has-lang").len(), 0);
    assert_eq!(rule_violations(bad_lang, "html-lang-valid").len(), 1);

    // Pass: valid regional BCP 47
    let good_lang = "<html lang=\"en-US\"><head><title>T</title></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(rule_violations(good_lang, "html-lang-valid").len(), 0);

    // Meta viewport: fail vs pass
    let bad_vp = "<html lang=\"en\"><head><title>T</title><meta name=\"viewport\" content=\"width=device-width, user-scalable=no\"></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(rule_violations(bad_vp, "meta-viewport").len(), 1);
    let bad_vp_scale = "<html lang=\"en\"><head><title>T</title><meta name=\"viewport\" content=\"width=device-width, maximum-scale=1.5\"></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(rule_violations(bad_vp_scale, "meta-viewport").len(), 1);
    let good_vp = "<html lang=\"en\"><head><title>T</title><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(rule_violations(good_vp, "meta-viewport").len(), 0);

    // Meta refresh: fail (>0) vs pass (0)
    let bad_refresh = "<html lang=\"en\"><head><title>T</title><meta http-equiv=\"refresh\" content=\"5;url=/next\"></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(rule_violations(bad_refresh, "meta-refresh").len(), 1);
    let zero_refresh = "<html lang=\"en\"><head><title>T</title><meta http-equiv=\"refresh\" content=\"0;url=/next\"></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(rule_violations(zero_refresh, "meta-refresh").len(), 0);
}

#[test]
fn accessible_name_rules_pass_fail_inapplicable() {
    // image-alt: fail, pass (alt), pass (decorative alt=""), inapplicable (aria-hidden)
    let imgs = page(
        r#"<img src="a.png">
           <img src="b.png" alt="Logo">
           <img src="c.png" alt="">
           <img src="d.png" aria-hidden="true">"#,
    );
    assert_eq!(rule_violations(&imgs, "image-alt").len(), 1);

    // button-name: fail, pass (text), pass (aria-label), inapplicable (hidden)
    let btns = page(
        r#"<button></button>
           <button>Save</button>
           <div role="button" aria-label="Close"></div>
           <button hidden></button>"#,
    );
    assert_eq!(rule_violations(&btns, "button-name").len(), 1);

    // link-name: fail, pass (img alt), inapplicable (<a> without href)
    let links = page(
        r#"<a href="/x"></a>
           <a href="/y"><img src="icon.png" alt="Home"></a>
           <a name="anchor"></a>"#,
    );
    assert_eq!(rule_violations(&links, "link-name").len(), 1);

    // input-image-alt
    let img_inputs = page(
        r#"<input type="image" src="go.png">
           <input type="image" src="go.png" alt="Go">"#,
    );
    assert_eq!(rule_violations(&img_inputs, "input-image-alt").len(), 1);

    // area-alt
    let areas = page(
        r#"<map name="m">
             <area shape="rect" coords="0,0,10,10" href="/a">
             <area shape="rect" coords="0,0,10,10" href="/b" alt="Region B">
             <area shape="rect" coords="0,0,10,10">
           </map>"#,
    );
    assert_eq!(rule_violations(&areas, "area-alt").len(), 1);

    // frame-title
    let frames = page(
        r#"<iframe src="/embed"></iframe>
           <iframe src="/embed" title="Video player"></iframe>
           <iframe src="/tracker" aria-hidden="true"></iframe>"#,
    );
    assert_eq!(rule_violations(&frames, "frame-title").len(), 1);

    // svg-img-alt
    let svgs = page(
        r#"<svg role="img"><rect width="10" height="10"/></svg>
           <svg role="img"><title>Chart</title><rect width="10" height="10"/></svg>
           <svg><rect width="10" height="10"/></svg>"#,
    );
    assert_eq!(rule_violations(&svgs, "svg-img-alt").len(), 1);
}

#[test]
fn form_rules_pass_fail_inapplicable() {
    let html = page(
        r#"<form>
             <input type="text">
             <label for="email">Email</label><input id="email" type="email">
             <label>Notes <textarea></textarea></label>
             <input type="hidden" name="csrf" value="abc">
             <input type="submit">
             <select><option>A</option></select>
             <select aria-label="Country"><option>US</option></select>
           </form>"#,
    );
    assert_eq!(rule_violations(&html, "label").len(), 1);
    assert_eq!(rule_violations(&html, "select-name").len(), 1);
}

#[test]
fn aria_rules_pass_fail_inapplicable() {
    // aria-roles: invalid & abstract fail; valid passes
    let roles = page(
        r#"<div role="notarole"></div>
           <div role="widget"></div>
           <div role="region" aria-label="Updates"></div>"#,
    );
    assert_eq!(rule_violations(&roles, "aria-roles").len(), 2);

    // aria-valid-attr
    let attrs = page(
        r#"<div aria-labelled="x"></div>
           <div aria-live="polite"></div>"#,
    );
    assert_eq!(rule_violations(&attrs, "aria-valid-attr").len(), 1);

    // aria-required-attr: custom checkbox missing aria-checked fails; native checkbox passes
    let req = page(
        r#"<div role="checkbox" aria-label="Agree"></div>
           <div role="checkbox" aria-checked="false" aria-label="Agree 2"></div>
           <input type="checkbox" role="checkbox" aria-label="Native">"#,
    );
    assert_eq!(rule_violations(&req, "aria-required-attr").len(), 1);

    // aria-hidden-focus: focusable button inside aria-hidden fails; tabindex=-1 passes
    let hidden_focus = page(
        r#"<div aria-hidden="true"><button>Trap</button></div>
           <div aria-hidden="true"><button tabindex="-1">Safe</button></div>"#,
    );
    assert_eq!(rule_violations(&hidden_focus, "aria-hidden-focus").len(), 1);

    // duplicate-id-aria: duplicate referenced by aria-labelledby fails; unreferenced duplicate is inapplicable
    let dup = page(
        r#"<span id="lbl">A</span><span id="lbl">B</span>
           <button aria-labelledby="lbl"></button>
           <div id="unref"></div><div id="unref"></div>"#,
    );
    assert_eq!(rule_violations(&dup, "duplicate-id-aria").len(), 1);
}

#[test]
fn structure_rules_pass_fail_inapplicable() {
    // heading-order & empty-heading
    let headings = page("<h3>Skipped</h3><h2></h2>");
    assert_eq!(rule_violations(&headings, "heading-order").len(), 1);
    assert_eq!(rule_violations(&headings, "empty-heading").len(), 1);

    // landmark-one-main & page-has-heading-one
    let no_landmarks = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><p>Plain text</p></body></html>";
    assert_eq!(rule_violations(no_landmarks, "landmark-one-main").len(), 1);
    assert_eq!(
        rule_violations(no_landmarks, "page-has-heading-one").len(),
        1
    );

    // list
    let lists = page("<ul><div>Bad</div></ul><ul><li>Good</li></ul>");
    assert_eq!(rule_violations(&lists, "list").len(), 1);

    // tabindex
    let tabs =
        page(r#"<div tabindex="3">Bad</div><div tabindex="0">Ok</div><div tabindex="-1">Ok</div>"#);
    assert_eq!(rule_violations(&tabs, "tabindex").len(), 1);

    // nested-interactive
    let nested = page(r#"<a href="/a"><button>Nested</button></a><a href="/b">Clean</a>"#);
    assert_eq!(rule_violations(&nested, "nested-interactive").len(), 1);
}

#[test]
fn color_contrast_alpha_compositing_and_large_text() {
    // Verify luminance & pure black/white contrast math (21:1)
    let black = Rgba::opaque(0, 0, 0);
    let white = Rgba::opaque(255, 255, 255);
    assert!((relative_luminance(black) - 0.0).abs() < 1e-6);
    assert!((relative_luminance(white) - 1.0).abs() < 1e-6);
    assert!((contrast_ratio(black, white) - 21.0).abs() < 0.01);

    // Verify alpha compositing: 50% black over white -> ~128 gray
    let mid = composite_over(Rgba::new(0, 0, 0, 0.5), white);
    assert!((i16::from(mid.r) - 128).abs() <= 1);

    // Build IR document with:
    // - Translucent dark overlay (rgba(0,0,0,0.5)) over white canvas -> rgb(128,128,128)
    // - p#fail: light gray text rgb(200,200,200) 16px -> fails 4.5:1
    // - p#large_pass: #767676 (rgb(118,118,118)) on white -> ~4.54:1; or #949494 (~3.03:1) at 24px -> passes 3.0:1!
    // - p#bold_large_pass: #949494 at 19px bold (700) -> passes 3.0:1!
    // - p#inapplicable: no computed style -> skipped
    let mut doc = parse_html(&page(
        r#"<div id="card"><p id="fail">Low contrast</p></div>
           <p id="large_pass">Large text</p>
           <p id="bold_large_pass">Bold large text</p>
           <p id="inapplicable">No computed style</p>"#,
    ));

    let set_style = |doc: &mut Document, id: &str, style: ComputedStyle| {
        let nid = doc.find_by_id(id).expect("node exists");
        doc.nodes[nid].computed = Some(style);
    };

    // Parent card has 50% translucent black over default white canvas -> effective bg rgb(128,128,128)
    set_style(
        &mut doc,
        "card",
        ComputedStyle {
            background_color: Some(Rgba::new(0, 0, 0, 0.5)),
            ..ComputedStyle::default()
        },
    );
    // Translucent foreground over rgb(128,128,128) -> low contrast
    set_style(
        &mut doc,
        "fail",
        ComputedStyle {
            color: Some(Rgba::new(220, 220, 220, 0.85)),
            font_size_px: Some(16.0),
            font_weight: Some(400),
            ..ComputedStyle::default()
        },
    );
    // #949494 on white is ~3.03:1: fails 4.5:1 normal, passes 3.0:1 large (>= 24px)
    set_style(
        &mut doc,
        "large_pass",
        ComputedStyle {
            color: Some(Rgba::opaque(148, 148, 148)),
            font_size_px: Some(24.0),
            font_weight: Some(400),
            ..ComputedStyle::default()
        },
    );
    // #949494 at 19px bold (700) also qualifies as large text (>= 18.66px bold)
    set_style(
        &mut doc,
        "bold_large_pass",
        ComputedStyle {
            color: Some(Rgba::opaque(148, 148, 148)),
            font_size_px: Some(19.0),
            font_weight: Some(700),
            ..ComputedStyle::default()
        },
    );

    let report = audit_document(&doc, &AuditOptions::default());
    let contrast_failures: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.rule_id == "color-contrast")
        .collect();
    assert_eq!(contrast_failures.len(), 1);
    assert_eq!(contrast_failures[0].selector, "#fail");
}

#[test]
fn target_size_wcag_22_and_ir_serde_roundtrip() {
    let mut doc = parse_html(&page(
        r#"<button id="small">X</button>
           <button id="ok">Save</button>
           <button id="no_rect">Static</button>"#,
    ));

    let small_id = doc.find_by_id("small").expect("small exists");
    doc.nodes[small_id].rect = Some(Rect {
        x: 10.0,
        y: 10.0,
        width: 18.0,
        height: 18.0,
    });
    let ok_id = doc.find_by_id("ok").expect("ok exists");
    doc.nodes[ok_id].rect = Some(Rect {
        x: 30.0,
        y: 10.0,
        width: 44.0,
        height: 24.0,
    });

    // Round-trip through JSON to verify browser-agnostic IR serialization
    let json = serde_json::to_string(&doc).expect("serialize IR");
    let decoded: Document = serde_json::from_str(&json).expect("deserialize IR");
    assert_eq!(doc, decoded);

    let report = audit_document(&decoded, &AuditOptions::default());
    let size_failures: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.rule_id == "target-size")
        .collect();
    assert_eq!(size_failures.len(), 1);
    assert_eq!(size_failures[0].selector, "#small");
}

#[test]
fn audit_options_off_warn_error() {
    let html = page(r#"<img src="a.png"><button></button>"#);
    let mut rules = BTreeMap::new();
    rules.insert("image-alt".to_owned(), RuleLevel::Off);
    rules.insert("button-name".to_owned(), RuleLevel::Warn);
    let report = audit_html(
        &html,
        &AuditOptions {
            rules,
            ..AuditOptions::default()
        },
    );

    assert_eq!(report.summary.violations, 0);
    assert_eq!(report.summary.warnings, 1);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].rule_id, "button-name");
    assert_eq!(report.violations[0].level, RuleLevel::Warn);
}

#[test]
fn audit_options_tag_filtering_and_disabled_rules() {
    use clearway::{Impact, RuleMeta};

    // Tag filter: only best-practice rules run (no wcag2a image-alt)
    let html = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><img src=\"a.png\"><p>No main or h1</p></body></html>";
    let report = audit_html(
        html,
        &AuditOptions {
            tags: Some(vec!["best-practice".to_owned()]),
            ..AuditOptions::default()
        },
    );
    assert!(report.summary.rules_run > 0);
    assert!(report.summary.rules_run < BUILTIN_RULES.len());
    assert!(report.violations.iter().all(|v| v.rule_id != "image-alt"));
    assert!(
        report
            .violations
            .iter()
            .any(|v| v.rule_id == "landmark-one-main")
    );

    // Disabled-by-default rule semantics
    let disabled_meta = RuleMeta {
        id: "experimental-rule",
        impact: Impact::Minor,
        wcag: "2.4.6 (AAA)",
        tags: &["wcag2aaa"],
        enabled_by_default: false,
        needs_review: false,
        help: "Example",
        description: "Example",
        docs_url: "https://example.com",
    };
    let default_opts = AuditOptions::default();
    assert_eq!(default_opts.effective_level(&disabled_meta), RuleLevel::Off);

    let all_opts = AuditOptions {
        include_disabled: true,
        ..AuditOptions::default()
    };
    assert_eq!(all_opts.effective_level(&disabled_meta), RuleLevel::Error);

    let mut overrides = BTreeMap::new();
    overrides.insert("experimental-rule".to_owned(), RuleLevel::Warn);
    let override_opts = AuditOptions {
        rules: overrides,
        ..AuditOptions::default()
    };
    assert_eq!(
        override_opts.effective_level(&disabled_meta),
        RuleLevel::Warn
    );
}

#[test]
fn ir_extended_fields_backwards_compatible_serde() {
    // Minimal legacy IR JSON without the newly added fields must still deserialize cleanly
    let legacy_json = r##"{
        "nodes": [
            { "id": 0, "parent": null, "children": [1], "tag": "#document" },
            { "id": 1, "parent": 0, "children": [], "tag": "div", "computed": { "display": "block" } }
        ],
        "root": 0
    }"##;
    let doc: Document = serde_json::from_str(legacy_json).expect("legacy IR should deserialize");
    assert!(!doc.css_orientation_locked);
    assert_eq!(doc.nodes[1].frame_tested, None);
    assert_eq!(doc.nodes[1].frame_has_focusable, None);
    assert_eq!(doc.nodes[1].media_duration_secs, None);
    let cs = doc.nodes[1].computed.as_ref().expect("computed exists");
    assert_eq!(cs.text_decoration_line, None);
    assert_eq!(cs.overflow_x, None);
    assert_eq!(cs.scroll_width, None);
}

#[test]
fn conformance_profile_and_w3c_spec_edge_cases() {
    use clearway::ConformanceProfile;

    // 0. ConformanceProfile: placeholder-only input fails in Standard (default), passes in W3cAct
    let placeholder_input = page(r#"<input type="text" placeholder="Search query">"#);
    let standard_report = audit_html(&placeholder_input, &AuditOptions::default());
    assert_eq!(
        standard_report
            .violations
            .iter()
            .filter(|v| v.rule_id == "label")
            .count(),
        1
    );
    let act_report = audit_html(
        &placeholder_input,
        &AuditOptions {
            profile: ConformanceProfile::W3cAct,
            ..AuditOptions::default()
        },
    );
    assert_eq!(
        act_report
            .violations
            .iter()
            .filter(|v| v.rule_id == "label")
            .count(),
        0
    );

    // 1. HTML5 semantics: first <title>, html-has-lang ignores xml:lang-only, meta-refresh +5
    let empty_first_title = "<!doctype html><html lang=\"en\"><head><title></title><title>Second</title></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(
        rule_violations(empty_first_title, "document-title").len(),
        1
    );
    let xml_lang_only = "<!doctype html><html xml:lang=\"en\"><head><title>T</title></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(rule_violations(xml_lang_only, "html-has-lang").len(), 1);
    let plus_refresh = "<!doctype html><html lang=\"en\"><head><title>T</title><meta http-equiv=\"refresh\" content=\"+5\"></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(rule_violations(plus_refresh, "meta-refresh").len(), 0);
    let first_zero_refresh = "<!doctype html><html lang=\"en\"><head><title>T</title><meta http-equiv=\"refresh\" content=\"0\"><meta http-equiv=\"refresh\" content=\"60\"></head><body><main><h1>H</h1></main></body></html>";
    assert_eq!(rule_violations(first_zero_refresh, "meta-refresh").len(), 0);

    // 2. WAI-ARIA: fallback role list, focusable separator, presentational conflict
    let aria_cases = page(
        r#"<div role="bogus button">Fallback valid</div>
           <div role="separator" tabindex="0">Missing valuenow</div>
           <img src="a.png" alt="" aria-labelledby="lbl">
           <span id="lbl">Desc</span>
           <h1 role="none" aria-label=""></h1>
           <div role="list"><div aria-live="polite"><div role="listitem">Broken</div></div></div>"#,
    );
    assert_eq!(rule_violations(&aria_cases, "aria-roles").len(), 0);
    assert_eq!(rule_violations(&aria_cases, "aria-required-attr").len(), 1);
    assert_eq!(
        rule_violations(&aria_cases, "presentation-role-conflict").len(),
        2
    );
    assert_eq!(rule_violations(&aria_cases, "empty-heading").len(), 1);
    assert_eq!(
        rule_violations(&aria_cases, "aria-required-children").len(),
        1
    );

    // 3. WCAG 1.4.12 cascading font-size from <style>
    let cascade_spacing = r#"<!doctype html><html lang="en"><head><title>T</title><style>p { font-size: 10px; }</style></head><body><main><h1>H</h1><div style="line-height: 15px !important;"><p>OK at 1.5 ratio</p></div></main></body></html>"#;
    assert_eq!(
        rule_violations(cascade_spacing, "avoid-inline-spacing").len(),
        0
    );

    // 4. WCAG 2.5.3 whole-word matching ("1" must not match "1a") + parenthetical stripping
    let lin = page(
        r#"<button aria-label="1a">1</button>
           <button aria-label="Next">Next (2)</button>"#,
    );
    assert_eq!(
        rule_violations(&lin, "label-content-name-mismatch").len(),
        1
    );

    // 5. Tables: row-header target for column header + td[headers] override + ARIA grid
    let tables = page(
        r#"<table><tr><th>Name</th><th>Age</th></tr><tr><th scope="row">Alice</th><td>30</td></tr></table>
           <div role="grid"><div role="row"><div role="columnheader">Col</div></div><div role="row"><div role="gridcell">Val</div></div></div>
           <table><tr><th id="h1">H1</th><th id="h2">H2</th></tr><tr><td headers="h1">A</td><td headers="h1">B</td></tr></table>"#,
    );
    assert_eq!(rule_violations(&tables, "th-has-data-cells").len(), 1);
}

#[test]
fn wcag_icon_glyph_accname_2e_and_aria_12_invariants() {
    // 1. WCAG 1.4.3 / 1.4.6 single-char icon glyph exemption on computed contrast
    let mut doc = parse_html(
        r#"<!doctype html><html lang="en"><head><title>T</title></head><body><main><h1>H</h1>
        <button id="icon" aria-label="Close">X</button>
        <p id="text">Low contrast text</p>
        </main></body></html>"#,
    );
    for id_name in ["icon", "text"] {
        if let Some(nid) = doc.find_by_id(id_name)
            && let Some(node) = doc.nodes.get_mut(nid)
        {
            node.computed = Some(ComputedStyle {
                color: Some(Rgba {
                    r: 102,
                    g: 102,
                    b: 102,
                    a: 1.0,
                }),
                background_color: Some(Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 1.0,
                }),
                font_size_px: Some(16.0),
                font_weight: Some(400),
                ..ComputedStyle::default()
            });
        }
    }
    let contrast_viols: Vec<_> = audit_document(&doc, &AuditOptions::default())
        .violations
        .into_iter()
        .filter(|v| v.rule_id == "color-contrast")
        .collect();
    assert_eq!(contrast_viols.len(), 1);
    assert!(contrast_viols[0].selector.contains("#text"));

    // 2. AccName 1.2 Step 2E embedded <select> + progressbar + HTML-AAM <label> on input buttons
    let forms = page(
        r#"<label for="s1"><select><option selected>Chosen</option><option>Other</option></select></label><input id="s1" type="text">
           <div id="e1"><select><option selected></option><option>Other</option></select></div><input id="f1" aria-labelledby="e1">
           <div id="e2"><span role="progressbar" aria-valuenow="23">23%</span></div><input id="f2" aria-labelledby="e2">
           <label>Go<input type="button"></label>
           <label>Pic<input type="image" src="a.png"></label>"#,
    );
    assert_eq!(rule_violations(&forms, "label").len(), 2);
    assert_eq!(rule_violations(&forms, "input-button-name").len(), 0);
    assert_eq!(rule_violations(&forms, "input-image-alt").len(), 0);

    // 3. WAI-ARIA 1.2/1.3 invariants: inert, aria-errormessage single IDREF, svg[role="image"], combobox aria-controls, slider aria-valuetext
    let aria = page(
        r#"<div aria-hidden="true"><div inert><button>Inert</button></div></div>
           <input aria-invalid="true" aria-errormessage="m1 m2"><span id="m1">A</span><span id="m2">B</span>
           <svg role="image" width="10" height="10"><circle r="5"></circle></svg>
           <div role="combobox" aria-expanded="true" aria-owns="m1">Combo</div>
           <div role="slider" aria-valuetext="3 minutes">Slider</div>"#,
    );
    assert_eq!(rule_violations(&aria, "aria-hidden-focus").len(), 0);
    assert_eq!(rule_violations(&aria, "aria-valid-attr-value").len(), 1);
    assert_eq!(rule_violations(&aria, "svg-img-alt").len(), 1);
    assert_eq!(rule_violations(&aria, "aria-required-attr").len(), 1);

    // 4. Meta viewport: maximum-scale=invalid fails AA, device-width passes
    let vp_bad = r#"<!doctype html><html lang="en"><head><title>T</title><meta name="viewport" content="maximum-scale=invalid"></head><body><main><h1>H</h1></main></body></html>"#;
    let vp_dw = r#"<!doctype html><html lang="en"><head><title>T</title><meta name="viewport" content="maximum-scale=device-width"></head><body><main><h1>H</h1></main></body></html>"#;
    assert_eq!(rule_violations(vp_bad, "meta-viewport").len(), 1);
    assert_eq!(rule_violations(vp_dw, "meta-viewport").len(), 0);
}
