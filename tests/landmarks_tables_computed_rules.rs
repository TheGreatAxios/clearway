//! Focused pass + fail tests for Landmark, Table, and Computed/Runtime rules.

use clearway::{
    AuditOptions, ComputedStyle, Document, Rect, Rgba, Violation, audit_document, audit_html,
    parse_html,
};

fn page(body: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><title>Test</title></head><body><main><h1>Title</h1>{body}</main></body></html>"
    )
}

fn violations(html: &str, rule_id: &str) -> Vec<Violation> {
    audit_html(html, &AuditOptions::default())
        .violations
        .into_iter()
        .filter(|v| v.rule_id == rule_id)
        .collect()
}

fn violations_with_disabled(html: &str, rule_id: &str) -> Vec<Violation> {
    audit_html(
        html,
        &AuditOptions {
            include_disabled: true,
            ..AuditOptions::default()
        },
    )
    .violations
    .into_iter()
    .filter(|v| v.rule_id == rule_id)
    .collect()
}

fn doc_incomplete(doc: &Document, rule_id: &str, include_disabled: bool) -> Vec<Violation> {
    audit_document(
        doc,
        &AuditOptions {
            include_disabled,
            ..AuditOptions::default()
        },
    )
    .incomplete
    .into_iter()
    .filter(|v| v.rule_id == rule_id)
    .collect()
}

fn doc_violations(doc: &Document, rule_id: &str, include_disabled: bool) -> Vec<Violation> {
    audit_document(
        doc,
        &AuditOptions {
            include_disabled,
            ..AuditOptions::default()
        },
    )
    .violations
    .into_iter()
    .filter(|v| v.rule_id == rule_id)
    .collect()
}

#[test]
fn landmark_rules_pass_and_fail() {
    // 1. landmark-banner-is-top-level
    let nested_banner = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><main><div role=\"banner\">Bad</div></main></body></html>";
    assert_eq!(
        violations(nested_banner, "landmark-banner-is-top-level").len(),
        1
    );
    // Per HTML-AAM, <header> inside <main> or <article> is NOT a banner landmark.
    let section_header = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><header>Top</header><main><header>Inner</header></main></body></html>";
    assert_eq!(
        violations(section_header, "landmark-banner-is-top-level").len(),
        0
    );

    // 2. landmark-complementary-is-top-level (disabled by default)
    let nested_aside = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><main><aside>Nested</aside></main></body></html>";
    assert_eq!(
        violations(nested_aside, "landmark-complementary-is-top-level").len(),
        0
    );
    assert_eq!(
        violations_with_disabled(nested_aside, "landmark-complementary-is-top-level").len(),
        1
    );
    let top_aside = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><main><h1>H</h1></main><aside>Side</aside></body></html>";
    assert_eq!(
        violations_with_disabled(top_aside, "landmark-complementary-is-top-level").len(),
        0
    );

    // 3. landmark-contentinfo-is-top-level
    let nested_ci = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><main><div role=\"contentinfo\">Bad</div></main></body></html>";
    assert_eq!(
        violations(nested_ci, "landmark-contentinfo-is-top-level").len(),
        1
    );
    // Per HTML-AAM, <footer> inside <main> or <article> is NOT contentinfo.
    let inner_footer = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><main><footer>Inner</footer></main><footer>Top</footer></body></html>";
    assert_eq!(
        violations(inner_footer, "landmark-contentinfo-is-top-level").len(),
        0
    );

    // 4. landmark-main-is-top-level
    let nested_main = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><aside><main>Bad</main></aside></body></html>";
    assert_eq!(
        violations(nested_main, "landmark-main-is-top-level").len(),
        1
    );
    // Unnamed <section> is not a landmark, so <main> inside it is top-level.
    let main_in_unnamed_section = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><section><main>Ok</main></section></body></html>";
    assert_eq!(
        violations(main_in_unnamed_section, "landmark-main-is-top-level").len(),
        0
    );

    // 5. landmark-no-duplicate-banner
    let dup_banner = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><header>One</header><div role=\"banner\">Two</div><main><h1>H</h1></main></body></html>";
    assert_eq!(
        violations(dup_banner, "landmark-no-duplicate-banner").len(),
        1
    );
    assert_eq!(
        violations(section_header, "landmark-no-duplicate-banner").len(),
        0
    );

    // 6. landmark-no-duplicate-contentinfo
    let dup_ci = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><main><h1>H</h1></main><footer>One</footer><div role=\"contentinfo\">Two</div></body></html>";
    assert_eq!(
        violations(dup_ci, "landmark-no-duplicate-contentinfo").len(),
        1
    );
    assert_eq!(
        violations(inner_footer, "landmark-no-duplicate-contentinfo").len(),
        0
    );

    // 7. landmark-no-duplicate-main
    let dup_main = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><main>One</main><div role=\"main\">Two</div></body></html>";
    assert_eq!(violations(dup_main, "landmark-no-duplicate-main").len(), 1);
    assert_eq!(violations(top_aside, "landmark-no-duplicate-main").len(), 0);

    // 8. landmark-unique
    let dup_navs = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><nav><a href=\"/\">A</a></nav><nav><a href=\"/b\">B</a></nav><main><h1>H</h1></main></body></html>";
    assert_eq!(violations(dup_navs, "landmark-unique").len(), 1);
    let named_navs = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><nav aria-label=\"Primary\"><a href=\"/\">A</a></nav><nav aria-label=\"Footer\"><a href=\"/b\">B</a></nav><main><h1>H</h1></main></body></html>";
    assert_eq!(violations(named_navs, "landmark-unique").len(), 0);

    // 9. region
    let outside_region = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><p>Loose text</p><main><h1>H</h1></main></body></html>";
    assert_eq!(violations(outside_region, "region").len(), 1);
    let with_skip_and_hidden = "<!doctype html><html lang=\"en\"><head><title>T</title></head><body><a href=\"#main\">Skip</a><div aria-hidden=\"true\">Decorative</div><main id=\"main\"><h1>H</h1><p>Inside</p></main></body></html>";
    assert_eq!(violations(with_skip_and_hidden, "region").len(), 0);
}

