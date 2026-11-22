# clearway benchmarks

Pluggable benchmark harness for `clearway`. The repository ships only `clearway`'s own runner (`benchmarks/runners/clearway.mjs`) and 5 self-contained HTML5 fixtures listed in [`cases.json`](cases.json):

- `fixtures/small-landing.html`: clean marketing page with landmarks, headings, links, and forms (0 expected violations).
- `fixtures/medium-violations.html`: page with 15+ WCAG 2.1/2.2 A/AA and ARIA violations.
- `fixtures/large-table.html`: structured multi-section data table (~2,850 elements) for O(N) scaling checks.
- `fixtures/modern-shadow-dom.html`: nested open Shadow DOM Web Components with `<slot>`, native `<dialog>`, invalid `popovertarget`, paste-blocking password input, `:focus { outline: none }`, negative `z-index` dark backdrop, and CSS gradients.
- `fixtures/contrast-and-layout.html`: WCAG AA/AAA solid contrast, an all-stops-failing CSS gradient, a `url()` image backdrop, a `z-index: -1` dark backdrop, a non-focusable `overflow: scroll` region, and cramped vs. spaced 24px targets.

## Running clearway

Build the release binary and run `benchmarks/run.mjs`:

```sh
cargo build --release
node benchmarks/run.mjs
```

If `puppeteer-core` is available on `NODE_PATH` (or in `node_modules`) alongside a local Chrome or Chromium binary, the harness serves `benchmarks/fixtures/` over `127.0.0.1` and runs every fixture in Headless Chrome (1 warmup run + 3 timed runs, reporting the median). Otherwise, it runs in static HTML5 mode (`page: null`). Set `CHROME_PATH` if Chrome or Chromium is in a non-standard location.

## Plugging in external runners

Pass one or more `--runner <path.mjs>` flags to compare any accessibility engine side by side:

```sh
node benchmarks/run.mjs \
  --runner ./runners/clearway.mjs \
  --runner /path/to/other-engine.mjs
```

Every runner plugin is an ES module exporting a single object with `name` and an async `run` method:

```js
export default {
  name: 'my-engine',

  async run({page, html, url}) {
    // `page` is a Puppeteer Page already navigated to `url` (or `null` in static mode).
    // `html` is the raw fixture markup string.
    const t0 = performance.now();
    // ...run your engine...
    return {
      ms: performance.now() - t0,
      violations: 0,
      incomplete: 0, // optional
      engineMs: undefined, // optional: split engine time from browser collection time
    };
  },
};
```
