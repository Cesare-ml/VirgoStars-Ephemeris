// Shape-contract tests of the Swiss-shaped facade against an installed `swisseph` package.
//
// LOCAL USE ONLY: the swisseph package is AGPL and is never a dependency of this
// project. These tests load it from a caller-supplied path and skip otherwise:
//   SWISSEPH_PATH=<dir of the installed swisseph package> DE441_PATH=<de441.eph> npm test
// Nothing it returns is stored; only agreement is asserted.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, basename } from 'node:path';

const require = createRequire(import.meta.url);
const { Engine, createSwissShapedFacade } = require('../index.js');
const swissPath = process.env.SWISSEPH_PATH;
const de441 = process.env.DE441_PATH;
const opts = { skip: swissPath && de441 ? false : 'SWISSEPH_PATH and DE441_PATH are required' };

let swiss;
let ours;
function setup() {
  if (!swiss) {
    swiss = require(swissPath);
    swiss.swe_set_ephe_path(dirname(de441));
    swiss.swe_set_jpl_file(basename(de441));
    ours = createSwissShapedFacade(new Engine(de441));
  }
}
const SPEED_JPL = 256 | 1; // SEFLG_SPEED | SEFLG_JPLEPH, passed to both
const wrap = (d) => ((d + 540) % 360) - 180;

test('result objects have exactly the same keys and value types', opts, () => {
  setup();
  const jd = swiss.swe_julday(1985, 6, 15, 10.5, swiss.SE_GREG_CAL);
  for (let ipl = 0; ipl <= 9; ipl += 1) {
    const a = swiss.swe_calc_ut(jd, ipl, SPEED_JPL);
    const b = ours.swe_calc_ut(jd, ipl, SPEED_JPL);
    assert.deepEqual(Object.keys(b), Object.keys(a), `keys of body ${ipl}`);
    for (const key of Object.keys(a)) assert.equal(typeof b[key], typeof a[key], `${key} of body ${ipl}`);
    assert.equal(b.rflag, a.rflag, `rflag of body ${ipl} (JPL mode)`);
  }
});

test('without SEFLG_SPEED both return zero speeds', opts, () => {
  setup();
  const jd = swiss.swe_julday(1985, 6, 15, 10.5, swiss.SE_GREG_CAL);
  const a = swiss.swe_calc_ut(jd, 0, 1);
  const b = ours.swe_calc_ut(jd, 0, 1);
  for (const key of ['longitudeSpeed', 'latitudeSpeed', 'distanceSpeed']) assert.equal(b[key], a[key], key);
});

test('positions agree within the documented UT-policy tolerance', opts, () => {
  setup();
  for (const [y, m, d, h] of [[1985, 6, 15, 10.5], [2000, 1, 1, 12], [2010, 11, 3, 4.25], [1962, 3, 21, 8]]) {
    const jd = swiss.swe_julday(y, m, d, h, swiss.SE_GREG_CAL);
    for (let ipl = 0; ipl <= 9; ipl += 1) {
      const a = swiss.swe_calc_ut(jd, ipl, SPEED_JPL);
      const b = ours.swe_calc_ut(jd, ipl, SPEED_JPL);
      const tolerance = ipl === 1 ? 0.8 : 0.1; // arcsec: Moon is limited by UT1-UTC / DeltaT
      assert.ok(Math.abs(wrap(b.longitude - a.longitude)) * 3600 < tolerance, `${y} body ${ipl}`);
      assert.ok(Math.abs(b.latitude - a.latitude) * 3600 < tolerance, `${y} body ${ipl} latitude`);
    }
  }
});

test('error objects have the single enumerable key "error"', opts, () => {
  setup();
  const a = swiss.swe_calc_ut(2451545, 9999, SPEED_JPL);
  const b = ours.swe_calc_ut(2451545, 9999, SPEED_JPL);
  assert.deepEqual(Object.keys(b), Object.keys(a));
  assert.deepEqual(Object.keys(b), ['error']);
  assert.equal(typeof b.error, 'string');
  assert.equal(b.code, 'E_UNSUPPORTED_BODY', 'machine-readable code stays available, non-enumerable');
});

test('callback: invoked synchronously, before the call returns, and the call also returns the result', opts, () => {
  setup();
  for (const [name, api] of [['swiss', swiss], ['ours', ours]]) {
    const order = [];
    const returned = api.swe_calc_ut(2451545, 0, SPEED_JPL, (value) => order.push(['callback', typeof value]));
    order.push(['after']);
    assert.deepEqual(order.map((x) => x[0]), ['callback', 'after'], name);
    assert.equal(typeof returned, 'object', `${name} returns the result as well`);
    assert.equal(typeof returned.longitude, 'number', name);
  }
});