#[test]
fn table_rules_pass_and_fail() {
    // 10. empty-table-header
    let empty_th = page(
        "<table><tr><th></th><th>Name</th></tr><tr><td>1</td><td>Ada</td></tr></table>
         <table role=\"presentation\"><tr><th></th><td>Layout</td></tr></table>",
    );
    assert_eq!(violations(&empty_th, "empty-table-header").len(), 1);
    let valid_th =
        page("<table><tr><th>ID</th><th>Name</th></tr><tr><td>1</td><td>Ada</td></tr></table>");
    assert_eq!(violations(&valid_th, "empty-table-header").len(), 0);

    // 11. scope-attr-valid
    let bad_scope = page(
        "<table><tr><th scope=\"invalid\">A</th><th scope=\"col\">B</th></tr><tr><td scope=\"row\">1</td><td>2</td></tr></table>",
    );
    assert_eq!(violations(&bad_scope, "scope-attr-valid").len(), 2);
    let good_scope = page(
        "<table><tr><th scope=\"col\">A</th><th scope=\"COLGROUP\">B</th></tr><tr><th scope=\"row\">1</th><td>2</td></tr></table>",
    );
    assert_eq!(violations(&good_scope, "scope-attr-valid").len(), 0);

    // 12. table-duplicate-name
    let dup_table = page(
        "<table summary=\"Quarterly Sales\"><caption>  quarterly sales </caption><tr><th>Q1</th></tr><tr><td>10</td></tr></table>",
    );
    assert_eq!(violations(&dup_table, "table-duplicate-name").len(), 1);
    let diff_table = page(
        "<table summary=\"Sales by quarter in USD\"><caption>Quarterly Sales</caption><tr><th>Q1</th></tr><tr><td>10</td></tr></table>",
    );
    assert_eq!(violations(&diff_table, "table-duplicate-name").len(), 0);

    // 13. table-fake-caption (disabled by default)
    let fake_cap = page(
        "<table><tr><td colspan=\"3\">Fake Title</td></tr><tr><td>A</td><td>B</td><td>C</td></tr></table>",
    );
    assert_eq!(violations(&fake_cap, "table-fake-caption").len(), 0);
    assert_eq!(
        violations_with_disabled(&fake_cap, "table-fake-caption").len(),
        1
    );
    let real_cap = page(
        "<table><caption>Real Title</caption><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>",
    );
    assert_eq!(
        violations_with_disabled(&real_cap, "table-fake-caption").len(),
        0
    );

    // 14. td-has-header (disabled by default; applies to >= 3x3 tables)
    let big_no_header = page(
        "<table>
           <tr><td>A1</td><td>A2</td><td>A3</td></tr>
           <tr><td>B1</td><td>B2</td><td>B3</td></tr>
           <tr><td>C1</td><td>C2</td><td>C3</td></tr>
         </table>",
    );
    assert_eq!(violations(&big_no_header, "td-has-header").len(), 0);
    assert_eq!(
        violations_with_disabled(&big_no_header, "td-has-header").len(),
        1
    );
    let big_with_header = page(
        "<table>
           <tr><th>H1</th><th>H2</th><th>H3</th></tr>
           <tr><td>B1</td><td>B2</td><td>B3</td></tr>
           <tr><td>C1</td><td>C2</td><td>C3</td></tr>
         </table>",
    );
    assert_eq!(
        violations_with_disabled(&big_with_header, "td-has-header").len(),
        0
    );

    // 15. td-headers-attr (reported on the enclosing <table>)
    let bad_headers = page(
        "<table id=\"t-self\"><tr><th id=\"h1\">Col</th></tr><tr><td id=\"c1\" headers=\"c1\">Self</td></tr></table>
         <table id=\"t-cross\"><tr><th id=\"h2\">Col</th></tr><tr><td headers=\"other-table\">Missing</td></tr></table>
         <table><tr><th id=\"other-table\">Other</th></tr><tr><td>X</td></tr></table>",
    );
    assert_eq!(violations(&bad_headers, "td-headers-attr").len(), 2);
    let good_headers =
        page("<table><tr><th id=\"h1\">Col</th></tr><tr><td headers=\"h1\">Val</td></tr></table>");
    assert_eq!(violations(&good_headers, "td-headers-attr").len(), 0);

    // 16. th-has-data-cells
    let orphan_th = page("<table><tr><th>Only Header</th></tr></table>");
    assert_eq!(violations(&orphan_th, "th-has-data-cells").len(), 1);
    assert_eq!(violations(&valid_th, "th-has-data-cells").len(), 0);
}

