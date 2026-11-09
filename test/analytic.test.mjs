import test from 'node:test';
import assert from 'node:assert/strict';
import { approximateHeliocentricEcliptic, approximateRelativeToEmBary, approximateApparentSunUsno } from '../src/index.mjs';

test('analytic JPL short-range coefficients yield finite plausible heliocentric positions', () => {
  for (const name of ['Mercury','Venus','EM_BARY','Mars','Jupiter','Saturn','Uranus','Neptune']) {
    const p = approximateHeliocentricEcliptic(name, 2451545);
    assert.equal(p.positionAu.length, 3);
    assert.ok(p.positionAu.every(Number.isFinite));
    assert.equal(p.frame, 'MEAN_ECLIPTIC_J2000');
  }
  const mercury = approximateHeliocentricEcliptic('Mercury', 2451545);
  assert.ok(Math.abs(mercury.positionAu[0] + 0.130) < 0.02);
  assert.ok(Math.abs(mercury.positionAu[1] + 0.447) < 0.02);
});

test('EM barycentric approximation explicitly marks its center', () => {
  const p = approximateRelativeToEmBary('Mars', 2451545);
  assert.equal(p.center, 'EARTH_MOON_BARYCENTER_NOT_EARTH');
  const sun = approximateRelativeToEmBary('Sun', 2451545);
  assert.equal(sun.center, p.center);
  assert.ok(Math.abs(sun.positionAu[0]) < 2);
});

test('USNO apparent Sun prototype near J2000 is around 280 degrees and 1AU', () => {
  const p = approximateApparentSunUsno(2451545);
  assert.ok(p.longitudeDeg > 275 && p.longitudeDeg < 285);
  assert.ok(Math.abs(p.distanceAu - 0.983) < 0.01);
  assert.equal(p.corrections, 'APPARENT_APPROX');
});

test('analytic fallbacks fail explicitly outside valid dates or for missing Moon', () => {
  assert.throws(()=>approximateHeliocentricEcliptic('Mars', 2500000), e=>e.code === 'OUT_OF_ANALYTIC_RANGE');
  assert.throws(()=>approximateHeliocentricEcliptic('Moon', 2451545), e=>e.code === 'UNSUPPORTED_BODY');
  assert.throws(()=>approximateApparentSunUsno(2000000), e=>e.code === 'OUT_OF_ANALYTIC_RANGE');
});
