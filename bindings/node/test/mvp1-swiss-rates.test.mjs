// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { checkedSwiss, checkedLocal, expectError, summary, gateFailed } = require('../scripts/validate-mvp1-swiss-rates.cjs');

test('the reference must actually return JPL speeds and finite coordinates', () => {
  const reference = { rflag: 257, longitude: 123, latitude: 4, distance: 1, longitudeSpeed: -0.1, latitudeSpeed: 0.2, distanceSpeed: 0.01 };
  assert.deepEqual(checkedSwiss(reference, 4, 2451545), [123, 4, 1, -0.1, 0.2, 0.01]);
  for (const bad of [{ rflag: 256 }, { distance: 0 }, { longitudeSpeed: NaN }, { latitude: 91 }]) {
    assert.throws(() => checkedSwiss({ ...reference, ...bad }, 4, 2451545));
  }
  assert.deepEqual(checkedLocal({ xx: [123, 4, 1, -0.1, 0.2, 0.01] }), [123, 4, 1, -0.1, 0.2, 0.01]);
  assert.throws(() => checkedLocal({ xx: [123, 4, 1, Infinity, 0, 0] }));
  assert.equal(expectError(() => { throw Object.assign(new Error(), { code: 'E_CALC' }); }, 'E_CALC'), 'E_CALC');
  assert.throws(() => expectError(() => {}, 'E_CALC'));
  assert.throws(() => expectError(() => { throw Object.assign(new Error(), { code: 'OTHER' }); }, 'E_CALC'));
});

test('station sign disagreements and missing roots fail the finite compatibility gate', () => {
  const point = { body: 4, positionArcsec: 0.5, lonRateSwissArcsecDay: 0, lonRateDerivativeArcsecDay: 0, latRateSwissArcsecDay: 0, latRateDerivativeArcsecDay: 0, distanceRateSwissAuDay: 2, distanceRateDerivativeAuDay: 0, radialCandidateResidualAuDay: 1 };
  const points = [{ ...point, category: 'endpoint' }, { ...point, category: 'near-sun' }];
  const station = { status: 'ok', signsMatch: true, publishedSignsMatch: true, engineMinusSwissDerivativeSeconds: 0, engineMinusSwissPublishedSeconds: 10 };
  const pass = summary(points, [station]);
  assert.equal(gateFailed(pass), false);
  assert.equal(pass.points.radialCandidateImproved, 2);
  assert.equal(pass.points.radialRmsCandidateAuDay, 1);
  for (const badStation of [{ ...station, publishedSignsMatch: false }, { ...station, engineMinusSwissPublishedSeconds: null }, { status: 'missing-bracket' }]) {
    assert.equal(gateFailed(summary(points, [badStation])), true);
  }
  assert.equal(gateFailed(summary([{ ...point, category: 'endpoint', positionArcsec: 1.01 }, points[1]], [station])), true);
  assert.throws(() => summary(points.slice(0, 1), [station]));
  assert.throws(() => summary(points, [{ ...station, publishedSignsMatch: undefined }]));
});