#[test]
fn frame_focus_and_media_runtime_rules_pass_and_fail() {
    // 18. css-orientation-lock (disabled by default)
    let mut orient_doc = parse_html(&page("<p>Hello</p>"));
    assert_eq!(
        doc_violations(&orient_doc, "css-orientation-lock", true).len(),
        0
    );
    orient_doc.css_orientation_locked = true;
    assert_eq!(
        doc_violations(&orient_doc, "css-orientation-lock", false).len(),
        0
    );
    assert_eq!(
        doc_violations(&orient_doc, "css-orientation-lock", true).len(),
        1
    );

    // 19. focus-order-semantics (disabled by default)
    let focus_html = page(
        "<div tabindex=\"0\">Generic focusable</div>
         <div tabindex=\"0\" role=\"button\">Semantic button</div>
         <button tabindex=\"0\">Native button</button>",
    );
    assert_eq!(violations(&focus_html, "focus-order-semantics").len(), 0);
    assert_eq!(
        violations_with_disabled(&focus_html, "focus-order-semantics").len(),
        1
    );

    // 20. frame-focusable-content
    let mut frame_doc = parse_html(&page(
        "<iframe id=\"f1\" title=\"A\" tabindex=\"-1\"></iframe>
         <iframe id=\"f2\" title=\"B\" tabindex=\"-1\"></iframe>
         <iframe id=\"f3\" title=\"C\"></iframe>",
    ));
    for node in &mut frame_doc.nodes {
        match node.id_attr() {
            Some("f1" | "f3") => node.frame_has_focusable = Some(true),
            Some("f2") => node.frame_has_focusable = Some(false),
            _ => {}
        }
    }
    assert_eq!(
        doc_violations(&frame_doc, "frame-focusable-content", false).len(),
        1
    );

    // 21. frame-tested (needs_review -> incomplete by default)
    let mut tested_doc = parse_html(&page(
        "<iframe id=\"u1\" title=\"Untested\"></iframe>
         <iframe id=\"t1\" title=\"Tested\"></iframe>
         <iframe id=\"n1\" title=\"Unknown\"></iframe>",
    ));
    for node in &mut tested_doc.nodes {
        match node.id_attr() {
            Some("u1") => node.frame_tested = Some(false),
            Some("t1") => node.frame_tested = Some(true),
            _ => {}
        }
    }
    assert_eq!(doc_violations(&tested_doc, "frame-tested", false).len(), 0);
    assert_eq!(doc_incomplete(&tested_doc, "frame-tested", false).len(), 1);

    // 24. no-autoplay-audio (needs_review -> incomplete by default, #t=start,end <= 3s passes)
    let mut media_doc = parse_html(&page(
        "<audio id=\"a-long\" autoplay src=\"a.mp3\"></audio>
         <video id=\"v-unknown\" autoplay src=\"v.mp4\"></video>
         <audio id=\"a-short\" autoplay src=\"s.mp3\"></audio>
         <audio id=\"a-frag-short\" autoplay src=\"clip.mp3#t=10,12.5\"></audio>
         <audio id=\"a-frag-long\" autoplay src=\"clip.mp3#t=10,15\"></audio>
         <audio id=\"a-src-frag-short\" autoplay><source src=\"clip.mp3#t=00:00:01,00:00:03.5\"></audio>
         <audio id=\"a-muted\" autoplay muted src=\"m.mp3\"></audio>
         <video id=\"v-ctrl\" autoplay controls src=\"c.mp4\"></video>",
    ));
    for node in &mut media_doc.nodes {
        match node.id_attr() {
            Some("a-long") => node.media_duration_secs = Some(10.0),
            Some("a-short") => node.media_duration_secs = Some(2.5),
            _ => {}
        }
    }
    assert_eq!(
        doc_violations(&media_doc, "no-autoplay-audio", false).len(),
        0
    );
    assert_eq!(
        doc_incomplete(&media_doc, "no-autoplay-audio", false).len(),
        3
    );
}

#[test]
fn incomplete_tier_override_and_text_shadow_handling() {
    // Explicit RuleLevel::Error override promotes needs_review findings to violations
    let media_doc = parse_html(&page(
        "<audio id=\"a-frag-long\" autoplay src=\"clip.mp3#t=10,15\"></audio>",
    ));
    let mut promote_opts = AuditOptions::default();
    promote_opts
        .rules
        .insert("no-autoplay-audio".to_owned(), clearway::RuleLevel::Error);
    let promoted = audit_document(&media_doc, &promote_opts);
    assert_eq!(
        promoted
            .violations
            .iter()
            .filter(|v| v.rule_id == "no-autoplay-audio")
            .count(),
        1
    );
    assert_eq!(
        promoted
            .incomplete
            .iter()
            .filter(|v| v.rule_id == "no-autoplay-audio")
            .count(),
        0
    );

    // text-shadow routes low-contrast findings to incomplete (both AA and AAA)
    let mut shadow_doc = parse_html(&page(
        "<p id=\"shadow-low\">Shadowed text</p><p id=\"plain-low\">Plain text</p>",
    ));
    for node in &mut shadow_doc.nodes {
        if node.id_attr() == Some("shadow-low") {
            node.computed = Some(ComputedStyle {
                color: Some(Rgba::opaque(0xaa, 0xaa, 0xaa)),
                background_color: Some(Rgba::opaque(0xff, 0xff, 0xff)),
                font_size_px: Some(16.0),
                font_weight: Some(400),
                has_text_shadow: true,
                ..ComputedStyle::default()
            });
        } else if node.id_attr() == Some("plain-low") {
            node.computed = Some(ComputedStyle {
                color: Some(Rgba::opaque(0xaa, 0xaa, 0xaa)),
                background_color: Some(Rgba::opaque(0xff, 0xff, 0xff)),
                font_size_px: Some(16.0),
                font_weight: Some(400),
                ..ComputedStyle::default()
            });
        }
    }
    assert_eq!(
        doc_violations(&shadow_doc, "color-contrast", false).len(),
        1
    );
    assert_eq!(
        doc_incomplete(&shadow_doc, "color-contrast", false).len(),
        1
    );
    assert_eq!(
        doc_violations(&shadow_doc, "color-contrast-enhanced", true).len(),
        1
    );
    assert_eq!(
        doc_incomplete(&shadow_doc, "color-contrast-enhanced", true).len(),
        1
    );
}

