// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require = createRequire(import.meta.url);
const api = require('../index.js');

test('experimental house provider accepts explicit historical UT1 and TT', () => {
  const provider = api.createExperimentalHistoricalHouseAngleProvider();
  for (const year of [-12999, 0, 2000, 3999]) {
    const ut1JulianDay = api.julday(year, 3, 20, 0, true);
    const input = {ut1JulianDay, ttJulianDay: ut1JulianDay + 69 / 86400,
      latitudeDeg: 41.9, longitudeDegEast: 12.5};
    const out = provider.calculateHouseAngles(input);
    assert.equal(out.model, 'EXPERIMENTAL_LONG_TERM_SIDEREAL_TIME');
    for (const key of Object.keys(input)) assert.equal(out[key], input[key]);
    for (const key of ['armcDeg', 'ascendantDeg', 'midheavenDeg', 'trueObliquityDeg'])
      assert.ok(Number.isFinite(out[key]));
  }
  assert.throws(() => provider.calculateHouseAngles({ut1JulianDay: NaN, ttJulianDay: 2451545,
    latitudeDeg: 0, longitudeDegEast: 0}));
});
