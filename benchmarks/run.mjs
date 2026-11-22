#!/usr/bin/env node
/**
 * @fileoverview Pluggable benchmark harness for `clearway`.
 *
 * Serves `benchmarks/fixtures/` over loopback HTTP, reads ground-truth cases
 * from `benchmarks/cases.json`, and runs one or more `--runner <path.mjs>`
 * plugins (defaulting to `./runners/clearway.mjs`).
 *
 * Licensed under the MIT OR Apache-2.0 license.
 */

import fs from 'node:fs';
import http from 'node:http';
import {createRequire} from 'node:module';
import path from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, '..');
const FIXTURES_DIR = path.join(SCRIPT_DIR, 'fixtures');
const CASES_PATH = path.join(SCRIPT_DIR, 'cases.json');
const DEFAULT_RUNNER = path.join(SCRIPT_DIR, 'runners', 'clearway.mjs');

const WARMUP_RUNS = 1;
const TIMED_RUNS = 3;

const CHROME_CANDIDATES = [
  process.env.CHROME_PATH,
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/Applications/Chromium.app/Contents/MacOS/Chromium',
  '/usr/bin/google-chrome',
  '/usr/bin/google-chrome-stable',
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
].filter(Boolean);

const nodeRequire = createRequire(import.meta.url);

/**
 * Parses CLI flags and returns resolved runner paths.
 * @param {!Array<string>} argv
 * @return {!Array<string>}
 */
function parseRunnerPaths(argv) {
  const paths = [];
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--runner' && i + 1 < argv.length) {
      paths.push(resolveRunnerPath(argv[i + 1]));
      i += 1;
    } else if (arg.startsWith('--runner=')) {
      paths.push(resolveRunnerPath(arg.slice('--runner='.length)));
    }
  }
  return paths.length > 0 ? paths : [DEFAULT_RUNNER];
}

/**
 * Resolves a runner path against CWD first, then `benchmarks/`.
 * @param {string} rawPath
 * @return {string}
 */
function resolveRunnerPath(rawPath) {
  const fromCwd = path.resolve(process.cwd(), rawPath);
  if (fs.existsSync(fromCwd)) {
    return fromCwd;
  }
  return path.resolve(SCRIPT_DIR, rawPath);
}

/**
 * Returns candidate `node_modules` directories from `NODE_PATH` and repo root.
 * @return {!Array<string>}
 */
function getSearchDirs() {
  const dirs = [];
  if (process.env.NODE_PATH) {
    for (const entry of process.env.NODE_PATH.split(path.delimiter)) {
      if (entry) {
        dirs.push(path.resolve(entry));
      }
    }
  }
  dirs.push(path.join(REPO_ROOT, 'node_modules'));
  dirs.push(path.join(SCRIPT_DIR, 'node_modules'));
  return dirs;
}

/**
 * Extracts the best ESM or main entry point from a parsed `package.json`.
 * @param {!Object} pkgJson
 * @return {?string}
 */
function resolvePackageEntry(pkgJson) {
  const exportsField = pkgJson.exports;
  if (typeof exportsField === 'string') {
    return exportsField;
  }
  if (exportsField && typeof exportsField === 'object') {
    const dot = exportsField['.'] || exportsField;
    if (typeof dot === 'string') {
      return dot;
    }
    if (dot && typeof dot === 'object') {
      return dot.import || dot.default || dot.require || null;
    }
  }
  return pkgJson.module || pkgJson.main || null;
}

/**
 * Resolves an optional npm package entry file.
 * @param {string} pkgName
 * @return {?string}
 */
function resolveOptionalPackage(pkgName) {
  try {
    return nodeRequire.resolve(pkgName);
  } catch {
    // Fall back to explicit NODE_PATH lookup (handles ESM-only packages).
  }
  for (const dir of getSearchDirs()) {
    const pkgDir = path.join(dir, pkgName);
    const pkgJsonPath = path.join(pkgDir, 'package.json');
    if (!fs.existsSync(pkgJsonPath)) {
      continue;
    }
    const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));
    const entry = resolvePackageEntry(pkgJson);
    if (entry) {
      const resolved = path.join(pkgDir, entry);
      if (fs.existsSync(resolved)) {
        return resolved;
      }
    }
  }
  return null;
}

/**
 * Dynamically imports `puppeteer-core` if installed.
 * @return {!Promise<?Object>}
 */
async function loadPuppeteer() {
  const entry = resolveOptionalPackage('puppeteer-core');
  if (!entry) {
    return null;
  }
  try {
    const mod = await import(pathToFileURL(entry).href);
    return mod.default || mod;
  } catch {
    return null;
  }
}

/**
 * Locates a local Chrome or Chromium executable.
 * @return {?string}
 */
function findChromeExecutable() {
  for (const candidate of CHROME_CANDIDATES) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  return null;
}

/**
 * Dynamically imports and validates a runner plugin module.
 * @param {string} runnerPath
 * @return {!Promise<{name: string, run: !Function}>}
 */
