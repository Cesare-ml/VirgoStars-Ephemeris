// SPDX-License-Identifier: CPAL-1.0.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { separation } = require('../scripts/audit-ancient-rate-outlier.cjs');

test('apparent angular separation handles scaled, opposite, and orthogonal vectors', () => {
  assert.equal(separation([1, 0, 0], [2, 0, 0]), 0);
  assert.equal(separation([1, 0, 0], [-2, 0, 0]), 180);
  assert.equal(separation([1, 0, 0], [0, 4, 0]), 90);
  assert.throws(() => separation([0, 0, 0], [1, 0, 0]));
  assert.throws(() => separation([NaN, 0, 0], [1, 0, 0]));
});
