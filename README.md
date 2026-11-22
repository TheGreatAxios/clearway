# clearway

[![Crates.io](https://img.shields.io/crates/v/clearway.svg)](https://crates.io/crates/clearway)
[![docs.rs](https://img.shields.io/docsrs/clearway)](https://docs.rs/clearway)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Fast, browser-agnostic WCAG 2.1/2.2 accessibility checker for HTML, live browsers, CI, and WebAssembly.

## Why Clearway

- **113 built-in rules** covering WCAG 2.0/2.1/2.2, WAI-ARIA 1.2/1.3, and HTML-AAM
- **Runs anywhere**: static HTML CLI, live Chrome/Playwright/CDP snapshots, Rust library, or WebAssembly
- **Honest `violations` vs. `incomplete` split**: deterministic failures fail CI; ambiguous cases like text over photos or text shadows go to review
- **CI-ready output**: machine-readable `--json` and `--sarif` (GitHub Code Scanning) out of the box

## Quickstart

Install from `crates.io` (or from a local checkout with `cargo install --path .`):

```sh
cargo install clearway

clearway index.html
curl -s https://example.com | clearway -
clearway index.html --json
clearway index.html --sarif > clearway.sarif
clearway index.html --tags wcag2a,wcag2aa
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

## Benchmark

Measured locally in static HTML5 mode (1 warmup + 3 timed runs, median reported) across the 5 self-contained fixtures in `benchmarks/fixtures/`:

```sh
cargo build --release && node benchmarks/run.mjs --runner benchmarks/runners/clearway.mjs
```

| Fixture | Elements | Expected bounds | Violations | Median latency |
| --- | ---: | ---: | ---: | ---: |
| `small-landing.html` | 123 | 0 | 0 | 3.5 ms |
| `medium-violations.html` | 52 | 12–30 | 15 | 4.1 ms |
| `large-table.html` | 2856 | 0 | 0 | 9.8 ms |
| `modern-shadow-dom.html` | 31 | 3–15 | 3 | 2.1 ms |
| `contrast-and-layout.html` | 50 | 4–12 (browser) | 0 | 2.1 ms |
| **Median** | - | - | - | **3.5 ms** |

`contrast-and-layout.html` requires a live browser (`puppeteer-core` + Chrome/Chromium) to evaluate computed layout and contrast bounds. Any other accessibility engine can be plugged into the same harness using the runner contract in [`benchmarks/README.md`](benchmarks/README.md).

## Testing & Verification

Run the unit, integration, and ACT-inspired conformance test suites in `tests/`:

```sh
cargo test
```

Generate local API documentation with `cargo doc --no-deps --open`.

## Documentation

- [API Documentation (docs.rs)](https://docs.rs/clearway) — Rust crate reference
- [CLI Reference & CI Integration](docs/cli.md) — flags, exit codes, JSON/SARIF output, and GitHub Code Scanning
- [Live Browsers & WebAssembly](docs/browser-and-wasm.md) — Playwright/Puppeteer/CDP snapshots and `clearway-wasm`
- [Rust Examples](examples/) — `basic.rs`, `browser_ir.rs`, and `batch_scan.rs`
- [Benchmarks](benchmarks/README.md) — fixtures, runner protocol, and reproduction steps

## License

Dual-licensed under MIT (`LICENSE-MIT`) or Apache-2.0 (`LICENSE-APACHE`), at your option.
