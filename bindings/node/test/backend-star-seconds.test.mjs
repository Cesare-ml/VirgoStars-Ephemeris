// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Opt-in local diagnostic; supplied catalogs and kernels remain outside Git.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, rmdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const { STAR_CANDIDATE_REPORT, STAR_QUALITY_REPORT, HIP2_PATH, DE441_PATH, SWISSEPH_PATH,
  BACKEND_ASTROLOGY_FUNC } = process.env;
const configured = STAR_CANDIDATE_REPORT && STAR_QUALITY_REPORT && HIP2_PATH && DE441_PATH
  && SWISSEPH_PATH && BACKEND_ASTROLOGY_FUNC;

test('stellar audit detects discarded seconds and accepts the corrected backend',
  { skip: configured ? false : 'local candidate reports, kernels, reference and backend required' }, () => {
    const script = fileURLToPath(new URL('../scripts/audit-star-backend.mjs', import.meta.url));
    const prefix = [script, STAR_CANDIDATE_REPORT, STAR_QUALITY_REPORT, HIP2_PATH, DE441_PATH, SWISSEPH_PATH];
    const run = (backend, env = process.env) => spawnSync(process.execPath, [...prefix, backend, '--seconds'],
      { encoding: 'utf8', env, maxBuffer: 1024 * 1024 });
    const corrected = run(BACKEND_ASTROLOGY_FUNC);
    assert.equal(corrected.status, 0, corrected.stderr);
    const report = JSON.parse(corrected.stdout);
    assert.equal(report.comparisons, report.selectedCandidates * report.requestedEpochs);
    assert.equal(report.backendComparisons, report.comparisons * 3);
    assert.equal(report.backendEpochCheck.mismatches, 0);

    // A fictional adapter deliberately reintroduces minute truncation. This is
    // a negative control, not a copy of a third-party or historical backend.
    const directory = mkdtempSync(join(tmpdir(), 'virgostars-star-seconds-'));
    const fixture = join(directory, 'truncate.cjs');
    try {
      writeFileSync(fixture, `module.exports.createAstrologyFunctions = facade => {
        const real = require(process.env.TEST_BACKEND_MODULE).createAstrologyFunctions(facade);
        return { getStella(date, ...args) { return real.getStella(date.slice(0, -2) + '00', ...args); } };
      };\n`);
      const truncated = run(fixture, { ...process.env, TEST_BACKEND_MODULE: BACKEND_ASTROLOGY_FUNC });
      assert.equal(truncated.status, 1, truncated.stderr);
      const bad = JSON.parse(truncated.stdout);
      assert.ok(bad.backendEpochCheck.mismatches > 0);
      assert.equal(bad.backendEpochCheck.mismatches, bad.backendComparisons);
    } finally {
      rmSync(fixture, { force: true });
      rmdirSync(directory);
    }
  });
