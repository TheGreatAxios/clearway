//! Focused pass + fail tests for Document, Names, Forms, Lists, and Media rules.

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

fn incomplete(html: &str, rule_id: &str) -> Vec<Violation> {
    audit_html(html, &AuditOptions::default())
        .incomplete
        .into_iter()
        .filter(|v| v.rule_id == rule_id)
        .collect()
}

fn incomplete_with_disabled(html: &str, rule_id: &str) -> Vec<Violation> {
    audit_html(
        html,
        &AuditOptions {
            include_disabled: true,
            ..AuditOptions::default()
        },
    )
    .incomplete
    .into_iter()
    .filter(|v| v.rule_id == rule_id)
    .collect()
}

#[test]
fn registry_has_at_least_seventy_nine_builtin_rules() {
    assert!(BUILTIN_RULES.len() >= 79);
}

#[test]
fn document_rules_milestone3_pass_and_fail() {
    // 1. html-xml-lang-mismatch
    let mismatch = r#"<!doctype html><html lang="en-US" xml:lang="fr-CA"><head><title>T</title></head><body><main><h1>H</h1></main></body></html>"#;
    assert_eq!(violations(mismatch, "html-xml-lang-mismatch").len(), 1);
    let matched = r#"<!doctype html><html lang="en-US" xml:lang="EN-GB"><head><title>T</title></head><body><main><h1>H</h1></main></body></html>"#;
    assert_eq!(violations(matched, "html-xml-lang-mismatch").len(), 0);

    // 2. valid-lang
    let lang_parts = page(
        r#"<p lang="123-bad">Bad</p>
           <span lang="zz">Invalid ISO 639-1</span>
           <p lang="es-419">Good</p>
           <p lang="">Empty allowed</p>"#,
    );
    assert_eq!(violations(&lang_parts, "valid-lang").len(), 2);

    // 3. meta-refresh-no-exceptions (disabled by default; no 20-hour exception)
    let long_refresh = r#"<!doctype html><html lang="en"><head><title>T</title><meta http-equiv="refresh" content="80000"></head><body><main><h1>H</h1></main></body></html>"#;
    assert_eq!(violations(long_refresh, "meta-refresh").len(), 0);
    assert_eq!(
        violations(long_refresh, "meta-refresh-no-exceptions").len(),
        0
    );
    assert_eq!(
        violations_with_disabled(long_refresh, "meta-refresh-no-exceptions").len(),
        1
    );
    let zero_refresh = r#"<!doctype html><html lang="en"><head><title>T</title><meta http-equiv="refresh" content="0"></head><body><main><h1>H</h1></main></body></html>"#;
    assert_eq!(
        violations_with_disabled(zero_refresh, "meta-refresh-no-exceptions").len(),
        0
    );

    // 4. meta-viewport-large (flags 2.0 <= maximum-scale < 5.0; < 2.0 / user-scalable=0 belong to meta-viewport)
    let vp_small = r#"<!doctype html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, maximum-scale=3.0"></head><body><main><h1>H</h1></main></body></html>"#;
    assert_eq!(violations(vp_small, "meta-viewport-large").len(), 1);
    let vp_no_scale = r#"<!doctype html><html lang="en"><head><title>T</title><meta name="viewport" content="user-scalable=0"></head><body><main><h1>H</h1></main></body></html>"#;
    assert_eq!(violations(vp_no_scale, "meta-viewport").len(), 1);
    assert_eq!(violations(vp_no_scale, "meta-viewport-large").len(), 0);
    let vp_large = r#"<!doctype html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, maximum-scale=5.0"></head><body><main><h1>H</h1></main></body></html>"#;
    assert_eq!(violations(vp_large, "meta-viewport-large").len(), 0);

    // 5. bypass
    let no_bypass = r#"<!doctype html><html lang="en"><head><title>T</title></head><body><nav><a href="/">Home</a></nav><div><p>Just a paragraph with no landmark, heading, or skip link.</p></div></body></html>"#;
    assert_eq!(violations(no_bypass, "bypass").len(), 1);
    let no_repeated_block = r#"<!doctype html><html lang="en"><head><title>T</title></head><body><div><p>Just a paragraph with no repeated navigation block.</p></div></body></html>"#;
    assert_eq!(violations(no_repeated_block, "bypass").len(), 0);
    let skip_bypass = r##"<!doctype html><html lang="en"><head><title>T</title></head><body><a href="#content">Skip</a><nav><a href="/">Home</a></nav><div id="content"><p>Body</p></div></body></html>"##;
    assert_eq!(violations(skip_bypass, "bypass").len(), 0);
    let main_bypass = page("<p>Has main and h1</p>");
    assert_eq!(violations(&main_bypass, "bypass").len(), 0);

    // 6. skip-link
    let broken_skip = r##"<!doctype html><html lang="en"><head><title>T</title></head><body><a href="#missing">Skip to main</a><main><h1>H</h1></main></body></html>"##;
    assert_eq!(violations(broken_skip, "skip-link").len(), 1);
    let hidden_skip = r##"<!doctype html><html lang="en"><head><title>T</title></head><body><a href="#sec">Skip</a><div id="sec" aria-hidden="true">Hidden</div><main><h1>H</h1></main></body></html>"##;
    assert_eq!(violations(hidden_skip, "skip-link").len(), 1);
    let valid_skip = r##"<!doctype html><html lang="en"><head><title>T</title></head><body><a href="#main-content">Skip</a><main id="main-content"><h1>H</h1></main></body></html>"##;
    assert_eq!(violations(valid_skip, "skip-link").len(), 0);

    // 7. avoid-inline-spacing (WCAG 1.4.12 thresholds)
    let spacing = page(
        r#"<p style="line-height: 1.2 !important;">Too tight line height</p>
           <p style="letter-spacing: 0.1em !IMPORTANT;">Too tight letter spacing</p>
           <p style="word-spacing: 2px ! important;">Too tight word spacing</p>
           <p style="line-height: 1.5 !important; letter-spacing: 0.12em !important; word-spacing: 0.16em !important;">Meets WCAG 1.4.12</p>
           <p style="line-height: 1.0 !important; line-height: 1.6 !important;">Last !important wins</p>
           <p style="line-height: 1.2; letter-spacing: 0.05em;">Adjustable</p>
           <p style="letter-spacing: initial !important; word-spacing: normal !important; line-height: inherit !important;">CSS keyword resets</p>
           <p style="letter-spacing: unset !important; word-spacing: revert-layer !important; line-height: auto !important;">More CSS keywords</p>"#,
    );
    assert_eq!(violations(&spacing, "avoid-inline-spacing").len(), 3);
}

