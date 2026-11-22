# clearway

[![CI](https://github.com/TheGreatAxios/clearway/actions/workflows/ci.yml/badge.svg)](https://github.com/TheGreatAxios/clearway/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/clearway.svg)](https://crates.io/crates/clearway)
[![docs.rs](https://docs.rs/clearway/badge.svg)](https://docs.rs/clearway)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Fast, browser-agnostic WCAG 2.1/2.2 accessibility checker for HTML, live browsers, CI, and WebAssembly.

## Why Clearway

- **113 built-in rules** covering WCAG 2.0/2.1/2.2, WAI-ARIA 1.2/1.3, and HTML-AAM
- **Runs anywhere**: static HTML CLI, live Chrome/Playwright/CDP snapshots, Rust library, or WebAssembly
- **Honest `violations` vs. `incomplete` split**: deterministic failures fail CI; ambiguous cases like text over photos or text shadows go to review with zero false alarms
- **CI-ready output**: machine-readable `--json` and `--sarif` (GitHub Code Scanning) out of the box

## Quickstart

```sh
cargo install clearway

clearway index.html
curl -s https://example.com | clearway -
clearway index.html --json
clearway index.html --sarif > clearway.sarif
clearway index.html --tags wcag2a,wcag2aa --profile act
```

Sample output:

```text
clearway: 1 violation(s), 0 warning(s), 0 incomplete across 95 rules (7 elements checked)
  [ERROR] (Critical) image-alt — Images must have alternate text
          selector: html > body > main > img
          detail:   Provide an alt attribute (or empty alt="" for decorative images) on every <img> element.
          snippet:  <img src="hero.png">
```

Rust library:

```rust
use clearway::{AuditOptions, audit_html};

let html = r#"<!doctype html><html lang="en"><head><title>Home</title></head><body><main><h1>Hello</h1></main></body></html>"#;
let report = audit_html(html, &AuditOptions::default());
assert_eq!(report.summary.violations, 0);
```

## Clearway Benchmark

Median runtime and finding counts (`v` violations, `i` incomplete) in Headless Chrome across 5 self-contained fixtures (see [`benchmarks/README.md`](benchmarks/README.md) for methodology and custom runners):

| Fixture | Elements | clearway (e2e / engine / v) | axe-core (ms / v) | IBM Equal Access (ms / v) | HTML_CodeSniffer (ms / v) | Siteimprove Alfa (ms / v) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small-landing.html | 124 | 10.5 ms (3.8 ms engine) / 0v | 43.6 ms / 0v | 32.1 ms / 3v / 20i | 5.2 ms / 1v / 3i | 85.5 ms / 8v |
| medium-violations.html | 53 | 8.4 ms (3.5 ms engine) / 17v | 513.7 ms / 14v / 2i | 22.3 ms / 12v / 10i | 4.7 ms / 11v / 3i | 41.2 ms / 20v / 1i |
| large-table.html | 2857 | 97.2 ms (19.5 ms engine) / 0v | 298.5 ms / 0v | 285.2 ms / 0v / 2i | 47.0 ms / 280v | 2960.3 ms / 7v |
| modern-shadow-dom.html | 44 | 8.2 ms (3.9 ms engine) / 5v | 32.1 ms / 2v / 2i | 17.6 ms / 2v / 5i | 4.8 ms / 2v / 2i | 29.0 ms / 4v / 1i |
| contrast-and-layout.html | 51 | 8.2 ms (3.7 ms engine) / 7v / 1i | 33.9 ms / 3v / 6i | 20.2 ms / 3v / 14i | 5.0 ms / 3v / 2i | 40.4 ms / 15v / 4i |
| **Median** | - | **8.4 ms** (3.8 ms engine) | **43.6 ms** | **22.3 ms** | **5.0 ms** | **41.2 ms** |

- **W3C WCAG ACT static suite (631 cases)**: 98.1% accuracy, 97.6% precision, 96.7% recall, 97.2% F1
- **Headless Chrome ACT (99 cases, 5 objective browser rules)**: 97.0% accuracy, 100.0% precision, 90.3% recall, 94.9% F1

## Documentation

- [CLI Reference & CI Integration](docs/cli.md) — flags, exit codes, JSON/SARIF output, and GitHub Code Scanning
- [Live Browsers & WebAssembly](docs/browser-and-wasm.md) — Playwright/Puppeteer/CDP snapshots and `clearway-wasm`
- [Rust Examples](examples/) — `basic.rs`, `browser_ir.rs`, and `batch_scan.rs`
- [Benchmarks](benchmarks/README.md) — fixtures, runner protocol, and reproduction steps
- [API Reference (docs.rs)](https://docs.rs/clearway)

## License

Dual-licensed under MIT (LICENSE-MIT) or Apache-2.0 (LICENSE-APACHE), at your option.
