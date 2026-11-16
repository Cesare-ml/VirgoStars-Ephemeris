// Tests that need a classic DE441 file: set DE441_PATH, otherwise they are skipped.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { Engine, julday, constants, createSwissShapedFacade } = require('../index.js');
const path = process.env.DE441_PATH;
const opts = { skip: path ? false : 'DE441_PATH is not set' };
const SPEED = constants.SEFLG_SPEED;

test('J2000 Sun and Moon are plausible, with speeds only when requested', opts, () => {
  const e = new Engine(path);
  const sun = e.calcTdb(2451545.0, 0, SPEED);
  assert.ok(Math.abs(sun.xx[0] - 280.368) < 0.001, String(sun.xx[0]));
  assert.ok(Math.abs(sun.xx[3] - 1.0194) < 0.001);
  assert.equal(sun.rflag & constants.SEFLG_JPLEPH, constants.SEFLG_JPLEPH);
  assert.equal(sun.rflag & constants.SEFLG_SWIEPH, 0, 'never claims Swiss data');
  const noSpeed = e.calcTdb(2451545.0, 0, 0);
  assert.deepEqual(noSpeed.xx.slice(3), [0, 0, 0]);
  assert.equal(noSpeed.xx[0], sun.xx[0]);
  const moon = e.calcTdb(2451545.0, 1, SPEED);
  assert.ok(Math.abs(moon.xx[0] - 223.315) < 0.002);
});

test('unsupported flags and bodies are explicit errors, not approximations', opts, () => {
  const e = new Engine(path);
  // 16 TRUEPOS, 32 J2000, 64 NONUT, 1024 NOABERR alone, 16384 BARYCTR, 32768 TOPOCTR, 65536 SIDEREAL
  for (const flags of [constants.SEFLG_SWIEPH, constants.SEFLG_MOSEPH, 16, 32, 64, 1024, 16384, 32768, 65536]) {
    assert.throws(() => e.calcTdb(2451545.0, 0, flags), (x) => x.code === 'E_UNSUPPORTED_FLAG', String(flags));
  }
  for (const ipl of [-1, 10, 12, 15, 99]) {
    assert.throws(() => e.calcTdb(2451545.0, ipl, SPEED), (x) => x.code === 'E_UNSUPPORTED_BODY', String(ipl));
  }
  assert.throws(() => e.calcTdb(NaN, 0, SPEED), (x) => x.code === 'E_RANGE');
});

test('UT entry reports its time policy and refuses to extrapolate', opts, () => {
  const e = new Engine(path);
  const modern = e.calcUt(2451545.0, 0, SPEED);
  assert.equal(modern.timeSource, 'IERS_LEAP_SECONDS');
  const old = e.calcUt(julday(1950, 5, 15, 12, true), 0, SPEED);
  assert.match(old.timeSource, /^DELTA_T_POLYNOMIAL:Espenak-Meeus/);
  assert.ok(Math.abs(old.xx[0] - 54.0) < 1.0);
  for (const jd of [2415000, 2461584.5, 3000000]) {
    assert.throws(() => e.calcUt(jd, 0, SPEED), (x) => x.code === 'E_RANGE', String(jd));
  }
});

test('two engines are isolated and give identical answers; results are deterministic', opts, () => {
  const a = new Engine(path);
  const b = new Engine(path);
  const first = a.calcTdb(2460000.5, 5, SPEED).xx;
  b.calcTdb(2415100.5, 1, SPEED);
  assert.deepEqual(a.calcTdb(2460000.5, 5, SPEED).xx, first);
  assert.deepEqual(b.calcTdb(2460000.5, 5, SPEED).xx, first);
});

test('NOGDEFL changes only the direction, and the barycenter label is reported', opts, () => {
  const e = new Engine(path);
  const on = e.calcTdb(2451778.5, 5, SPEED);
  const off = e.calcTdb(2451778.5, 5, SPEED | constants.SEFLG_NOGDEFL);
  assert.ok(Math.abs(on.xx[2] / off.xx[2] - 1) < 1e-14, 'deflection leaves the range unchanged to rounding');
  assert.ok(on.notes.some((n) => n.includes('SYSTEM BARYCENTER')));
  assert.ok(on.notes.some((n) => n.includes('light deflection')));
  assert.ok(!off.notes.some((n) => n.includes('solar gravitational')));
  assert.equal(off.rflag & constants.SEFLG_NOGDEFL, constants.SEFLG_NOGDEFL);
});

