// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { julday } from '../index.js';
const { assess } = createRequire(import.meta.url)('../scripts/assess-mvp1-radial-speeds.cjs');

test('radial gate enforces coverage, threshold and Sun exclusion', () => {
  const hash = 'a'.repeat(64), rows = [];
  for (const year of [-11000, -8000, -3000, -1000, 400, 1600, 2600, 3800]) for (const month of [1, 4, 7, 10]) for (let body = 0; body < 10; body++) rows.push({ year, month, body, jdTdb: julday(year, month, 15, 12, true), correctionApplied: body !== 0, baselineResidualAuDay: 0, candidateResidualAuDay: 0 });
  const report = { schema: 'experimental-radial-candidate-holdout/1', status: 'EXPERIMENTAL_NOT_ACCEPTED', grid: 'confirmation', kernelSha256: hash, rows };
  assert.equal(assess(report, hash, 5e-7).failures, 0);
  rows[1].candidateResidualAuDay = -5.1e-7;
  assert.equal(assess(report, hash, 5e-7).failures, 1);
  rows[1].candidateResidualAuDay = 5e-7;
  assert.equal(assess(report, hash, 5e-7).failures, 0);
  rows[0].correctionApplied = true;
  assert.throws(() => assess(report, hash, 5e-7));
  rows[0].correctionApplied = false;
  rows[1].body = 0;
  assert.throws(() => assess(report, hash, 5e-7));
});
