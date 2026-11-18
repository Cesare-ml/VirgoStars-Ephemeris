// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { Engine, julday } from '../index.js';

const kernel = process.env.DE441_PATH;
test('the explicit radial candidate preserves the existing six-field derivative', { skip: kernel ? false : 'DE441_PATH required' }, () => {
  const engine = new Engine(kernel);
  for (const year of [-12999, 0, 2000, 3999]) {
    const jd = julday(year, 7, 1, 12, true);
    for (const body of [0, 1, 4, 9]) {
      const base = engine.experimentalLongTermDirectObliquityTdb(jd, body);
      const candidate = engine.experimentalRadialRateCandidateTdb(jd, body);
      assert.deepEqual(candidate.base.xx, base.xx);
      assert.equal(candidate.candidateAuPerTdbDay, base.xx[5] + candidate.correctionAuPerTdbDay);
      assert.ok(Number.isFinite(candidate.candidateAuPerTdbDay));
      assert.match(candidate.notes.join(' '), /not a position derivative/);
      assert.equal(candidate.correctionApplied, body !== 0);
      if (body === 0) assert.equal(candidate.correctionAuPerTdbDay, 0);
    }
  }
  assert.throws(() => engine.experimentalRadialRateCandidateTdb(NaN, 0), e => e.code === 'E_RANGE');
  assert.throws(() => engine.experimentalRadialRateCandidateTdb(2451545, 15), e => e.code === 'E_UNSUPPORTED_BODY');
});
