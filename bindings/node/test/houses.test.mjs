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

test('native binding returns the rising polar Ascendant in all seven synthetic latitude cases', () => {
  // Modification 2026-11-18: tests the Rust-backed Node provider at the same
  // time/longitude as the owner's earlier receipt. Numerical comparison is a
  // native regression, NOT an independent astronomical/house-cusp reference.
  const provider = createHouseAngleProvider();
  const j2000 = {ut1JulianDay: 2451545, ttJulianDay: 2451545.0008,
    longitudeDegEast: 12.4964};
  const samples = [
    [-80, 7.16881630295217],
    [-66.55, 12.481452337837476],
    [-45, 17.42420249955754],
    [0, 24.778143295766903],
    [45, 41.09682061519927],
    [66.55, 100.47525199627685],
    [80, 344.5489205114233],
  ];
  const expectedArmcDeg = 292.95347186375847;
  const expectedMcDeg = 291.23482098822;
  const circularDistance = (a, b) => Math.abs((a - b + 540) % 360 - 180);
  for (const [latitudeDeg, expectedAscDeg] of samples) {
    const result = provider.calculateHouseAngles({...j2000, latitudeDeg});
    assert.ok(circularDistance(result.ascendantDeg, expectedAscDeg) < 1e-7,
      `${latitudeDeg}°: native Ascendant differs from owner fixture`);
    assert.ok(circularDistance(result.armcDeg, expectedArmcDeg) < 1e-7,
      `${latitudeDeg}°: native ARMC unexpectedly changed`);
    assert.ok(circularDistance(result.midheavenDeg, expectedMcDeg) < 1e-7,
      `${latitudeDeg}°: native Midheaven unexpectedly changed`);

    const radians = Math.PI / 180;
    const phi = latitudeDeg * radians;
    const theta = result.armcDeg * radians;
    const epsilon = result.trueObliquityDeg * radians;
    const longitude = result.ascendantDeg * radians;
    const x = Math.cos(longitude);
    const y = Math.cos(epsilon) * Math.sin(longitude);
    const z = Math.sin(epsilon) * Math.sin(longitude);
    const sinAltitude = Math.sin(phi) * z + Math.cos(phi) *
      (x * Math.cos(theta) + y * Math.sin(theta));
    const risingDerivative = Math.cos(phi) *
      (y * Math.cos(theta) - x * Math.sin(theta));
    assert.ok(Math.abs(sinAltitude) < 1e-10, `${latitudeDeg}°: not on horizon`);
    assert.ok(risingDerivative > 0, `${latitudeDeg}°: setting rather than rising`);
  }
});

test('UT convenience binding also selects rising ecliptic crossing at polar latitude', () => {
  const result = houseAnglesUt(2451545, 80, 12.4964);
  const radians = Math.PI / 180;
  const theta = result.armcDeg * radians;
  const epsilon = result.trueObliquityDeg * radians;
  const longitude = result.ascendantDeg * radians;
  const risingDerivative = Math.cos(80 * radians) *
    (Math.cos(epsilon) * Math.sin(longitude) * Math.cos(theta) -
      Math.cos(longitude) * Math.sin(theta));
  assert.ok(risingDerivative > 0, 'houseAnglesUt must forward the rising Rust angle');
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