#[test]
fn computed_style_and_layout_rules_pass_and_fail() {
    // 17. color-contrast-enhanced (disabled by default; 7.0:1 normal, 4.5:1 large)
    // #767676 on #ffffff is ~4.54:1 (passes AA, fails AAA normal).
    let mut contrast_doc = parse_html(&page("<p id=\"p1\">Medium gray</p><p id=\"p2\">Dark</p>"));
    for node in &mut contrast_doc.nodes {
        if node.id_attr() == Some("p1") {
            node.computed = Some(ComputedStyle {
                color: Some(Rgba::opaque(0x76, 0x76, 0x76)),
                font_size_px: Some(16.0),
                font_weight: Some(400),
                ..ComputedStyle::default()
            });
        } else if node.id_attr() == Some("p2") {
            node.computed = Some(ComputedStyle {
                color: Some(Rgba::opaque(0x11, 0x11, 0x11)),
                font_size_px: Some(16.0),
                font_weight: Some(400),
                ..ComputedStyle::default()
            });
        }
    }
    assert_eq!(
        doc_violations(&contrast_doc, "color-contrast-enhanced", false).len(),
        0
    );
    assert_eq!(
        doc_violations(&contrast_doc, "color-contrast-enhanced", true).len(),
        1
    );

    // 22. hidden-content (disabled by default)
    let mut hidden_doc = parse_html(&page(
        "<div id=\"h1\"><p>Hidden secret</p></div>
         <div id=\"h2\">   </div>
         <div id=\"v1\">Visible</div>",
    ));
    for node in &mut hidden_doc.nodes {
        if matches!(node.id_attr(), Some("h1" | "h2")) {
            node.computed = Some(ComputedStyle {
                display: Some("none".to_owned()),
                ..ComputedStyle::default()
            });
        }
    }
    assert_eq!(
        doc_violations(&hidden_doc, "hidden-content", false).len(),
        0
    );
    assert_eq!(doc_violations(&hidden_doc, "hidden-content", true).len(), 1);

    // 23. link-in-text-block
    let mut link_doc = parse_html(&page(
        "<p id=\"pb\">Read our complete guide covering <a id=\"bad-link\" href=\"/a\">bad link</a> and <a id=\"ul-link\" href=\"/b\">underlined</a> and <a id=\"hc-link\" href=\"/c\">high contrast</a> examples.</p>",
    ));
    for node in &mut link_doc.nodes {
        match node.id_attr() {
            Some("pb") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0, 0, 0)),
                    ..ComputedStyle::default()
                });
            }
            Some("bad-link") => {
                // #222222 vs #000000 is ~1.3:1 (< 3.0) with no underline -> fail
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0x22, 0x22, 0x22)),
                    text_decoration_line: Some("none".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some("ul-link") => {
                // Same color, but underlined -> pass
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0x22, 0x22, 0x22)),
                    text_decoration_line: Some("underline".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some("hc-link") => {
                // #767676 vs #000000 is ~4.6:1 (>= 3.0) -> pass
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0x76, 0x76, 0x76)),
                    text_decoration_line: Some("none".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            _ => {}
        }
    }
    assert_eq!(
        doc_violations(&link_doc, "link-in-text-block", false).len(),
        1
    );
}

#[test]
fn p_as_heading_and_scrollable_region_rules_pass_and_fail() {
    // 25. p-as-heading (disabled by default)
    let mut p_doc = parse_html(&page(
        "<p id=\"fake-h\">Section Title</p>
         <p id=\"body-p\">This is a normal body paragraph with regular styling.</p>
         <p id=\"sentence-p\">This ends with a period.</p>
         <p id=\"body-p2\">Another regular body paragraph.</p>",
    ));
    for node in &mut p_doc.nodes {
        match node.id_attr() {
            Some("fake-h" | "sentence-p") => {
                node.computed = Some(ComputedStyle {
                    font_size_px: Some(24.0),
                    font_weight: Some(700),
                    ..ComputedStyle::default()
                });
            }
            Some("body-p" | "body-p2") => {
                node.computed = Some(ComputedStyle {
                    font_size_px: Some(16.0),
                    font_weight: Some(400),
                    ..ComputedStyle::default()
                });
            }
            _ => {}
        }
    }
    assert_eq!(doc_violations(&p_doc, "p-as-heading", false).len(), 0);
    assert_eq!(doc_violations(&p_doc, "p-as-heading", true).len(), 1);

    // 26. scrollable-region-focusable
    let mut scroll_doc = parse_html(&page(
        "<div id=\"bad-scroll\"><p>Long text</p></div>
         <div id=\"ok-tab\" tabindex=\"0\"><p>Long text</p></div>
         <div id=\"ok-child\"><button>Inside</button></div>
         <div id=\"no-overflow\"><p>Short</p></div>",
    ));
    for node in &mut scroll_doc.nodes {
        match node.id_attr() {
            Some("bad-scroll" | "ok-tab" | "ok-child") => {
                node.computed = Some(ComputedStyle {
                    overflow_y: Some("auto".to_owned()),
                    scroll_height: Some(500.0),
                    client_height: Some(200.0),
                    ..ComputedStyle::default()
                });
            }
            Some("no-overflow") => {
                node.computed = Some(ComputedStyle {
                    overflow_y: Some("auto".to_owned()),
                    scroll_height: Some(200.0),
                    client_height: Some(200.0),
                    ..ComputedStyle::default()
                });
            }
            _ => {}
        }
    }
    assert_eq!(
        doc_violations(&scroll_doc, "scrollable-region-focusable", false).len(),
        1
    );
}

