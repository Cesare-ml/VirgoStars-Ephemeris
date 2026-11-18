// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { nextUp, nextDown, summarize } = require('../scripts/audit-de441-epoch-quantization.cjs');

test('adjacent binary64 values preserve ordering for positive and negative epochs', () => {
  for (const value of [-3026727, -1, -Number.MIN_VALUE, -0, 0, Number.MIN_VALUE, 1, 2451545]) {
    assert.ok(nextUp(value) > value);
    assert.ok(nextDown(value) < value);
    assert.ok(nextDown(nextUp(value)) === value);
  }
  assert.equal(nextUp(Infinity), Infinity);
  assert.equal(nextDown(-Infinity), -Infinity);
  assert.ok(Number.isNaN(nextUp(NaN)));
});

test('summarizes signed time error and adjacent-epoch sensitivity', () => {
  const row = { month: 1, errorMicroseconds: -12, stepMicroseconds: 40, icrfStepArcsec: 0.00002, eclipticStepArcsec: 0.000021 };
  const rows = [{ ...row, year: -12000, id: 1 }, { ...row, year: -12000, id: 2, errorMicroseconds: -15 }, { ...row, year: 2000, id: 1, icrfStepArcsec: 0.00003 }];
  const result = summarize(rows);
  assert.equal(result.preHorizons.cases, 2);
  assert.equal(result.all.maxRoundingErrorMicroseconds.value, -15);
  assert.equal(result.all.maxMoonIcrfStepArcsec.value, 0.00003);
  assert.throws(() => summarize([]), /invalid rows/);
  assert.throws(() => summarize([{ ...row, year: -12000, id: 1, stepMicroseconds: 0 }]), /invalid rows/);
  assert.throws(() => summarize(rows.slice(2)), /empty group/);
});