#[test]
fn name_rules_milestone3_pass_and_fail() {
    // 8. input-button-name
    let input_btns = page(
        r#"<input type="button">
           <input type="submit" value="">
           <input type="reset" value="   ">
           <input type="button" value="Apply">
           <input type="submit">
           <input type="reset">
           <input type="button" aria-label="Close">"#,
    );
    assert_eq!(violations(&input_btns, "input-button-name").len(), 3);

    // 9. role-img-alt
    let role_imgs = page(
        r#"<div role="img"></div>
           <span role="img" aria-label="Star rating"></span>
           <img src="a.png" role="img" alt="Handled by image-alt">"#,
    );
    assert_eq!(violations(&role_imgs, "role-img-alt").len(), 1);

    // 10. object-alt (fallback text inside <object> does not compute its AccName)
    let objects = page(
        r#"<object data="movie.swf"></object>
           <object data="movie.swf" aria-label="Intro video"></object>
           <object data="chart.pdf">Fallback description</object>"#,
    );
    assert_eq!(violations(&objects, "object-alt").len(), 2);

    // 11. summary-name
    let summaries = page(
        "<details><summary></summary><p>Hidden</p></details>
           <details><summary>More info</summary><p>Visible</p></details>",
    );
    assert_eq!(violations(&summaries, "summary-name").len(), 1);

    // 12. frame-title-unique (needs_review -> incomplete by default)
    let frames = page(
        r#"<iframe src="/a" title="Widget"></iframe>
           <iframe src="/b" title=" widget "></iframe>
           <iframe src="/c" title="Distinct"></iframe>"#,
    );
    assert_eq!(violations(&frames, "frame-title-unique").len(), 0);
    assert_eq!(incomplete(&frames, "frame-title-unique").len(), 1);

    // 13. image-redundant-alt
    let redundant = page(
        r#"<a href="/home"><img src="home.png" alt="Home"> Home</a>
           <button><img src="save.png" alt="Save"> save </button>
           <a href="/about"><img src="about.png" alt=""> About</a>
           <a href="/help"><img src="icon.png" alt="Question mark"> Help</a>"#,
    );
    assert_eq!(violations(&redundant, "image-redundant-alt").len(), 2);

    // 14. server-side-image-map
    let ismap = page(
        r##"<a href="/map"><img src="map.png" alt="Map" ismap></a>
           <img src="map.png" alt="Client map" usemap="#m">"##,
    );
    assert_eq!(violations(&ismap, "server-side-image-map").len(), 1);

    // 15. identical-links-same-purpose (disabled by default, needs_review -> incomplete)
    let links = page(
        r#"<a href="/a">Read more</a>
           <a href="/b">read   more</a>
           <a href="/a">Read more</a>
           <a href="/c">Pricing</a>"#,
    );
    assert_eq!(incomplete(&links, "identical-links-same-purpose").len(), 0);
    assert_eq!(
        violations_with_disabled(&links, "identical-links-same-purpose").len(),
        0
    );
    assert_eq!(
        incomplete_with_disabled(&links, "identical-links-same-purpose").len(),
        1
    );
}

