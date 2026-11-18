// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { summarize } = require('../scripts/audit-horizons-tt-tdb-sensitivity.cjs');

test('time and position sensitivity keeps signed residual changes', () => {
  const rows = [
    { id: 1, year: -9000, month: 2, offsetDifferenceMs: -0.3, roundedDifferenceMs: -0.28, timeImpactArcsec: 0.2, oldResidualArcsec: 0.3, erfaResidualArcsec: 0.1 },
    { id: 1, year: 2000, month: 8, offsetDifferenceMs: 0.01, roundedDifferenceMs: 0, timeImpactArcsec: 0.01, oldResidualArcsec: 0.01, erfaResidualArcsec: 0.02 },
    { id: 2, year: -9000, month: 2, offsetDifferenceMs: -0.3, roundedDifferenceMs: -0.28, timeImpactArcsec: 0.05, oldResidualArcsec: 0.02, erfaResidualArcsec: 0.06 },
  ];
  const summary = summarize(rows);
  assert.equal(summary.cases, 3);
  assert.equal(summary.maxOffsetDifferenceMs.value, -0.3);
  assert.ok(Math.abs(summary.moon.maxResidualChange.value + 0.2) < 1e-15);
  assert.equal(summary.other.maxImpact.value, 0.05);
  assert.throws(() => summarize([]), /invalid rows/);
  assert.throws(() => summarize([{ ...rows[0], timeImpactArcsec: NaN }]), /invalid rows/);
  assert.throws(() => summarize(rows.slice(0, 2)), /empty group/);
});