test('facade: Swiss-shaped result keys, SWIEPH policy, error objects and callbacks', opts, () => {
  const e = new Engine(path);
  const strict = createSwissShapedFacade(e);
  const r = strict.swe_calc_ut(2451545.0, strict.SE_SUN, strict.SEFLG_SPEED | strict.SEFLG_SWIEPH);
  assert.equal(r.code, 'E_UNSUPPORTED_FLAG', 'SWIEPH is rejected by default');
  assert.equal(typeof r.error, 'string');

  const swe = createSwissShapedFacade(e, { acceptSwiephFlag: true });
  const sun = swe.swe_calc_ut(2451545.0, swe.SE_SUN, swe.SEFLG_SPEED | swe.SEFLG_SWIEPH);
  assert.deepEqual(Object.keys(sun), ['longitude', 'latitude', 'distance', 'longitudeSpeed', 'latitudeSpeed', 'distanceSpeed', 'rflag']);
  for (const key of Object.keys(sun)) assert.equal(typeof sun[key], 'number', key);
  const withProvenance = createSwissShapedFacade(e, { acceptSwiephFlag: true, provenance: true }).swe_calc_ut(2451545.0, 0, SPEED);
  assert.equal(typeof withProvenance.timeSource, 'string');
  assert.equal(sun.rflag & swe.SEFLG_SWIEPH, 0);
  assert.ok(Math.abs(sun.longitude - 280.37) < 0.01);

  // Unsupported things return error objects (do not throw), like the Swiss binding.
  assert.deepEqual(Object.keys(swe.swe_calc_ut(2451545.0, 15, swe.SEFLG_SPEED)), ['error']);
  assert.equal(swe.swe_calc_ut(2451545.0, 15, swe.SEFLG_SPEED).code, 'E_UNSUPPORTED_BODY');
  assert.ok(swe.swe_calc_ut(2451545.0, 0, 16).error, 'TRUEPOS is not offered');

  let received;
  const returned = swe.swe_calc_ut(2451545.0, swe.SE_SUN, swe.SEFLG_SPEED, (value) => {
    received = value;
  });
  assert.equal(returned.longitude, sun.longitude, 'the call also returns the result, like the Swiss binding');
  assert.equal(received.longitude, sun.longitude, 'callback variant is synchronous');
  assert.equal(swe.swe_get_planet_name(swe.SE_MARS).name, 'Mars');
});

test('facade: a planetary chart for a fictional instant has ten finite bodies', opts, () => {
  const swe = createSwissShapedFacade(new Engine(path), { acceptSwiephFlag: true });
  const jd = swe.swe_julday(1985, 6, 15, 10.5, swe.SE_GREG_CAL);
  for (let ipl = swe.SE_SUN; ipl <= swe.SE_PLUTO; ipl += 1) {
    const p = swe.swe_calc_ut(jd, ipl, swe.SEFLG_SPEED | swe.SEFLG_SWIEPH);
    assert.ok(!p.error, p.error);
    assert.ok(p.longitude >= 0 && p.longitude < 360 && Number.isFinite(p.longitudeSpeed), String(ipl));
  }
});

test('equatorial and heliocentric requests: keys, rflag bits, the Sun-from-the-Sun zero vector, Earth only heliocentric', opts, () => {
  const swe = createSwissShapedFacade(new Engine(path), { acceptSwiephFlag: true });
  const equ = swe.swe_calc(2451545.0, swe.SE_MARS, SPEED | constants.SEFLG_EQUATORIAL);
  assert.deepEqual(Object.keys(equ), ['rectAscension', 'declination', 'distance', 'rectAscensionSpeed', 'declinationSpeed', 'distanceSpeed', 'rflag']);
  assert.ok(Math.abs(equ.rectAscension - 330.516) < 0.001 && Math.abs(equ.declination + 13.1827) < 0.001);
  assert.equal(equ.rflag, 1 | 256 | 2048);
  const helio = swe.swe_calc(2451545.0, swe.SE_MARS, SPEED | constants.SEFLG_HELCTR);
  assert.deepEqual(Object.keys(helio), ['longitude', 'latitude', 'distance', 'longitudeSpeed', 'latitudeSpeed', 'distanceSpeed', 'rflag']);
  assert.equal(helio.rflag, 1 | 8 | 256 | 512 | 1024, 'heliocentric implies no deflection and no aberration');
  assert.ok(Math.abs(helio.distance - 1.3912) < 0.001);
  const sun = swe.swe_calc(2451545.0, swe.SE_SUN, SPEED | constants.SEFLG_HELCTR);
  assert.deepEqual([sun.longitude, sun.latitude, sun.distance, sun.longitudeSpeed], [0, 0, 0, 0]);
  const earth = swe.swe_calc(2451545.0, constants.SE_EARTH, SPEED | constants.SEFLG_HELCTR);
  assert.ok(Math.abs(earth.longitude - 100.368) < 0.001 && Math.abs(earth.distance - 0.98333) < 0.0001);
  assert.equal(swe.swe_calc(2451545.0, constants.SE_EARTH, SPEED).code, 'E_UNSUPPORTED_BODY', 'no geocentric Earth');
  const both = swe.swe_calc(2451545.0, swe.SE_MARS, SPEED | constants.SEFLG_HELCTR | constants.SEFLG_EQUATORIAL);
  assert.ok(Math.abs(both.rectAscension - 0.0496) < 0.001 && Math.abs(both.declination + 1.526) < 0.001);
  // Fixed stars do not offer these modes yet and say so.
  assert.equal(typeof swe.swe_get_planet_name(14).name, 'string');
});
