// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { Engine, julday } from '../index.js';

const kernel = process.env.DE441_PATH;
test('the three-rate candidate retains its base and all separate candidates', { skip: kernel ? false : 'DE441_PATH required' }, () => {
  const engine = new Engine(kernel);
  for (const year of [-12999, -11000, 0, 2000, 3500, 3999]) {
    const jd = julday(year, 7, 1, 12, true);
    for (let body = 0; body < 10; body++) {
      const two = engine.experimentalCombinedRateCandidateTdb(jd, body);
      const lat = engine.experimentalLatitudeRateCandidateTdb(jd, body);
      const three = engine.experimentalThreeRateCandidateTdb(jd, body);
      assert.deepEqual(three.base.xx, two.base.xx);
      assert.deepEqual(three.base.xx, lat.base.xx);
      for (const i of [0, 1, 2, 3, 5]) assert.equal(three.candidateXx[i], two.candidateXx[i]);
      assert.equal(three.candidateXx[4], lat.candidateDegPerTdbDay);
      assert.equal(lat.candidateDegPerTdbDay, lat.base.xx[4] + lat.correctionDegPerTdbDay);
      assert.equal(three.radialCorrectionApplied, body !== 0);
      assert.match(three.notes.join(' '), /not position derivatives/);
    }
  }
  assert.throws(() => engine.experimentalThreeRateCandidateTdb(NaN, 0), e => e.code === 'E_RANGE');
  assert.throws(() => engine.experimentalThreeRateCandidateTdb(2451545, 15), e => e.code === 'E_UNSUPPORTED_BODY');
  assert.throws(() => engine.experimentalLatitudeRateCandidateTdb(NaN, 0), e => e.code === 'E_RANGE');
});

test('selected latitude cases improve against the local Swiss published speeds',
  { skip: kernel && process.env.SWISSEPH_PATH ? false : 'DE441_PATH and SWISSEPH_PATH required' }, () => {
    const require = createRequire(import.meta.url);
    const swiss = require(process.env.SWISSEPH_PATH);
    const { dirname, basename } = require('node:path');
    const { checkedSwiss, tdbToTt } = require('../scripts/validate-mvp1-swiss-rates.cjs');
    swiss.swe_set_ephe_path(dirname(kernel)); swiss.swe_set_jpl_file(basename(kernel));
    const engine = new Engine(kernel);
    for (const [jd, body] of [[-1931169, 0], [2086407, 1], [2086407, 4], [3145520, 9]]) {
      const reference = checkedSwiss(swiss.swe_calc(tdbToTt(jd), body, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED), body, jd);
      const result = engine.experimentalThreeRateCandidateTdb(jd, body);
      assert.ok(Math.abs(result.candidateXx[4] - reference[4]) < Math.abs(result.base.xx[4] - reference[4]));
    }
  });
