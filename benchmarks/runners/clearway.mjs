/**
 * @fileoverview Built-in benchmark runner plugin for `clearway`.
 *
 * Supports both live Headless Chrome DOM IR snapshots (via
 * `collectors/browser-snapshot.js`) and static HTML5 mode when `page` is null.
 *
 * Licensed under the MIT OR Apache-2.0 license.
 */

import {execFileSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import {performance} from 'node:perf_hooks';
import {fileURLToPath} from 'node:url';

const RUNNER_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(RUNNER_DIR, '..', '..');
const COLLECTOR_PATH = path.join(REPO_ROOT, 'collectors', 'browser-snapshot.js');

const COLLECTOR_SOURCE = fs.readFileSync(COLLECTOR_PATH, 'utf8');

/**
 * Locates the compiled `clearway` binary.
 * @return {string}
 */
function findClearwayBinary() {
  const releaseBin = path.join(REPO_ROOT, 'target', 'release', 'clearway');
  if (fs.existsSync(releaseBin)) {
    return releaseBin;
  }
  const debugBin = path.join(REPO_ROOT, 'target', 'debug', 'clearway');
  if (fs.existsSync(debugBin)) {
    return debugBin;
  }
  return 'clearway';
}

const CLEARWAY_BIN = findClearwayBinary();

/**
 * Runs the `clearway` CLI and parses its JSON output.
 * Exit code 1 (violations found) still emits a valid JSON report.
 * @param {!Array<string>} args
 * @param {string} stdinText
 * @return {!Object}
 */
function runClearwayJson(args, stdinText) {
  const options = {
    encoding: 'utf8',
    input: stdinText,
    maxBuffer: 64 * 1024 * 1024,
  };
  try {
    const stdout = execFileSync(CLEARWAY_BIN, args, options);
    return JSON.parse(stdout);
  } catch (err) {
    if (err && typeof err.stdout === 'string' && err.stdout.trim()) {
      return JSON.parse(err.stdout);
    }
    throw err;
  }
}

export default {
  name: 'clearway',

  /**
   * Audits a fixture via live Headless Chrome IR or static HTML5 stdin.
   * @param {{page: ?Object, html: string, url: string}} ctx
   * @return {!Promise<{ms: number, engineMs: number, violations: number,
   *   incomplete: number, elements: number}>}
   */
  async run({page, html}) {
    if (!page) {
      const t0 = performance.now();
      const report = runClearwayJson(['-', '--json'], html);
      const elapsed = performance.now() - t0;
      return {
        ms: elapsed,
        engineMs: elapsed,
        violations: report.summary.violations,
        incomplete: report.summary.incomplete || 0,
        elements: report.summary.nodes_checked,
      };
    }

    await page.addScriptTag({content: COLLECTOR_SOURCE});

    const t0 = performance.now();
    const snapshot = await page.evaluate(() => {
      return globalThis.collectClearwayDocument(document);
    });
    const tCollect = performance.now();
    const payload = JSON.stringify(snapshot);
    const report = runClearwayJson(['-', '--ir', '--json'], payload);
    const tEnd = performance.now();

    return {
      ms: tEnd - t0,
      engineMs: tEnd - tCollect,
      violations: report.summary.violations,
      incomplete: report.summary.incomplete || 0,
      elements: report.summary.nodes_checked,
    };
  },
};
