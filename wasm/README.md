# clearway-wasm

Universal WebAssembly bindings for [`clearway`](https://github.com/TheGreatAxios/clearway) using `wasm-bindgen`.

Works in browsers, Chrome extensions, Cloudflare Workers, Node, Bun, and Deno.

## Build

```sh
# Bundlers (Vite, webpack, esbuild, Cloudflare Workers)
wasm-pack build --target bundler

# Node / Bun
wasm-pack build --target nodejs

# Direct browser ES modules
wasm-pack build --target web
```

## Usage

```js
import { audit_html, audit_document, builtin_rules } from "./pkg/clearway_wasm.js";

const report = JSON.parse(audit_html('<html lang="en"><head><title>Hi</title></head><body><main><h1>Hi</h1></main></body></html>'));
console.log(report.summary);

const rules = JSON.parse(builtin_rules());
console.log(`${rules.length} rules`);
```

## License

Dual-licensed under MIT (../LICENSE-MIT) or Apache-2.0 (../LICENSE-APACHE), at your option.
