# CLI Reference & CI Integration

## Usage

```sh
clearway <html-or-ir-json|-> [OPTIONS]
```

Pass a local file path or `-` to read HTML5 (or Document IR JSON when `--ir` is set) from standard input.

## Flags

| Flag | Description |
| --- | --- |
| `--json` | Emit the full `AuditReport` as pretty-printed JSON. Also works with `--rules` to output rule metadata as JSON. |
| `--sarif` | Emit SARIF 2.1.0 JSON for GitHub Code Scanning and PR annotations. |
| `--ir` | Parse input as a serialized browser Document IR JSON snapshot instead of raw HTML5. |
| `--tags <list>` | Comma-separated list of tags to run (e.g. `wcag2a,wcag2aa`, `wcag21aa`, `wcag22aa`, `ACT`, `best-practice`, `clearway`). |
| `--profile <standard\|act>` | Conformance profile: `standard` (default) flags `placeholder`-only form inputs under `label`; `act` (also accepts `w3c-act`) accepts `placeholder` as a fallback accessible name on `label` per W3C AccName. |
| `--all-rules` | Include rules disabled by default (`wcag2aaa`, experimental, obsolete, and opt-in extended rules). |
| `--rules` | List all 113 built-in rules with WCAG references, impact levels, default states, and tags, then exit. |
| `--bench <n>` | Run `<n>` audit iterations (after 1 warmup run) and print `us/audit` and `audits/sec`. |
| `-h`, `--help` | Print usage information and exit. |

## Examples

```sh
# Audit a static HTML file
clearway dist/index.html

# Pipe HTML from curl or a template renderer
curl -s https://example.com | clearway - --json

# Audit a live browser Document IR snapshot
clearway snapshot.json --ir --json

# Run only WCAG 2.0/2.1 Level A and AA checks
clearway dist/index.html --tags wcag2a,wcag2aa,wcag21a,wcag21aa

# Accept placeholder as a fallback accessible name on `label` (W3C AccName)
clearway dist/index.html --profile act

# Include opt-in and AAA rules
clearway dist/index.html --all-rules

# Inspect available rules as JSON
clearway --rules --json

# Microbenchmark HTML5 parse + full audit loop
clearway dist/index.html --bench 1000
```

## Exit Codes

| Code | Meaning |
| --- | --- |
| `0` | Audit completed with zero `violations` (warnings and `incomplete` items do not fail the run), or `--rules` / `--bench` / `--help` succeeded. |
| `1` | One or more `violations` were found. Full text, `--json`, or `--sarif` output is still written to stdout. |
| `2` | Invalid CLI arguments, unreadable input file, invalid IR JSON, or serialization error. |

## GitHub Code Scanning (SARIF)

Generate a SARIF 2.1.0 report and upload it to GitHub Code Scanning so violations appear directly on pull requests:

```yaml
name: Accessibility
on: [pull_request, push]

jobs:
  clearway:
    runs-on: ubuntu-latest
    permissions:
      security-events: write
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install clearway
      - name: Run Clearway audit
        run: clearway dist/index.html --sarif > clearway.sarif
        continue-on-error: true
      - name: Upload SARIF report
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: clearway.sarif
```

## Coding-Agent Loops

For automated fix-and-verify loops, run `clearway` with `--json`. Every entry in `violations` and `incomplete` includes a deterministic `rule_id`, `impact`, CSS `selector`, `description`, and `html_snippet`. Because exit code `1` still prints valid JSON on stdout, agents can parse stdout directly and iterate until `report.summary.violations === 0`.