#[test]
fn form_rules_milestone3_pass_and_fail() {
    // 16. autocomplete-valid (empty and disabled are inapplicable per ACT 73f2c2)
    let ac = page(
        r#"<form>
             <input id="a" type="text" aria-label="A" autocomplete="not-a-token">
             <input id="b" type="text" aria-label="B" autocomplete="billing invalid-field">
             <input id="c" type="text" aria-label="C" autocomplete="">
             <input id="g" type="text" aria-label="G" autocomplete="not-a-token" disabled>
             <input id="d" type="email" aria-label="D" autocomplete="section-main billing work email">
             <input id="e" type="password" aria-label="E" autocomplete="current-password webauthn">
             <input id="f" type="text" aria-label="F" autocomplete="off">
           </form>"#,
    );
    assert_eq!(violations(&ac, "autocomplete-valid").len(), 2);

    // 17. form-field-multiple-labels (routed to incomplete by default)
    let multi_lbl = page(
        r#"<form>
             <label for="u1">First</label>
             <label for="u1">Second</label>
             <input id="u1" type="text">
             <label for="u2">Only one</label>
             <input id="u2" type="text">
           </form>"#,
    );
    let multi_report = audit_html(&multi_lbl, &AuditOptions::default());
    assert_eq!(
        multi_report
            .violations
            .iter()
            .filter(|v| v.rule_id == "form-field-multiple-labels")
            .count(),
        0
    );
    assert_eq!(
        multi_report
            .incomplete
            .iter()
            .filter(|v| v.rule_id == "form-field-multiple-labels")
            .count(),
        1
    );

    // 18. label-content-name-mismatch (skips <select> options and visually hidden elements)
    let mismatch = page(
        r#"<button aria-label="Submit form">Send</button>
           <button aria-label="Send message now">Send</button>
           <a href="/docs" aria-label="Read Documentation">Documentation</a>
           <a href="/blog" aria-label="Articles">Blog</a>
           <select aria-label="Choose state"><option>Arkansas</option><option>Kansas</option></select>
           <button style="opacity: 0" aria-label="Hidden action">Different text</button>"#,
    );
    assert_eq!(
        violations(&mismatch, "label-content-name-mismatch").len(),
        2
    );

    // 19. label-title-only
    let title_only = page(
        r#"<form>
             <input type="text" title="Search query">
             <span id="hint">Enter code</span>
             <input type="text" aria-describedby="hint">
             <input type="text" aria-label="Search" title="Search query">
             <label for="ok">Name</label><input id="ok" type="text" title="Full name">
           </form>"#,
    );
    assert_eq!(violations(&title_only, "label-title-only").len(), 2);

    // 20. accesskeys
    let keys = page(
        r#"<button accesskey="s">Save</button>
           <button accesskey="S">Submit</button>
           <button accesskey="c">Cancel</button>"#,
    );
    assert_eq!(violations(&keys, "accesskeys").len(), 1);
}