#[test]
fn real_chrome_contrast_and_frame_exemptions() {
    // 1. color-contrast & color-contrast-enhanced: inactive UI controls and their labels
    let mut contrast_doc = parse_html(&page(
        "<button id=\"dis-btn\" disabled>Disabled</button>
         <div id=\"aria-dis\" role=\"button\" aria-disabled=\"true\">Aria disabled</div>
         <fieldset disabled><label id=\"fs-lbl\">Fieldset label <input /></label></fieldset>
         <label id=\"wrap-lbl\">Wrapped <input disabled /></label>
         <label id=\"for-lbl\" for=\"dis-inp\">For label</label><input id=\"dis-inp\" disabled />
         <label id=\"ref-lbl\">Ref label</label><div role=\"textbox\" aria-labelledby=\"ref-lbl\" aria-disabled=\"true\">val</div>
         <button id=\"active-btn\">Active low contrast</button>",
    ));
    for node in &mut contrast_doc.nodes {
        if matches!(
            node.id_attr(),
            Some(
                "dis-btn"
                    | "aria-dis"
                    | "fs-lbl"
                    | "wrap-lbl"
                    | "for-lbl"
                    | "ref-lbl"
                    | "active-btn"
            )
        ) {
            node.computed = Some(ComputedStyle {
                color: Some(Rgba::opaque(0x88, 0x88, 0x88)),
                background_color: Some(Rgba::opaque(0xFF, 0xFF, 0xFF)),
                font_size_px: Some(16.0),
                font_weight: Some(400),
                ..ComputedStyle::default()
            });
        }
    }
    assert_eq!(
        doc_violations(&contrast_doc, "color-contrast", false).len(),
        1
    );
    assert_eq!(
        doc_violations(&contrast_doc, "color-contrast-enhanced", true).len(),
        1
    );

    // 2. frame-focusable-content: skip inert, 1x1px, and behind open <dialog>
    let mut frame_doc = parse_html(&page(
        "<iframe id=\"inert-f\" tabindex=\"-1\" inert title=\"Inert\"></iframe>
         <iframe id=\"tiny-f\" tabindex=\"-1\" title=\"Tiny\"></iframe>
         <iframe id=\"real-f\" tabindex=\"-1\" title=\"Real\"></iframe>",
    ));
    for node in &mut frame_doc.nodes {
        match node.id_attr() {
            Some("inert-f" | "real-f") => {
                node.frame_has_focusable = Some(true);
                node.rect = Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 300.0,
                    height: 150.0,
                });
            }
            Some("tiny-f") => {
                node.frame_has_focusable = Some(true);
                node.rect = Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                });
            }
            _ => {}
        }
    }
    assert_eq!(
        doc_violations(&frame_doc, "frame-focusable-content", false).len(),
        1
    );

    let mut modal_frame_doc = parse_html(&page(
        "<iframe id=\"bg-f\" tabindex=\"-1\" title=\"Behind\"></iframe>
         <dialog open><p>Modal</p></dialog>",
    ));
    for node in &mut modal_frame_doc.nodes {
        if node.id_attr() == Some("bg-f") {
            node.frame_has_focusable = Some(true);
            node.rect = Some(Rect {
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: 150.0,
            });
        }
    }
    assert_eq!(
        doc_violations(&modal_frame_doc, "frame-focusable-content", false).len(),
        0
    );
}

#[test]
fn real_chrome_scroll_link_and_target_exemptions() {
    // 3. scrollable-region-focusable: skip empty spacer container
    let mut spacer_doc = parse_html(&page(
        "<section id=\"empty-scroll\"><div style=\"width:1000px;height:1px\"></div></section>",
    ));
    for node in &mut spacer_doc.nodes {
        if node.id_attr() == Some("empty-scroll") {
            node.computed = Some(ComputedStyle {
                overflow_x: Some("scroll".to_owned()),
                scroll_width: Some(1000.0),
                client_width: Some(500.0),
                ..ComputedStyle::default()
            });
        }
    }
    assert_eq!(
        doc_violations(&spacer_doc, "scrollable-region-focusable", false).len(),
        0
    );

    // 4. link-in-text-block & target-size: standalone nav links vs inline prose links
    let mut link_target_doc = parse_html(&page(
        "<nav><ul><li id=\"nav-li\"><a id=\"nav-link\" href=\"/docs\">Docs</a></li></ul></nav>
         <p id=\"prose-p\">Read the <a id=\"inline-link\" href=\"/guide\">guide</a> today.</p>",
    ));
    for node in &mut link_target_doc.nodes {
        match node.id_attr() {
            Some("nav-li" | "prose-p") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0, 0, 0)),
                    ..ComputedStyle::default()
                });
            }
            Some("nav-link" | "inline-link") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0x22, 0x22, 0x22)),
                    display: Some("inline".to_owned()),
                    text_decoration_line: Some("none".to_owned()),
                    ..ComputedStyle::default()
                });
                node.rect = Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 40.0,
                    height: 18.0,
                });
            }
            _ => {}
        }
    }
    // Only inline-link is in a text block with non-link text (nav-link is standalone)
    assert_eq!(
        doc_violations(&link_target_doc, "link-in-text-block", false).len(),
        1
    );
    // Only nav-link fails target-size (inline-link is exempt per WCAG 2.5.8 inline exception)
    assert_eq!(
        doc_violations(&link_target_doc, "target-size", false).len(),
        1
    );
}

