use std::collections::BTreeMap;

use clearway::{
    AuditOptions, ComputedStyle, Rgba, RuleLevel, audit_document, audit_html, parse_html,
    to_sarif_json,
};
use serde_json::Value;

#[test]
fn sarif_2_1_0_maps_errors_warnings_incomplete_and_deduplicated_rules() {
    // 1. Error: img-alt (two images -> deduplicated in tool.driver.rules)
    // 2. Warning: button-name overridden to RuleLevel::Warn
    // 3. Incomplete (note): color-contrast over background-image -> requires visual review
    let mut doc = parse_html(
        r#"<!doctype html><html lang="en"><head><title>SARIF Test</title></head><body>
            <main>
                <img src="/a.png">
                <img src="/b.png">
                <button></button>
                <p id="hero">Low contrast over photo</p>
            </main>
        </body></html>"#,
    );

    if let Some(hero_id) = doc.find_by_id("hero")
        && let Some(node) = doc.nodes.get_mut(hero_id)
    {
        node.computed = Some(ComputedStyle {
            color: Some(Rgba::opaque(200, 200, 200)),
            background_color: Some(Rgba::opaque(255, 255, 255)),
            has_background_image: true,
            ..ComputedStyle::default()
        });
    }

    let mut rules = BTreeMap::new();
    rules.insert("button-name".to_owned(), RuleLevel::Warn);
    let options = AuditOptions {
        rules,
        ..AuditOptions::default()
    };

    let report = audit_document(&doc, &options);
    let sarif_str = to_sarif_json(&report, "public/index.html").expect("valid SARIF JSON");
    let sarif: Value = serde_json::from_str(&sarif_str).expect("parseable JSON");

    assert_eq!(
        sarif["$schema"],
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    assert_eq!(sarif["version"], "2.1.0");

    let run = &sarif["runs"][0];
    let driver = &run["tool"]["driver"];
    assert_eq!(driver["name"], "clearway");
    assert_eq!(driver["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        driver["informationUri"],
        "https://github.com/TheGreatAxios/clearway"
    );

    let driver_rules = driver["rules"].as_array().expect("driver.rules array");
    let img_alt_rules = driver_rules
        .iter()
        .filter(|r| r["id"] == "image-alt")
        .count();
    assert_eq!(img_alt_rules, 1, "driver.rules must deduplicate rule IDs");

    let first_rule = &driver_rules[0];
    assert!(first_rule["shortDescription"]["text"].is_string());
    assert!(first_rule["helpUri"].is_string());
    assert!(first_rule["properties"]["tags"].is_array());

    let results = run["results"].as_array().expect("results array");
    let has_error = results
        .iter()
        .any(|r| r["ruleId"] == "image-alt" && r["level"] == "error");
    let has_warning = results
        .iter()
        .any(|r| r["ruleId"] == "button-name" && r["level"] == "warning");
    let has_note = results
        .iter()
        .any(|r| r["ruleId"] == "color-contrast" && r["level"] == "note");

    assert!(has_error, "expected image-alt error result in SARIF");
    assert!(has_warning, "expected button-name warning result in SARIF");
    assert!(has_note, "expected color-contrast note result in SARIF");

    for result in results {
        assert_eq!(
            result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "public/index.html"
        );
        assert!(
            result["locations"][0]["logicalLocations"][0]["fullyQualifiedName"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        );
        assert!(
            result["properties"]["htmlSnippet"]
                .as_str()
                .is_some_and(|s| s.starts_with('<'))
        );
    }

    // Also verify clean HTML produces valid empty-results SARIF 2.1.0
    let clean = audit_html(
        r#"<!doctype html><html lang="en"><head><title>Clean</title></head><body><main><h1>Hello</h1></main></body></html>"#,
        &AuditOptions::default(),
    );
    let clean_sarif: Value =
        serde_json::from_str(&to_sarif_json(&clean, "stdin").expect("clean SARIF"))
            .expect("valid JSON");
    assert_eq!(
        clean_sarif["runs"][0]["results"].as_array().map(Vec::len),
        Some(0)
    );
}
