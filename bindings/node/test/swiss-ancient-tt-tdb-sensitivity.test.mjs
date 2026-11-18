// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { summarize } = require('../scripts/audit-swiss-ancient-tt-tdb-sensitivity.cjs');

test('summarizes timing and direction maxima by epoch and target', () => {
  const base = { month: 4, offsetDifferenceMs: -0.1, roundedDifferenceMs: -0.12, roundTripDifferenceMs: 0, icrfImpactArcsec: 0.02, eclipticImpactArcsec: 0.03, simpleResidualArcsec: 0.04, erfaResidualArcsec: 0.05 };
  const rows = [
    { ...base, year: -12000, id: 1, icrfImpactArcsec: 0.2 },
    { ...base, year: -12000, id: 2, offsetDifferenceMs: -0.4 },
    { ...base, year: 2000, id: 1, erfaResidualArcsec: 0.5 },
    { ...base, year: 2000, id: 2, eclipticImpactArcsec: 0.4 },
  ];
  const result = summarize(rows);
  assert.equal(result.all.cases, 4);
  assert.equal(result.preHorizons.cases, 2);
  assert.equal(result.all.offsetDifferenceMs.value, -0.4);
  assert.equal(result.all.eclipticImpactArcsec.id, 2);
  assert.equal(result.preHorizons.moonIcrfImpactArcsec.value, 0.2);
  assert.equal(result.all.moonErfaResidualArcsec.value, 0.5);
  assert.throws(() => summarize([]), /invalid rows/);
  assert.throws(() => summarize([{ ...rows[0], icrfImpactArcsec: NaN }]), /invalid rows/);
  assert.throws(() => summarize(rows.slice(2)), /empty group/);
});