#[test]
fn background_image_contrast_routing_and_opaque_card_occlusion() {
    let mut doc = parse_html(&page(
        "<section id=\"hero\">
           <p id=\"hero-caption\">White text over photo</p>
           <div id=\"opaque-card\"><p id=\"card-text\">White text on white card</p></div>
         </section>",
    ));
    for node in &mut doc.nodes {
        match node.id_attr() {
            Some("hero") => {
                node.computed = Some(ComputedStyle {
                    background_color: Some(Rgba::new(0, 0, 0, 0.0)),
                    has_background_image: true,
                    ..ComputedStyle::default()
                });
            }
            Some("hero-caption") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(255, 255, 255)),
                    background_color: Some(Rgba::new(0, 0, 0, 0.0)),
                    font_size_px: Some(16.0),
                    font_weight: Some(400),
                    ..ComputedStyle::default()
                });
            }
            Some("card-text") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(180, 180, 180)),
                    background_color: Some(Rgba::new(0, 0, 0, 0.0)),
                    font_size_px: Some(16.0),
                    font_weight: Some(400),
                    ..ComputedStyle::default()
                });
            }
            Some("opaque-card") => {
                node.computed = Some(ComputedStyle {
                    background_color: Some(Rgba::opaque(255, 255, 255)),
                    ..ComputedStyle::default()
                });
            }
            _ => {}
        }
    }

    // hero-caption is routed to incomplete; card-text is a hard violation (both AA and AAA).
    let std_violations = doc_violations(&doc, "color-contrast", false);
    let std_incomplete = doc_incomplete(&doc, "color-contrast", false);
    assert_eq!(std_violations.len(), 1);
    assert_eq!(std_violations[0].selector, "#card-text");
    assert_eq!(std_incomplete.len(), 1);
    assert_eq!(std_incomplete[0].selector, "#hero-caption");

    let enhanced_violations = doc_violations(&doc, "color-contrast-enhanced", true);
    let enhanced_incomplete = doc_incomplete(&doc, "color-contrast-enhanced", true);
    assert_eq!(enhanced_violations.len(), 1);
    assert_eq!(enhanced_violations[0].selector, "#card-text");
    assert_eq!(enhanced_incomplete.len(), 1);
    assert_eq!(enhanced_incomplete[0].selector, "#hero-caption");
}

#[test]
fn target_size_wcag_258_spacing_exception_and_sr_only_skip() {
    let mut doc = parse_html(&page(
        "<a id=\"skip\" href=\"#main\">Skip to content</a>
         <a id=\"spaced-1\" href=\"/a\">A</a>
         <a id=\"spaced-2\" href=\"/b\">B</a>
         <a id=\"cramped-1\" href=\"/c\">C</a>
         <a id=\"cramped-2\" href=\"/d\">D</a>",
    ));

    for node in &mut doc.nodes {
        match node.id_attr() {
            Some("skip") => {
                // 1x1px visually-hidden .sr-only skip link next to cramped targets -> exempt
                node.rect = Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                });
                node.computed = Some(ComputedStyle {
                    overflow_x: Some("hidden".to_owned()),
                    overflow_y: Some("hidden".to_owned()),
                    is_clipped: true,
                    ..ComputedStyle::default()
                });
            }
            Some("spaced-1") => {
                // 20x20 at x=0..20 (center x=10); 24px circle spans -2..22
                node.rect = Some(Rect {
                    x: 0.0,
                    y: 40.0,
                    width: 20.0,
                    height: 20.0,
                });
            }
            Some("spaced-2") => {
                // 20x20 at x=28..48 (center x=38); center distance = 28px >= 24px -> passes
                node.rect = Some(Rect {
                    x: 28.0,
                    y: 40.0,
                    width: 20.0,
                    height: 20.0,
                });
            }
            Some("cramped-1") => {
                // 20x20 at x=0..20 (center x=10)
                node.rect = Some(Rect {
                    x: 0.0,
                    y: 100.0,
                    width: 20.0,
                    height: 20.0,
                });
            }
            Some("cramped-2") => {
                // 20x20 at x=22..42 (center x=32); center distance = 22px < 24px -> both fail
                node.rect = Some(Rect {
                    x: 22.0,
                    y: 100.0,
                    width: 20.0,
                    height: 20.0,
                });
            }
            _ => {}
        }
    }

    let fails = doc_violations(&doc, "target-size", false);
    let selectors: Vec<&str> = fails.iter().map(|v| v.selector.as_str()).collect();
    assert_eq!(selectors, vec!["#cramped-1", "#cramped-2"]);
}

#[test]
fn region_collapses_to_highest_non_landmark_ancestor() {
    let html = r#"<!doctype html><html lang="en"><head><title>T</title></head><body>
        <div id="page-wrap">
            <div id="promo-banner">
                <h2>Promo</h2>
                <p>Details</p>
                <a href="/buy">Buy</a>
            </div>
            <main><h1>Main</h1></main>
        </div>
    </body></html>"#;
    let fails = violations(html, "region");
    let selectors: Vec<&str> = fails.iter().map(|v| v.selector.as_str()).collect();
    assert_eq!(selectors, vec!["#promo-banner"]);
}

#[test]
fn link_in_text_block_non_color_distinctions_and_punctuation_separators() {
    let mut doc = parse_html(&page(
        r#"<p id="prose">Read our detailed overview covering <a id="bb" href="/1">bordered</a>, <a id="bs" href="/2">shadowed</a>, <a id="bold" href="/3">bold</a>, and <a id="plain" href="/4">plain</a> links.</p>
           <p id="crumbs"><a id="c1" href="/a">Home</a> / <a id="c2" href="/b">Docs</a> | <a id="c3" href="/c">API</a></p>"#,
    ));
    for node in &mut doc.nodes {
        match node.id_attr() {
            Some("prose" | "crumbs") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0, 0, 0)),
                    font_weight: Some(400),
                    ..ComputedStyle::default()
                });
            }
            Some("bb") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0x22, 0x22, 0x22)),
                    text_decoration_line: Some("none".to_owned()),
                    has_border_bottom: true,
                    ..ComputedStyle::default()
                });
            }
            Some("bs") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0x22, 0x22, 0x22)),
                    text_decoration_line: Some("none".to_owned()),
                    has_box_shadow: true,
                    ..ComputedStyle::default()
                });
            }
            Some("bold") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0x22, 0x22, 0x22)),
                    text_decoration_line: Some("none".to_owned()),
                    font_weight: Some(700),
                    ..ComputedStyle::default()
                });
            }
            Some("plain" | "c1" | "c2" | "c3") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::opaque(0x22, 0x22, 0x22)),
                    text_decoration_line: Some("none".to_owned()),
                    font_weight: Some(400),
                    ..ComputedStyle::default()
                });
            }
            _ => {}
        }
    }
    let fails = doc_violations(&doc, "link-in-text-block", false);
    let selectors: Vec<&str> = fails.iter().map(|v| v.selector.as_str()).collect();
    assert_eq!(selectors, vec!["#plain"]);
}

