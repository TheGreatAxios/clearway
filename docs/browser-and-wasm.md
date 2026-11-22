# Live Browsers & WebAssembly

Static HTML analysis catches structural, semantic, and ARIA issues without launching a browser. For layout, computed color contrast, Shadow DOM, and interactive state checks, collect a live DOM snapshot with [`collectors/browser-snapshot.js`](../collectors/browser-snapshot.js) or run Clearway directly in JavaScript via WebAssembly.

## Live Browser Snapshots (Playwright, Puppeteer, CDP)

`collectors/browser-snapshot.js` has zero runtime dependencies and attaches `globalThis.collectClearwayDocument(document)` in any browser context (Chromium, Firefox, WebKit, or CDP `Runtime.evaluate`). It serializes the live DOM into the JSON Document IR consumed by `clearway --ir` or `clearway::audit_document`.

### Playwright example

```js
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import { chromium } from "playwright";

const collector = fs.readFileSync("collectors/browser-snapshot.js", "utf8");
const browser = await chromium.launch();
const page = await browser.newPage();
await page.goto("http://localhost:3000", { waitUntil: "networkidle" });
await page.addScriptTag({ content: collector });

const snapshot = await page.evaluate(() => globalThis.collectClearwayDocument(document));
const out = execFileSync("clearway", ["-", "--ir", "--json"], {
  input: JSON.stringify(snapshot),
  encoding: "utf8",
});
console.log(JSON.parse(out).summary);
await browser.close();
```

Puppeteer works identically (`page.addScriptTag({ content: collector })` + `page.evaluate(...)`). Over raw CDP, evaluate the collector source via `Runtime.evaluate`, then call `collectClearwayDocument(document)` with `returnByValue: true`.

Note: `clearway` exits with code `1` when violations are found, while still printing the full JSON report to stdout. Catch `err.stdout` when invoking via `execFileSync` if you expect non-zero violations.

### What live browser snapshots unlock

Compared to static HTML, a browser IR snapshot includes computed styles and geometry so Clearway can verify:

- **Open Shadow DOM & `<slot>` composition**: traverses nested Web Components and slotted content.
- **Same-origin `<iframe>` trees**: inspects embedded documents inline.
- **`z-index` visual contrast**: resolves effective foreground and background colors across stacked layers and semi-transparent backdrops.
- **CSS gradients and backgrounds**: flags gradients where every stop fails contrast, and routes non-provable image/gradient backdrops to `incomplete` instead of false positives.
- **WCAG 2.2 target size**: checks computed bounding rects (`24×24px` minimum and spacing exemptions).
- **Focus and scrollability**: detects suppressed focus outlines (`outline: none` without a replacement indicator) and non-focusable scrollable regions (`overflow: scroll|auto`).

## WebAssembly (`clearway-wasm`)

The [`wasm/`](../wasm/) crate compiles `clearway` to WebAssembly with `wasm-bindgen` for browsers, extensions, Cloudflare Workers, Node, Bun, and Deno.

### Build

```sh
cd wasm

# Bundlers (Vite, webpack, esbuild, Cloudflare Workers)
wasm-pack build --target bundler

# Node / Bun
wasm-pack build --target nodejs

# Direct browser ES modules
wasm-pack build --target web
```

### Usage

```js
import { audit_html, audit_document, builtin_rules } from "./wasm/pkg/clearway_wasm.js";

// Static HTML5
const report = JSON.parse(
  audit_html('<html lang="en"><head><title>Hi</title></head><body><main><h1>Hi</h1></main></body></html>')
);
console.log(report.summary);

// Live browser snapshot (in-page or extension)
const docJson = JSON.stringify(globalThis.collectClearwayDocument(document));
const liveReport = JSON.parse(audit_document(docJson));
console.log(liveReport.violations);

// List all 113 built-in rules
const rules = JSON.parse(builtin_rules());
```

Both `audit_html` and `audit_document` accept an optional second `options_json` string matching `clearway::AuditOptions` (for example, `JSON.stringify({ tags: ["wcag2a", "wcag2aa"] })`).
