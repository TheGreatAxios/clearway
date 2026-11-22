use std::time::Instant;

use clearway::{AuditOptions, audit_html};

fn main() {
    let pages = [
        r#"<!doctype html><html lang="en"><head><title>Home</title></head><body><main><h1>Home</h1></main></body></html>"#,
        r#"<!doctype html><html lang="en"><head><title>Pricing</title></head><body><main><h1>Pricing</h1><img src="chart.png"></main></body></html>"#,
        r#"<!doctype html><html><body><a href="/docs"></a></body></html>"#,
    ];

    let options = AuditOptions::default();
    let rounds = 100usize;
    let start = Instant::now();
    let mut total_violations = 0usize;

    for _ in 0..rounds {
        for html in &pages {
            total_violations += audit_html(html, &options).summary.violations;
        }
    }

    let total_audits = rounds * pages.len();
    let elapsed = start.elapsed().as_secs_f64();
    #[allow(clippy::cast_precision_loss)]
    let throughput = total_audits as f64 / elapsed.max(f64::EPSILON);
    println!(
        "Audited {total_audits} pages ({total_violations} total violations) in {:.2}ms ({:.0} audits/sec)",
        elapsed * 1000.0,
        throughput
    );
}
