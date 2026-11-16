// House-angle provider tests: need the built addon, no ephemeris file.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { createHouseAngleProvider, houseAnglesUt } = require('../index.js');

const base = { ut1JulianDay: 2461354.5, ttJulianDay: 2461354.5 + 69.2 / 86400, latitudeDeg: 41.9028, longitudeDegEast: 12.4964 };

test('provider returns the contract fields and is immutable', () => {
  const provider = createHouseAngleProvider();
  assert.ok(Object.isFrozen(provider));
  const a = provider.calculateHouseAngles(base);
  assert.ok(Object.isFrozen(a));
  assert.equal(a.model, 'IAU2006_2000B_APPROXIMATE_SIDEREAL_TIME');
  for (const key of ['armcDeg', 'ascendantDeg', 'midheavenDeg', 'trueObliquityDeg']) {
    assert.ok(Number.isFinite(a[key]), key);
  }
  assert.equal(a.ut1JulianDay, base.ut1JulianDay);
  assert.equal(a.ttJulianDay, base.ttJulianDay);
  assert.ok(Math.abs(a.trueObliquityDeg - 23.4379) < 0.001);
});

test('east longitude adds to ARMC and wraps at 360 degrees', () => {
  const provider = createHouseAngleProvider();
  const west = provider.calculateHouseAngles({ ...base, longitudeDegEast: 0 }).armcDeg;
  const east = provider.calculateHouseAngles({ ...base, longitudeDegEast: 100 }).armcDeg;
  assert.ok(Math.abs(((east - west - 100 + 540) % 360) - 180) < 1e-9);
  for (let lon = -180; lon <= 180; lon += 36) {
    const armc = provider.calculateHouseAngles({ ...base, longitudeDegEast: lon }).armcDeg;
    assert.ok(armc >= 0 && armc < 360, String(lon));
  }
});

test('the recorded Rome reference is reproduced to a few milliarcseconds', () => {
  // The ascendant of the recorded 2026-11-10 fixture is 157.8264147901 deg.
  const a = createHouseAngleProvider().calculateHouseAngles(base);
  assert.ok(Math.abs(a.ascendantDeg - 157.8264147901) < 1e-6, String(a.ascendantDeg));
  assert.ok(Math.abs(a.midheavenDeg - 63.6782674613) < 1e-6, String(a.midheavenDeg));
});

test('invalid sites and unsupported epochs fail with E_RANGE', () => {
  const provider = createHouseAngleProvider();
  for (const latitudeDeg of [90, -90, 91, NaN, Infinity]) {
    assert.throws(() => provider.calculateHouseAngles({ ...base, latitudeDeg }), (e) => e.code === 'E_RANGE', String(latitudeDeg));
  }
  assert.throws(() => provider.calculateHouseAngles({ ...base, longitudeDegEast: NaN }), (e) => e.code === 'E_RANGE');
  assert.throws(() => provider.calculateHouseAngles({ ...base, ut1JulianDay: 2300000, ttJulianDay: 2300000 }), (e) => e.code === 'E_RANGE');
});

test('houseAnglesUt declares its time source and refuses to extrapolate', () => {
  assert.equal(houseAnglesUt(2461354.5, 41.9, 12.5).timeSource, 'IERS_LEAP_SECONDS');
  assert.match(houseAnglesUt(2433283.0, 41.9, 12.5).timeSource, /^DELTA_T_POLYNOMIAL:/);
  assert.throws(() => houseAnglesUt(2415000, 41.9, 12.5), (e) => e.code === 'E_RANGE');
  assert.throws(() => houseAnglesUt(2461584.5, 41.9, 12.5), (e) => e.code === 'E_RANGE');
});