#[test]
fn visually_hidden_opacity_zero_and_sr_only_skipped_by_visual_rules() {
    let mut doc = parse_html(&page(
        r#"<div id="fade">
             <p id="fade-p">Hidden text <a id="fade-link" href="/x">link</a></p>
             <button id="fade-btn">Go</button>
             <button id="fade-btn-2">Go2</button>
           </div>
           <div id="sr-box">
             <span id="sr-text">Play Video</span>
           </div>
           <span id="clipped-text">Search</span>
           <p id="visible-low">Low contrast</p>"#,
    ));
    let low_fg = Rgba::opaque(180, 180, 180);
    let white_bg = Rgba::opaque(255, 255, 255);
    for node in &mut doc.nodes {
        match node.id_attr() {
            Some("fade") => {
                node.computed = Some(ComputedStyle {
                    opacity: Some(0.0),
                    ..ComputedStyle::default()
                });
            }
            Some("fade-p" | "sr-text" | "visible-low") => {
                node.computed = Some(ComputedStyle {
                    color: Some(low_fg),
                    background_color: Some(white_bg),
                    ..ComputedStyle::default()
                });
            }
            Some("fade-link") => {
                node.computed = Some(ComputedStyle {
                    color: Some(low_fg),
                    text_decoration_line: Some("none".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some("fade-btn") => {
                node.rect = Some(Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 12.0,
                    height: 12.0,
                });
            }
            Some("fade-btn-2") => {
                node.rect = Some(Rect {
                    x: 24.0,
                    y: 10.0,
                    width: 12.0,
                    height: 12.0,
                });
            }
            Some("sr-box") => {
                node.rect = Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                });
                node.computed = Some(ComputedStyle {
                    overflow_x: Some("hidden".to_owned()),
                    overflow_y: Some("hidden".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some("clipped-text") => {
                node.computed = Some(ComputedStyle {
                    color: Some(low_fg),
                    background_color: Some(white_bg),
                    is_clipped: true,
                    ..ComputedStyle::default()
                });
            }
            _ => {}
        }
    }

    let cc_fails = doc_violations(&doc, "color-contrast", false);
    assert_eq!(
        cc_fails
            .iter()
            .map(|v| v.selector.as_str())
            .collect::<Vec<_>>(),
        vec!["#visible-low"]
    );
    let apca_fails = doc_violations(&doc, "apca-contrast", true);
    assert_eq!(
        apca_fails
            .iter()
            .map(|v| v.selector.as_str())
            .collect::<Vec<_>>(),
        vec!["#visible-low"]
    );
    assert_eq!(doc_violations(&doc, "link-in-text-block", false).len(), 0);
    assert_eq!(doc_violations(&doc, "target-size", false).len(), 0);
}

#[test]
fn precision_audit_contrast_and_scroll_regressions() {
    let mut doc = parse_html(&page(
        r#"<p id="trans-fg">Transparent text</p>
           <div id="zero-clip"><span id="zero-clip-txt">Clipped zero width</span></div>
           <select aria-label="State"><option id="opt-el">Arkansas</option></select>
           <span id="emoji-bulb">💡</span>
           <span id="bmp-star">★</span>
           <p id="equal-ratio">White over sibling hero</p>
           <div id="zero-scroll">Off-screen drawer content</div>"#,
    ));

    let white = Rgba::opaque(255, 255, 255);
    let low_gray = Rgba::opaque(180, 180, 180);

    for node in &mut doc.nodes {
        match node.id_attr() {
            Some("trans-fg") => {
                node.computed = Some(ComputedStyle {
                    color: Some(Rgba::new(2, 1, 1, 0.0)),
                    background_color: Some(white),
                    ..ComputedStyle::default()
                });
            }
            Some("zero-clip") => {
                node.rect = Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 20.0,
                });
                node.computed = Some(ComputedStyle {
                    overflow_x: Some("hidden".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some("zero-clip-txt" | "opt-el" | "emoji-bulb" | "bmp-star") => {
                node.computed = Some(ComputedStyle {
                    color: Some(low_gray),
                    background_color: Some(white),
                    ..ComputedStyle::default()
                });
            }
            Some("equal-ratio") => {
                node.computed = Some(ComputedStyle {
                    color: Some(white),
                    background_color: Some(white),
                    ..ComputedStyle::default()
                });
            }
            Some("zero-scroll") => {
                node.rect = Some(Rect {
                    x: -300.0,
                    y: 0.0,
                    width: 0.0,
                    height: 400.0,
                });
                node.computed = Some(ComputedStyle {
                    overflow_y: Some("auto".to_owned()),
                    scroll_height: Some(600.0),
                    client_height: Some(400.0),
                    ..ComputedStyle::default()
                });
            }
            _ => {}
        }
    }

    let cc_fails: Vec<String> = doc_violations(&doc, "color-contrast", false)
        .into_iter()
        .map(|v| v.selector)
        .collect();
    assert_eq!(cc_fails, vec!["#bmp-star"]);
    let cc_inc: Vec<String> = doc_incomplete(&doc, "color-contrast", false)
        .into_iter()
        .map(|v| v.selector)
        .collect();
    assert_eq!(cc_inc, vec!["#equal-ratio"]);
    assert_eq!(
        doc_violations(&doc, "scrollable-region-focusable", false).len(),
        0
    );
}

#[test]
fn precision_audit_link_in_text_block_regressions() {
    let mut doc = parse_html(&page(
        r#"<p id="footer-bar">© 2026 Company | <a id="footer-privacy" href="/p">Privacy Policy</a> | <a id="footer-sitemap" href="/s">Sitemap</a></p>
           <p id="or-bar"><a id="or-a" href="/a">Browse by State</a> or <a id="or-b" href="/b">View Arkansas</a></p>
           <p id="br-block"><a id="br-link" href="/careers"><strong id="br-strong">Careers</strong></a><br>Explore open roles.</p>
           <p id="prose-semi">Read about <a id="semi-link" href="/semi">semibold links</a> in prose.</p>
           <p id="prose-bold">Bold paragraph with <a id="normal-in-bold" href="/nb">normal link</a> inside.</p>
           <p id="prose-bad">Regular paragraph with <a id="true-bad-link" href="/bad">low contrast link</a> inside.</p>"#,
    ));

    let white = Rgba::opaque(255, 255, 255);
    let dark = Rgba::opaque(34, 34, 34);
    let low_diff_blue = Rgba::opaque(0, 51, 204);

    for node in &mut doc.nodes {
        match node.id_attr() {
            Some("footer-bar" | "or-bar" | "br-block" | "prose-semi" | "prose-bad") => {
                node.computed = Some(ComputedStyle {
                    color: Some(dark),
                    background_color: Some(white),
                    font_weight: Some(400),
                    display: Some("block".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some("prose-bold") => {
                node.computed = Some(ComputedStyle {
                    color: Some(dark),
                    background_color: Some(white),
                    font_weight: Some(700),
                    display: Some("block".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some(
                "footer-privacy" | "footer-sitemap" | "or-a" | "or-b" | "br-link"
                | "normal-in-bold" | "true-bad-link",
            ) => {
                node.computed = Some(ComputedStyle {
                    color: Some(low_diff_blue),
                    font_weight: Some(400),
                    display: Some("inline".to_owned()),
                    text_decoration_line: Some("none".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some("br-strong") => {
                node.computed = Some(ComputedStyle {
                    color: Some(low_diff_blue),
                    font_weight: Some(700),
                    display: Some("inline".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some("semi-link") => {
                node.computed = Some(ComputedStyle {
                    color: Some(low_diff_blue),
                    font_weight: Some(600),
                    display: Some("inline".to_owned()),
                    text_decoration_line: Some("none".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            _ => {}
        }
    }

    let litb_fails: Vec<String> = doc_violations(&doc, "link-in-text-block", false)
        .into_iter()
        .map(|v| v.selector)
        .collect();
    assert_eq!(litb_fails, vec!["#true-bad-link"]);
}

#[test]
fn color_contrast_skips_overflow_hidden_offscreen_elements() {
    let mut doc = parse_html(
        r#"<!doctype html><html lang="en"><head><title>T</title></head><body>
           <div id="carousel">
             <button id="visible-slide">Current Month</button>
             <button id="offscreen-slide">Next Month</button>
           </div>
           <div id="scroller">
             <button id="scrolled-item">Scrollable Item</button>
           </div>
           </body></html>"#,
    );
    let low_fg = Rgba::opaque(180, 180, 180);
    let white = Rgba::opaque(255, 255, 255);
    for node in &mut doc.nodes {
        match node.id_attr() {
            Some("carousel") => {
                node.rect = Some(Rect {
                    x: 40.0,
                    y: 100.0,
                    width: 220.0,
                    height: 200.0,
                });
                node.computed = Some(ComputedStyle {
                    background_color: Some(white),
                    overflow_x: Some("hidden".to_owned()),
                    overflow_y: Some("hidden".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some("visible-slide") => {
                node.rect = Some(Rect {
                    x: 50.0,
                    y: 120.0,
                    width: 100.0,
                    height: 30.0,
                });
                node.computed = Some(ComputedStyle {
                    color: Some(low_fg),
                    background_color: Some(white),
                    font_size_px: Some(14.0),
                    font_weight: Some(400),
                    ..ComputedStyle::default()
                });
            }
            Some("offscreen-slide") => {
                node.rect = Some(Rect {
                    x: 261.0,
                    y: 120.0,
                    width: 218.0,
                    height: 30.0,
                });
                node.computed = Some(ComputedStyle {
                    color: Some(low_fg),
                    background_color: Some(white),
                    font_size_px: Some(14.0),
                    font_weight: Some(400),
                    ..ComputedStyle::default()
                });
            }
            Some("scroller") => {
                node.rect = Some(Rect {
                    x: 40.0,
                    y: 320.0,
                    width: 220.0,
                    height: 200.0,
                });
                node.computed = Some(ComputedStyle {
                    background_color: Some(white),
                    overflow_x: Some("auto".to_owned()),
                    overflow_y: Some("auto".to_owned()),
                    ..ComputedStyle::default()
                });
            }
            Some("scrolled-item") => {
                node.rect = Some(Rect {
                    x: 261.0,
                    y: 340.0,
                    width: 120.0,
                    height: 30.0,
                });
                node.computed = Some(ComputedStyle {
                    color: Some(low_fg),
                    background_color: Some(white),
                    font_size_px: Some(14.0),
                    font_weight: Some(400),
                    ..ComputedStyle::default()
                });
            }
            _ => {}
        }
    }

    let fails: Vec<String> = doc_violations(&doc, "color-contrast", false)
        .into_iter()
        .map(|v| v.selector)
        .collect();
    assert_eq!(fails, vec!["#visible-slide", "#scrolled-item"]);
}
