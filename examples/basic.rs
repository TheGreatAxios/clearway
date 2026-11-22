use clearway::{AuditOptions, audit_html};

fn main() {
    let html = r#"<!doctype html>
<html>
  <body>
    <img src="hero.png">
    <button></button>
  </body>
</html>"#;

    let report = audit_html(html, &AuditOptions::default());
    println!("Found {} violations:", report.summary.violations);
    for v in &report.violations {
        println!("- [{}] {}: {}", v.rule_id, v.selector, v.help);
    }
}