#[test]
fn structure_list_and_duplicate_id_rules_milestone3_pass_and_fail() {
    // 21. definition-list
    let dl = page(
        "<dl><p>Disallowed</p><dt>Term</dt><dd>Def</dd></dl>
           <dl><div><dt>Wrapped term</dt><dd>Wrapped def</dd></div></dl>
           <dl><div><span>Bad inside div</span></div></dl>",
    );
    assert_eq!(violations(&dl, "definition-list").len(), 2);

    // 22. dlitem
    let dlitem = page(
        "<div><dt>Orphan term</dt><dd>Orphan def</dd></div>
           <dl><dt>Good term</dt><dd>Good def</dd></dl>
           <dl><div><dt>Wrapped good</dt><dd>Wrapped good</dd></div></dl>",
    );
    assert_eq!(violations(&dlitem, "dlitem").len(), 2);

    // 23. listitem
    let li = page(
        r#"<div><li>Orphan</li></div>
           <ul><li>In ul</li></ul>
           <ol><li>In ol</li></ol>
           <div role="list"><li>In role list</li></div>"#,
    );
    assert_eq!(violations(&li, "listitem").len(), 1);

    // 24. duplicate-id (disabled by default)
    let dup = page(
        r#"<div id="dup">First</div>
           <div id="dup">Second</div>
           <div id="unique">Unique</div>"#,
    );
    assert_eq!(violations(&dup, "duplicate-id").len(), 0);
    assert_eq!(violations_with_disabled(&dup, "duplicate-id").len(), 1);

    // 25. duplicate-id-active (disabled by default)
    let dup_active = page(
        r#"<button id="act">One</button>
           <button id="act">Two</button>
           <div id="inert">A</div>
           <div id="inert">B</div>"#,
    );
    assert_eq!(violations(&dup_active, "duplicate-id-active").len(), 0);
    assert_eq!(
        violations_with_disabled(&dup_active, "duplicate-id-active").len(),
        1
    );
}

#[test]
fn media_rules_milestone3_pass_and_fail() {
    // 26. blink
    let blink = page("<blink>Flashing</blink><p>Normal</p>");
    assert_eq!(violations(&blink, "blink").len(), 1);

    // 27. marquee
    let marquee = page("<marquee>Scrolling</marquee><p>Normal</p>");
    assert_eq!(violations(&marquee, "marquee").len(), 1);

    // 28. video-caption (needs_review -> incomplete by default)
    let video = page(
        r#"<video src="a.mp4"></video>
           <video src="b.mp4" aria-hidden="true"></video>
           <video src="c.mp4"><track kind="captions" src="c.vtt"></video>"#,
    );
    assert_eq!(violations(&video, "video-caption").len(), 0);
    assert_eq!(incomplete(&video, "video-caption").len(), 1);

    // 29. audio-caption (disabled by default, needs_review -> incomplete)
    let audio = page(
        r#"<audio src="a.mp3"></audio>
           <audio src="b.mp3"><track kind="captions" src="b.vtt"></audio>"#,
    );
    assert_eq!(incomplete(&audio, "audio-caption").len(), 0);
    assert_eq!(violations_with_disabled(&audio, "audio-caption").len(), 0);
    assert_eq!(incomplete_with_disabled(&audio, "audio-caption").len(), 1);
}
