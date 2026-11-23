//! Focused pass + fail tests for all 22 WAI-ARIA rules added in Milestone 2.

use clearway::{AuditOptions, BUILTIN_RULES, Violation, audit_html};

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

#[test]
fn registry_has_at_least_fifty_builtin_rules() {
    assert!(BUILTIN_RULES.len() >= 50);
}

#[test]
fn aria_name_rules_pass_and_fail() {
    // 1. aria-command-name
    let cmd = page(
        r#"<div role="button"></div>
           <div role="link"></div>
           <div role="menuitem"></div>
           <div role="button" aria-label="Save"></div>
           <div role="link">Home</div>
           <div role="menuitem">Open</div>"#,
    );
    assert_eq!(violations(&cmd, "aria-command-name").len(), 3);

    // 2. aria-dialog-name
    let dialog = page(
        r#"<div role="dialog">Body only</div>
           <div role="alertdialog"></div>
           <div role="dialog" aria-label="Settings">Body</div>
           <div role="alertdialog" aria-labelledby="dlg-title"><h2 id="dlg-title">Confirm</h2></div>"#,
    );
    assert_eq!(violations(&dialog, "aria-dialog-name").len(), 2);

    // 3. aria-input-field-name
    let inputs = page(
        r#"<div role="textbox"></div>
           <div role="combobox" aria-expanded="false"></div>
           <div role="slider" aria-valuenow="5"></div>
           <div role="textbox" aria-label="Bio"></div>
           <div role="searchbox" aria-label="Filter"></div>"#,
    );
    assert_eq!(violations(&inputs, "aria-input-field-name").len(), 3);

    // 4. aria-meter-name
    let meter = page(
        r#"<div role="meter" aria-valuenow="50"></div>
           <div role="meter" aria-valuenow="50" aria-label="CPU usage"></div>"#,
    );
    assert_eq!(violations(&meter, "aria-meter-name").len(), 1);

    // 5. aria-progressbar-name
    let progress = page(
        r#"<div role="progressbar"></div>
           <div role="progressbar" aria-label="Upload progress"></div>"#,
    );
    assert_eq!(violations(&progress, "aria-progressbar-name").len(), 1);

    // 6. aria-tab-name
    let tab = page(
        r#"<div role="tablist">
             <div role="tab"></div>
             <div role="tab">Overview</div>
           </div>"#,
    );
    assert_eq!(violations(&tab, "aria-tab-name").len(), 1);

    // 7. aria-toggle-field-name (skips native <input> and non-toggle containers like menu/radiogroup)
    let toggle = page(
        r#"<div role="checkbox" aria-checked="false"></div>
           <div role="switch" aria-checked="true"></div>
           <ul role="menu"><li role="menuitem">Item</li></ul>
           <div role="radiogroup"><div role="radio" aria-checked="false">A</div></div>
           <div role="checkbox" aria-checked="true" aria-label="Accept"></div>
           <input type="checkbox" role="checkbox">"#,
    );
    assert_eq!(violations(&toggle, "aria-toggle-field-name").len(), 2);

    // 8. aria-tooltip-name
    let tooltip = page(
        r#"<div role="tooltip"></div>
           <div role="tooltip">Helpful hint</div>"#,
    );
    assert_eq!(violations(&tooltip, "aria-tooltip-name").len(), 1);

    // 9. aria-treeitem-name
    let tree = page(
        r#"<div role="tree">
             <div role="treeitem"></div>
             <div role="treeitem">Folder A</div>
           </div>"#,
    );
    assert_eq!(violations(&tree, "aria-treeitem-name").len(), 1);
}

#[test]
fn aria_allowed_attr_and_role_pass_and_fail() {
    // 10. aria-allowed-attr
    let allowed_attr = page(
        r#"<div aria-pressed="true">Bad</div>
           <button aria-pressed="true">Good</button>
           <button aria-checked="true">Bad</button>
           <div aria-live="polite">Good global</div>"#,
    );
    assert_eq!(violations(&allowed_attr, "aria-allowed-attr").len(), 2);

    // 11. aria-allowed-role
    let allowed_role = r#"<!doctype html><html lang="en"><head>
        <title>Test</title>
        <meta name="x" role="button">
        <link rel="stylesheet" href="/a.css" role="navigation">
      </head><body>
        <main role="button"><h1>Title</h1>
          <br role="button">
          <br role="presentation">
          <input type="hidden" role="textbox">
          <div role="button" aria-label="Ok">Ok</div>
        </main>
      </body></html>"#;
    assert_eq!(violations(allowed_role, "aria-allowed-role").len(), 5);
}