async function loadRunner(runnerPath) {
  const mod = await import(pathToFileURL(runnerPath).href);
  const plugin = mod.default || mod;
  if (!plugin || typeof plugin.name !== 'string' ||
      typeof plugin.run !== 'function') {
    throw new Error(
        `Invalid runner at ${runnerPath}: expected default export ` +
        `{name, async run({page, html, url})}`,
    );
  }
  return plugin;
}

/**
 * Starts a loopback static HTTP server for `benchmarks/fixtures/`.
 * @param {!Set<string>} allowedFiles
 * @return {!Promise<{server: !http.Server, baseUrl: string}>}
 */
function startFixtureServer(allowedFiles) {
  const fixturesRoot = fs.realpathSync(FIXTURES_DIR);
  const fixturesPrefix = fixturesRoot.endsWith(path.sep) ?
    fixturesRoot :
    fixturesRoot + path.sep;
  return new Promise((resolve, reject) => {
    const server = http.createServer((req, res) => {
      let decodedPath;
      try {
        decodedPath = decodeURIComponent((req.url || '/').split('?')[0]);
      } catch {
        res.writeHead(400, {'content-type': 'text/plain; charset=utf-8'});
        res.end('Bad request');
        return;
      }
      const relPath = decodedPath.replace(/^\/+/, '');
      if (!relPath || relPath.includes('/') || relPath.includes('\\') ||
          !allowedFiles.has(relPath)) {
        res.writeHead(404, {'content-type': 'text/plain; charset=utf-8'});
        res.end('Not found');
        return;
      }
      const candidatePath = path.resolve(fixturesRoot, relPath);
      if (!candidatePath.startsWith(fixturesPrefix) ||
          !fs.existsSync(candidatePath)) {
        res.writeHead(404, {'content-type': 'text/plain; charset=utf-8'});
        res.end('Not found');
        return;
      }
      const realPath = fs.realpathSync(candidatePath);
      if (!realPath.startsWith(fixturesPrefix)) {
        res.writeHead(404, {'content-type': 'text/plain; charset=utf-8'});
        res.end('Not found');
        return;
      }
      res.writeHead(200, {'content-type': 'text/html; charset=utf-8'});
      res.end(fs.readFileSync(realPath));
    });
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      const port = typeof addr === 'object' && addr ? addr.port : 0;
      resolve({server, baseUrl: `http://127.0.0.1:${port}`});
    });
  });
}

/**
 * Computes the median of a non-empty numeric array.
 * @param {!Array<number>} values
 * @return {number}
 */
function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) {
    return sorted[mid];
  }
  return (sorted[mid - 1] + sorted[mid]) / 2;
}

/**
 * Formats a millisecond duration to 1 decimal place.
 * @param {number} ms
 * @return {string}
 */
function fmtMs(ms) {
  return `${ms.toFixed(1)} ms`;
}

/**
 * Executes a single runner iteration with an optional fresh browser page.
 * @param {{name: string, run: !Function}} runner
 * @param {?Object} browser
 * @param {string} url
 * @param {string} html
 * @return {!Promise<!Object>}
 */
async function executeOnce(runner, browser, url, html) {
  if (!browser) {
    return runner.run({page: null, html, url});
  }
  const page = await browser.newPage();
  try {
    await page.setViewport({width: 1280, height: 800});
    await page.goto(url, {waitUntil: 'load'});
    return await runner.run({page, html, url});
  } finally {
    await page.close();
  }
}

/**
 * Benchmarks one runner on one fixture (1 warmup + 3 timed runs).
 * @param {{name: string, run: !Function}} runner
 * @param {?Object} browser
 * @param {string} url
 * @param {string} html
 * @return {!Promise<{ms: number, engineMs: ?number, violations: number,
 *   incomplete: number, elements: ?number}>}
 */
async function benchmarkCase(runner, browser, url, html) {
  for (let i = 0; i < WARMUP_RUNS; i += 1) {
    await executeOnce(runner, browser, url, html);
  }

  const samples = [];
  for (let i = 0; i < TIMED_RUNS; i += 1) {
    samples.push(await executeOnce(runner, browser, url, html));
  }

  const msValues = samples.map((s) => s.ms);
  const engineValues = samples
      .map((s) => s.engineMs)
      .filter((v) => typeof v === 'number');
  const last = samples[samples.length - 1];

  return {
    ms: median(msValues),
    engineMs: engineValues.length > 0 ? median(engineValues) : null,
    violations: last.violations,
    incomplete: typeof last.incomplete === 'number' ? last.incomplete : 0,
    elements: typeof last.elements === 'number' ? last.elements : null,
  };
}

/**
 * Formats one table cell for a runner result.
 * @param {!Object} result
 * @param {boolean} showEngineMs
 * @return {string}
 */
