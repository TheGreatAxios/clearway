# clearway

[![CI](https://github.com/TheGreatAxios/clearway/actions/workflows/ci.yml/badge.svg)](https://github.com/TheGreatAxios/clearway/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/clearway.svg)](https://crates.io/crates/clearway)
[![docs.rs](https://docs.rs/clearway/badge.svg)](https://docs.rs/clearway)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

`clearway` is a browser-agnostic, pure-Rust accessibility engine for HTML5 documents and live browser DOM snapshots, built directly on W3C WCAG 2.0/2.1/2.2, WAI-ARIA 1.2/1.3, HTML-AAM, AccName 1.2, and W3C ACT Rules. It ships with 113 built-in rules: 105 core WCAG, WAI-ARIA, and best-practice rules plus 8 extended modern-web rules.

The library core is synchronous, has 4 pure-Rust runtime dependencies (`ego-tree`, `scraper`, `serde`, `serde_json`), forbids `unsafe` code, and compiles to native targets and `wasm32-unknown-unknown`. Deterministic failures report as `violations`, while ambiguous cases such as text over background images or non-provable gradients report as `incomplete` for human review.

## Quickstart

```sh
cargo install clearway
clearway index.html
curl -s https://example.com | clearway -
```

Sample output:

```text
clearway: 1 violation(s), 0 warning(s), 0 incomplete across 95 rules (7 elements checked)
  [ERROR] (Critical) image-alt — Images must have alternate text
          selector: html > body > main > img
          detail:   Provide an alt attribute (or empty alt="" for decorative images) on every <img> element.
          snippet:  <img src="hero.png">
```

## Clearway Benchmark

The repository includes a pluggable benchmark harness in [`benchmarks/run.mjs`](benchmarks/run.mjs) backed by a ground-truth fixture manifest in [`benchmarks/cases.json`](benchmarks/cases.json). Only `clearway`'s own runner ships in the repo; external engines plug in via `--runner <path>` (see [`benchmarks/README.md`](benchmarks/README.md)).

Median runtime and finding counts (`v` violations, `i` incomplete) in Headless Chrome across 5 self-contained fixtures (1 warmup + 3 timed runs per fixture):

| Fixture | Elements | clearway (e2e / engine / v) | axe-core (ms / v) | IBM Equal Access (ms / v) | HTML_CodeSniffer (ms / v) | Siteimprove Alfa (ms / v) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small-landing.html | 124 | 10.5 ms (3.8 ms engine) / 0v | 43.6 ms / 0v | 32.1 ms / 3v / 20i | 5.2 ms / 1v / 3i | 85.5 ms / 8v |
| medium-violations.html | 53 | 8.4 ms (3.5 ms engine) / 17v | 513.7 ms / 14v / 2i | 22.3 ms / 12v / 10i | 4.7 ms / 11v / 3i | 41.2 ms / 20v / 1i |
| large-table.html | 2857 | 97.2 ms (19.5 ms engine) / 0v | 298.5 ms / 0v | 285.2 ms / 0v / 2i | 47.0 ms / 280v | 2960.3 ms / 7v |
| modern-shadow-dom.html | 44 | 8.2 ms (3.9 ms engine) / 5v | 32.1 ms / 2v / 2i | 17.6 ms / 2v / 5i | 4.8 ms / 2v / 2i | 29.0 ms / 4v / 1i |
| contrast-and-layout.html | 51 | 8.2 ms (3.7 ms engine) / 7v / 1i | 33.9 ms / 3v / 6i | 20.2 ms / 3v / 14i | 5.0 ms / 3v / 2i | 40.4 ms / 15v / 4i |
| **Median** | - | **8.4 ms** (3.8 ms engine) | **43.6 ms** | **22.3 ms** | **5.0 ms** | **41.2 ms** |

W3C WCAG ACT conformance:

- **W3C WCAG ACT static suite (631 cases)**: 98.1% accuracy, 97.6% precision, 96.7% recall, 97.2% F1
- **Headless Chrome ACT (99 cases, 5 objective browser rules)**: 97.0% accuracy, 100.0% precision, 90.3% recall, 94.9% F1

## CLI options

```sh
# Emit structured JSON (accepts stdin with "-")
cat index.html | clearway - --json

# Emit SARIF 2.1.0 for GitHub Code Scanning
clearway index.html --sarif > clearway.sarif

# Audit a live browser Document IR snapshot
clearway snapshot.json --ir --json

# Filter by WCAG, ACT, best-practice, or clearway tags
clearway index.html --tags wcag2a,wcag2aa
clearway index.html --tags clearway

# Switch conformance profile (standard default, or strict W3C ACT / AccName 1.2)
clearway index.html --profile standard
clearway index.html --profile act

# Enable rules that are off by default (wcag2aaa, experimental, obsolete, opt-in extensions)
clearway index.html --all-rules

# List all 113 built-in rules
clearway --rules

# Benchmark HTML5 parsing and the full rule suite
clearway index.html --bench 1000
```

## Rust library and examples

```rust
use clearway::{AuditOptions, audit_html};

let html = r#"<!doctype html><html lang="en"><head><title>Home</title></head><body><main><h1>Hello</h1></main></body></html>"#;
let report = audit_html(html, &AuditOptions::default());
assert_eq!(report.summary.violations, 0);
```

Runnable examples live in [`examples/`](examples/):

- [`examples/basic.rs`](examples/basic.rs): audit an HTML string and inspect violations (`cargo run --example basic`)
- [`examples/browser_ir.rs`](examples/browser_ir.rs): audit a serialized browser Document IR (`cargo run --example browser_ir`)
- [`examples/batch_scan.rs`](examples/batch_scan.rs): scan multiple HTML files in one pass (`cargo run --example batch_scan`)

## Browser snapshots, WebAssembly, and coding agents

- [`collectors/browser-snapshot.js`](collectors/browser-snapshot.js) runs in Playwright, Puppeteer, or CDP `Runtime.evaluate` to serialize computed styles, bounding rects, open Shadow DOM trees, same-origin iframes, `z-index` contrast layers, and CSS gradients into the JSON IR consumed by `--ir` or `audit_document`.
- [`wasm/`](wasm/) builds the `wasm-bindgen` package (`clearway-wasm`) for browsers, extensions, Cloudflare Workers, Node, Bun, and Deno.
- Coding-agent loops and CI pipelines can run `clearway --json` for deterministic rule IDs, selectors, and snippets, or `clearway --sarif` to upload findings directly to GitHub Code Scanning.

## License

Dual-licensed under MIT (LICENSE-MIT) or Apache-2.0 (LICENSE-APACHE), at your option.