#[test]
fn aria_braille_conditional_deprecated_hidden_body_pass_and_fail() {
    // 12. aria-braille-equivalent
    let braille = page(
        r#"<button aria-braillelabel="btn"></button>
           <button aria-braillelabel="btn">Save</button>
           <div role="region" aria-label="Slide" aria-brailleroledescription="sld"></div>
           <div role="region" aria-label="Slide" aria-roledescription="slide" aria-brailleroledescription="sld"></div>"#,
    );
    assert_eq!(violations(&braille, "aria-braille-equivalent").len(), 2);

    // 13. aria-conditional-attr
    let cond = page(
        r#"<input type="checkbox" aria-checked="true" aria-label="Bad">
           <input type="checkbox" checked aria-label="Good">
           <table><tr aria-posinset="1" aria-setsize="2"><td colspan="2" aria-colspan="2">A</td></tr></table>
           <table><tr><td colspan="2">B</td></tr></table>"#,
    );
    assert_eq!(violations(&cond, "aria-conditional-attr").len(), 3);

    // 14. aria-deprecated-role
    let deprecated = page(
        r#"<div role="directory">Old</div>
           <div role="list"><div role="listitem">New</div></div>"#,
    );
    assert_eq!(violations(&deprecated, "aria-deprecated-role").len(), 1);

    // 15. aria-hidden-body
    let hidden_body = r#"<!doctype html><html lang="en"><head><title>T</title></head><body aria-hidden="true"><main><h1>Hi</h1></main></body></html>"#;
    assert_eq!(violations(hidden_body, "aria-hidden-body").len(), 1);
    let visible_body = page("<p>Visible</p>");
    assert_eq!(violations(&visible_body, "aria-hidden-body").len(), 0);
}

#[test]
fn aria_prohibited_children_parent_pass_and_fail() {
    // 16. aria-prohibited-attr: explicit role="presentation" is a hard violation;
    // generic/native non-widget naming without explicit role routes to `incomplete`;
    // aria-hidden="true" elements are skipped.
    let prohibited = page(
        r#"<div aria-label="Generic label">Text</div>
           <p aria-label="Paragraph label">Text</p>
           <span role="presentation" aria-label="Presentational label">Text</span>
           <div aria-hidden="true" aria-label="Hidden generic">Text</div>
           <div role="region" aria-label="Landmark label">Text</div>"#,
    );
    let prohib_report = audit_html(&prohibited, &AuditOptions::default());
    assert_eq!(
        prohib_report
            .violations
            .iter()
            .filter(|v| v.rule_id == "aria-prohibited-attr")
            .count(),
        1
    );
    assert_eq!(
        prohib_report
            .incomplete
            .iter()
            .filter(|v| v.rule_id == "aria-prohibited-attr")
            .count(),
        2
    );

    // 17. aria-required-children (including aria-owns & aria-busy="true")
    let req_children = page(
        r#"<div role="list"><p>Not a listitem</p></div>
           <div role="list"><div role="listitem">Item</div></div>
           <div role="tablist" aria-owns="owned-tab"></div>
           <div id="owned-tab" role="tab">Owned Tab</div>
           <div role="list" aria-busy="true"></div>"#,
    );
    assert_eq!(violations(&req_children, "aria-required-children").len(), 1);

    // 18. aria-required-parent (including aria-owns)
    let req_parent = page(
        r#"<div role="tab">Orphan tab</div>
           <div role="tablist"><div role="tab">Good tab</div></div>
           <div role="list" aria-owns="owned-li"></div>
           <div id="owned-li" role="listitem">Owned item</div>"#,
    );
    assert_eq!(violations(&req_parent, "aria-required-parent").len(), 1);
}

#[test]
fn aria_roledescription_text_valid_attr_value_and_presentation_conflict() {
    // 19. aria-roledescription (disabled by default, runs with include_disabled)
    let roledesc = page(
        r#"<div aria-roledescription="slide">Generic fails</div>
           <section role="region" aria-label="S1" aria-roledescription="slide">Semantic passes</section>"#,
    );
    assert_eq!(violations(&roledesc, "aria-roledescription").len(), 0);
    assert_eq!(
        violations_with_disabled(&roledesc, "aria-roledescription").len(),
        1
    );

    // 20. aria-text
    let aria_text = page(
        r#"<div role="text"><button>Focusable child</button></div>
           <div role="text"><span>Plain text</span></div>"#,
    );
    assert_eq!(violations(&aria_text, "aria-text").len(), 1);

    // 21. aria-valid-attr-value
    let attr_val = page(
        r#"<div aria-hidden="maybe">Bad bool</div>
           <button aria-pressed="definitely">Bad tristate</button>
           <button aria-labelledby="nonexistent-id">Bad IDREF</button>
           <span id="ok-id">Label</span>
           <button aria-labelledby="ok-id" aria-pressed="true">Valid</button>"#,
    );
    assert_eq!(violations(&attr_val, "aria-valid-attr-value").len(), 3);

    // 22. presentation-role-conflict
    let conflict = page(
        r#"<button role="none">Focusable</button>
           <div role="presentation" tabindex="0">Tabindex</div>
           <div role="none" aria-label="Conflict">Global ARIA</div>
           <div role="presentation">Clean</div>"#,
    );
    assert_eq!(violations(&conflict, "presentation-role-conflict").len(), 3);
}

