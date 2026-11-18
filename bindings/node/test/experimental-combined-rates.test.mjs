// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { Engine, julday } from '../index.js';

const kernel = process.env.DE441_PATH;
test('the combined candidate preserves base derivatives and the individual candidates', { skip: kernel ? false : 'DE441_PATH required' }, () => {
  const engine = new Engine(kernel);
  for (const year of [-12999, -11000, 0, 2000, 3500, 3999]) {
    const jd = julday(year, 7, 1, 12, true);
    for (let body = 0; body < 10; body++) {
      const base = engine.experimentalLongTermDirectObliquityTdb(jd, body);
      const longitude = engine.experimentalLongitudeRateCandidateTdb(jd, body);
      const radial = engine.experimentalRadialRateCandidateTdb(jd, body);
      const combined = engine.experimentalCombinedRateCandidateTdb(jd, body);
      assert.deepEqual(combined.base.xx, base.xx);
      assert.deepEqual(combined.candidateXx.slice(0, 3), base.xx.slice(0, 3));
      assert.equal(combined.candidateXx[3], longitude.candidateDegPerTdbDay);
      assert.equal(combined.candidateXx[4], base.xx[4]);
      assert.equal(combined.candidateXx[5], radial.candidateAuPerTdbDay);
      assert.equal(combined.radialCorrectionApplied, body !== 0);
      assert.match(combined.notes.join(' '), /not position derivatives/);
    }
  }
  assert.throws(() => engine.experimentalCombinedRateCandidateTdb(NaN, 0), e => e.code === 'E_RANGE');
  assert.throws(() => engine.experimentalCombinedRateCandidateTdb(2451545, 15), e => e.code === 'E_UNSUPPORTED_BODY');
});