test('swe_julday matches arithmetically, including overflow and truncation', opts, () => {
  setup();
  const cases = [[2001, 13, 1, 0], [2001, 0, 1, 0], [2001, 14, 15, 3], [2001, 1, 0, 0], [2001, 1, -1, 0], [2001, 1, 32, 0],
    [2001, 2, 29, 0], [2001, 1, 1, -1], [2001, 1, 1, 48], [2001, 1, 1, 23.9999999], [1582, 10, 10, 0], [0, 1, 1, 0],
    [-4712, 1, 1, 12], [2001, 1, 1.5, 0], [2001.5, 1, 1, 0], [2001, 1.5, 1, 0], [2026, 11, 16, 12.75]];
  for (const c of cases) {
    for (const cal of [1, 0]) {
      const a = swiss.swe_julday(...c, cal);
      const b = ours.swe_julday(...c, cal);
      assert.ok(Math.abs(a - b) < 1e-9, `${JSON.stringify(c)} cal ${cal}: ${a} vs ${b}`);
    }
  }
  assert.ok(Number.isNaN(swiss.swe_julday(2000, 1)) && Number.isNaN(ours.swe_julday(2000, 1)), 'missing arguments give NaN');
});

test('swe_revjul matches on a grid and in both calendars', opts, () => {
  setup();
  for (const cal of [1, 0]) {
    for (let jd = 1721059.5; jd < 2488069.5; jd += 41233.377) {
      const a = swiss.swe_revjul(jd, cal);
      const b = ours.swe_revjul(jd, cal);
      assert.deepEqual(Object.keys(b), Object.keys(a));
      assert.equal(b.year, a.year); assert.equal(b.month, a.month); assert.equal(b.day, a.day);
      assert.ok(Math.abs(b.hour - a.hour) < 1e-6, `${jd}`);
    }
  }
});

test('SE_ECL_NUT: same slots and values within a fraction of a milliarcsecond', opts, () => {
  setup();
  for (const jd of [2415100.5, 2433282.5, 2451545, 2460000.5, 2480000.5]) {
    const a = swiss.swe_calc(jd, swiss.SE_ECL_NUT, 0);
    const b = ours.swe_calc(jd, ours.SE_ECL_NUT, 0);
    assert.deepEqual(Object.keys(b), Object.keys(a));
    for (const key of ['longitude', 'latitude', 'distance', 'longitudeSpeed']) {
      assert.ok(Math.abs(a[key] - b[key]) < 3e-7, `${jd} ${key}: ${a[key]} vs ${b[key]}`);
    }
    assert.equal(b.latitudeSpeed, 0); assert.equal(b.distanceSpeed, 0); assert.equal(b.rflag, a.rflag);
  }
});

test('swe_get_planet_name: the ten bodies share names; unknown numbers print as the number', opts, () => {
  setup();
  for (let i = 0; i <= 9; i += 1) assert.equal(ours.swe_get_planet_name(i).name, swiss.swe_get_planet_name(i).name);
  assert.equal(ours.swe_get_planet_name(9999).name, swiss.swe_get_planet_name(9999).name);
});

test('the constants the backend reads have the same values', opts, () => {
  setup();
  for (const name of ['SE_SUN', 'SE_MOON', 'SE_PLUTO', 'SE_GREG_CAL', 'SE_JUL_CAL', 'SE_ECL_NUT', 'SEFLG_SPEED', 'SEFLG_SWIEPH', 'SEFLG_JPLEPH', 'SEFLG_MOSEPH', 'SEFLG_NOGDEFL']) {
    assert.equal(ours[name], swiss[name], name);
  }
});

test('equatorial output: same keys, same rflag, positions within the common floor', opts, () => {
  setup();
  const jd = 2451545.0 + 12345.678;
  for (let ipl = 0; ipl <= 9; ipl += 1) {
    const flags = SPEED_JPL | 2048;
    const a = swiss.swe_calc(jd, ipl, flags);
    const b = ours.swe_calc(jd, ipl, flags);
    assert.deepEqual(Object.keys(b), Object.keys(a), `keys of body ${ipl}`);
    assert.equal(b.rflag, a.rflag, `rflag of body ${ipl}`);
    assert.ok(Math.abs(wrap(b.rectAscension - a.rectAscension)) * 3600 < 0.01, `RA body ${ipl}`);
    assert.ok(Math.abs(b.declination - a.declination) * 3600 < 0.01, `Dec body ${ipl}`);
  }
});

test('heliocentric output: same keys and rflag; Moon, Earth and the outer planets agree closely', opts, () => {
  setup();
  const jd = 2451545.0 + 12345.678;
  const sunA = swiss.swe_calc(jd, 0, SPEED_JPL | 8);
  const sunB = ours.swe_calc(jd, 0, SPEED_JPL | 8);
  assert.deepEqual(sunB, sunA, 'the Sun seen from the Sun is the same zero result');
  for (const [ipl, toleranceArcsec] of [[1, 0.01], [14, 0.01], [5, 0.05], [6, 0.05], [7, 0.05], [8, 0.05], [9, 0.05]]) {
    const a = swiss.swe_calc(jd, ipl, SPEED_JPL | 8);
    const b = ours.swe_calc(jd, ipl, SPEED_JPL | 8);
    assert.deepEqual(Object.keys(b), Object.keys(a), `keys of body ${ipl}`);
    assert.equal(b.rflag, a.rflag, `rflag of body ${ipl}`);
    assert.ok(Math.abs(wrap(b.longitude - a.longitude)) * 3600 < toleranceArcsec, `longitude body ${ipl}`);
    assert.ok(Math.abs(b.latitude - a.latitude) * 3600 < toleranceArcsec, `latitude body ${ipl}`);
  }
});