#[test]
fn html_aam_aria_precision_and_violation_attribution() {
    // Case-sensitive ARIA state values and ARIA 1.3 roles/attrs.
    let aria_case = page(
        r#"<div role="checkbox" aria-checked="TRUE">Bad case</div>
           <div role="comment" aria-level="1" aria-posinset="1" aria-setsize="2">Ok</div>
           <div role="alert" aria-expanded="false" aria-disabled="false">Ok</div>"#,
    );
    assert_eq!(violations(&aria_case, "aria-allowed-attr").len(), 1);
    assert_eq!(violations(&aria_case, "aria-roles").len(), 0);

    // DPUB host-element restrictions, non-first <summary>, and checkbox role="button" without aria-pressed.
    let dpub = page(
        r##"<div role="doc-backlink">Bad host</div>
           <a href="#" role="doc-backlink">Good host</a>
           <details><summary role="button">First</summary><summary role="tab">Second</summary></details>
           <input type="checkbox" role="button" aria-haspopup="true" />
           <input type="checkbox" role="button" aria-pressed="false" />"##,
    );
    assert_eq!(violations(&dpub, "aria-allowed-role").len(), 2);

    // Conditional checkbox/radio/row attributes.
    let cond = page(
        r#"<input type="checkbox" aria-checked="false" />
           <input type="checkbox" aria-checked="true" checked />
           <input type="radio" aria-checked="true" />
           <div role="grid"><div role="row" aria-expanded="false">Row</div></div>"#,
    );
    assert_eq!(violations(&cond, "aria-conditional-attr").len(), 2);

    // HTML-AAM prohibited naming on no-role elements (incomplete) vs name-contributing span inside button (pass).
    let prohib = page(
        r#"<cite aria-label="Author">Name</cite>
           <div role="button"><span aria-label="Save"></span></div>"#,
    );
    let prohib_rep = audit_html(&prohib, &AuditOptions::default());
    assert_eq!(
        prohib_rep
            .violations
            .iter()
            .filter(|v| v.rule_id == "aria-prohibited-attr")
            .count(),
        0
    );
    assert_eq!(
        prohib_rep
            .incomplete
            .iter()
            .filter(|v| v.rule_id == "aria-prohibited-attr")
            .count(),
        1
    );

    // Cross-document duplicate-id-aria: same ID in parent doc and flattened iframe does not collide.
    let cross_doc = page(
        r#"<label for="shared">Outer</label><input id="shared" type="text">
           <iframe title="Widget"><label for="shared">Inner</label><input id="shared" type="text"></iframe>"#,
    );
    assert_eq!(violations(&cross_doc, "duplicate-id-aria").len(), 0);

    // First-element attribution on landmark-unique and <label> naming on <button>.
    let attr_doc = page(
        r#"<nav id="n1">One</nav><nav id="n2">Two</nav>
           <label for="b1">Submit</label><button id="b1"></button>"#,
    );
    let lm = violations(&attr_doc, "landmark-unique");
    assert_eq!(lm.len(), 1);
    assert_eq!(lm[0].selector, "#n1");
    assert_eq!(violations(&attr_doc, "button-name").len(), 0);
}

#[test]
fn button_name_aria_command_no_double_count_and_nested_menuitem() {
    // Non-native [role="button"] is reported only by aria-command-name, not button-name.
    // Native <button role="button"> is reported only by button-name, not aria-command-name.
    let doc = page(
        r#"<a id="a-btn" role="button"></a>
           <button id="native-btn" role="button"></button>"#,
    );
    let btn_fails = violations(&doc, "button-name");
    let cmd_fails = violations(&doc, "aria-command-name");
    assert_eq!(
        btn_fails
            .iter()
            .map(|v| v.selector.as_str())
            .collect::<Vec<_>>(),
        vec!["#native-btn"]
    );
    assert_eq!(
        cmd_fails
            .iter()
            .map(|v| v.selector.as_str())
            .collect::<Vec<_>>(),
        vec!["#a-btn"]
    );

    // Non-focusable <li role="menuitem"> wrapping <a href> is not nested-interactive,
    // while a focusable <li role="menuitem" tabindex="0"> wrapping <a href> is.
    let menu = page(
        r#"<ul role="menu">
             <li id="non-focusable" role="menuitem"><a href="/about">About</a></li>
             <li id="focusable" role="menuitem" tabindex="0"><a href="/contact">Contact</a></li>
           </ul>"#,
    );
    let nested_fails = violations(&menu, "nested-interactive");
    assert_eq!(
        nested_fails
            .iter()
            .map(|v| v.selector.as_str())
            .collect::<Vec<_>>(),
        vec!["#focusable"]
    );
}
