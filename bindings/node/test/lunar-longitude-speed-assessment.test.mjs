// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { assess, expectedEpochs } = require('../scripts/assess-lunar-longitude-speed.cjs');

test('the lunar speed threshold rejects a violation and invalid coverage', () => {
  const hash = 'a'.repeat(64), epochs = expectedEpochs();
  const rows = epochs.map((jdTdb, i) => ({ jdTdb, category: i === 192 ? 'first' : i === 193 ? 'last' : 'grid', longitudeCandidateMinusSwiss: 0 }));
  const report = { schema: 'lunar-speed-resolution/1', status: 'OPEN_NO_ACCEPTED_RATE_TOLERANCE', inputSha256: [hash], rows };
  assert.equal(assess(report, hash, 0.3).failures, 0);
  rows[0].longitudeCandidateMinusSwiss = -0.31;
  assert.equal(assess(report, hash, 0.3).failures, 1);
  rows[0].longitudeCandidateMinusSwiss = 0.3;
  assert.equal(assess(report, hash, 0.3).failures, 0);
  assert.throws(() => assess(report, hash, NaN));
  rows[1].jdTdb = rows[0].jdTdb;
  assert.throws(() => assess(report, hash, 0.3));
});