function formatCell(result, showEngineMs) {
  const timing = showEngineMs && result.engineMs !== null ?
    `${fmtMs(result.ms)} (${fmtMs(result.engineMs)} engine)` :
    fmtMs(result.ms);
  const counts = result.incomplete > 0 ?
    `${result.violations}v / ${result.incomplete}i` :
    `${result.violations}v`;
  return `${timing} / ${counts}`;
}

/**
 * Entry point.
 * @return {!Promise<void>}
 */
async function main() {
  const runnerPaths = parseRunnerPaths(process.argv.slice(2));
  const cases = JSON.parse(fs.readFileSync(CASES_PATH, 'utf8'));
  const allowedFiles = new Set(cases.map((c) => c.file));

  const runners = [];
  for (const runnerPath of runnerPaths) {
    runners.push(await loadRunner(runnerPath));
  }

  const puppeteer = await loadPuppeteer();
  const chromePath = findChromeExecutable();
  const useBrowser = Boolean(puppeteer && chromePath);

  console.log(
      `mode: ${useBrowser ? 'Headless Chrome' : 'Static HTML5'} ` +
      `(${WARMUP_RUNS} warmup + ${TIMED_RUNS} timed runs, median reported)`,
  );
  console.log(`runners: ${runners.map((r) => r.name).join(', ')}`);
  console.log('');

  const {server, baseUrl} = await startFixtureServer(allowedFiles);
  let browser = null;

  try {
    if (useBrowser) {
      browser = await puppeteer.launch({
        executablePath: chromePath,
        headless: true,
        args: ['--no-sandbox', '--disable-setuid-sandbox'],
      });
    }

    const rows = [];
    for (const testCase of cases) {
      const filePath = path.join(FIXTURES_DIR, testCase.file);
      const html = fs.readFileSync(filePath, 'utf8');
      const url = `${baseUrl}/${testCase.file}`;
      const results = {};
      let elements = null;

      for (const runner of runners) {
        const res = await benchmarkCase(runner, browser, url, html);
        results[runner.name] = res;
        if (elements === null && res.elements !== null) {
          elements = res.elements;
        }
      }

      rows.push({testCase, elements, results});
    }

    const runnerHasDistinctEngine = {};
    for (const runner of runners) {
      runnerHasDistinctEngine[runner.name] = useBrowser && rows.some((row) => {
        const res = row.results[runner.name];
        return res.engineMs !== null &&
          Math.abs(res.ms - res.engineMs) > 0.05;
      });
    }

    const headers = [
      'Fixture',
      'Elements',
      ...runners.map((r) => (
        runnerHasDistinctEngine[r.name] ?
          `${r.name} (e2e / engine / v)` :
          `${r.name} (ms / v)`
      )),
    ];
    const sep = headers.map((_, idx) => (idx === 0 ? '---' : '---:'));

    console.log(`| ${headers.join(' | ')} |`);
    console.log(`| ${sep.join(' | ')} |`);

    for (const row of rows) {
      const cells = [
        row.testCase.file,
        row.elements !== null ? String(row.elements) : '-',
        ...runners.map((r) =>
          formatCell(row.results[r.name], runnerHasDistinctEngine[r.name]),
        ),
      ];
      console.log(`| ${cells.join(' | ')} |`);
    }

    const medianCells = [
      '**Median**',
      '-',
      ...runners.map((r) => {
        const medMs = median(rows.map((row) => row.results[r.name].ms));
        if (runnerHasDistinctEngine[r.name]) {
          const medEng = median(
              rows.map((row) => row.results[r.name].engineMs),
          );
          return `**${fmtMs(medMs)}** (${fmtMs(medEng)} engine)`;
        }
        return `**${fmtMs(medMs)}**`;
      }),
    ];
    console.log(`| ${medianCells.join(' | ')} |`);

    const boundFailures = [];
    let checkedBounds = 0;
    for (const row of rows) {
      const {
        file,
        expectedViolationsMin,
        expectedViolationsMax,
        requiresBrowser,
      } = row.testCase;
      if (!useBrowser && requiresBrowser) {
        continue;
      }
      checkedBounds += 1;
      const primary = row.results.clearway || row.results[runners[0].name];
      if (primary.violations < expectedViolationsMin ||
          primary.violations > expectedViolationsMax) {
        boundFailures.push(
            `${file}: got ${primary.violations} violations ` +
            `(expected ${expectedViolationsMin}..${expectedViolationsMax})`,
        );
      }
    }

    console.log('');
    if (boundFailures.length > 0) {
      console.error('Ground-truth bound check FAILED:');
      for (const msg of boundFailures) {
        console.error(`  - ${msg}`);
      }
      process.exitCode = 1;
    } else {
      console.log(
          `Ground-truth bounds: PASS (${checkedBounds}/${rows.length} fixtures)`,
      );
    }
  } finally {
    if (browser) {
      await browser.close();
    }
    await new Promise((resolve) => server.close(resolve));
  }
}

await main();
