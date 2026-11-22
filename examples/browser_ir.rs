use clearway::{AuditOptions, ComputedStyle, Rect, Rgba, audit_document, parse_html};

fn main() {
    let html = r#"<!doctype html>
<html lang="en">
  <head><title>Browser IR Example</title></head>
  <body>
    <main>
      <h1>Controls</h1>
      <button id="save">Save</button>
      <button id="cancel">Cancel</button>
    </main>
  </body>
</html>"#;

    let mut doc = parse_html(html);
    for node in &mut doc.nodes {
        if node.tag == "button" {
            let x = if node.attr("id") == Some("save") {
                0.0
            } else {
                18.0
            };
            node.computed = Some(ComputedStyle {
                color: Some(Rgba::opaque(180, 180, 180)),
                background_color: Some(Rgba::opaque(255, 255, 255)),
                font_size_px: Some(14.0),
                ..ComputedStyle::default()
            });
            node.rect = Some(Rect {
                x,
                y: 0.0,
                width: 16.0,
                height: 16.0,
            });
        }
    }

    let report = audit_document(&doc, &AuditOptions::default());
    for v in &report.violations {
        println!("[{}] {}: {}", v.rule_id, v.selector, v.help);
    }
}
