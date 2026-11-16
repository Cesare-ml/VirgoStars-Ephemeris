// Horizon coordinates, refraction, rise/set/transit.
//
// Parts that need no data run always; parts that need a classic DE441 file skip
// without DE441_PATH; the comparison with an installed `swisseph` package is LOCAL
// USE ONLY (AGPL, never a dependency) and skips without SWISSEPH_PATH. Nothing the
// reference returns is stored; only agreement is asserted.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, basename } from 'node:path';

const require = createRequire(import.meta.url);
const { Engine, constants: C, createSwissShapedFacade, refrac, refracExtended } = require('../index.js');
const de441 = process.env.DE441_PATH;
const swissPath = process.env.SWISSEPH_PATH;
const needsKernel = { skip: de441 ? false : 'DE441_PATH is not set' };
const needsReference = { skip: de441 && swissPath ? false : 'DE441_PATH and SWISSEPH_PATH are required' };

const ROME = { lon: 12.5, lat: 41.9 };
const JD_MARCH_20_2024_0H = 2460389.5;

// ------------------------------------------------------------ no data needed

test('plain refraction: no atmosphere, standard values and the horizon rule', () => {
  assert.equal(refrac(10, 0, 15, true), 10, 'pressure 0 means no atmosphere');
  assert.ok(Math.abs(refrac(10, 1013.25, 15, true) - 10.088848272) < 1e-8);
  assert.equal(refrac(-3, 1013.25, 15, true), -3, 'stays below the horizon: unchanged');
  assert.ok(refrac(0, 1013.25, 15, true) > 0.47, 'about 28 arcminutes at the horizon');
});

test('extended refraction: round trip, dip and the horizon rule', () => {
  const up = refracExtended(5, 0, 1013.25, 15, 0.0065, true);
  const back = refracExtended(up.apparentAltitudeDeg, 0, 1013.25, 15, 0.0065, false);
  assert.ok(Math.abs(back.trueAltitudeDeg - 5) < 1e-9);
  assert.equal(refracExtended(-0.6, 0, 1013.25, 15, 0.0065, true).apparentAltitudeDeg, -0.6);
  assert.ok(refracExtended(-0.5, 0, 1013.25, 15, 0.0065, true).apparentAltitudeDeg > 0);
  const high = refracExtended(10, 2000, 1013.25, 15, 0.0065, true);
  assert.ok(Math.abs(high.dipDeg + 1.2421) < 1e-4, `dip ${high.dipDeg}`);
});

test('the facade exposes the refraction calls with the reference result shapes', () => {
  const facade = createSwissShapedFacade({});
  assert.deepEqual(Object.keys(facade.swe_refrac(10, 1013.25, 15, C.SE_TRUE_TO_APP)), ['refraction']);
  assert.deepEqual(
    Object.keys(facade.swe_refrac_extended(10, 0, 1013.25, 15, 0.0065, C.SE_TRUE_TO_APP)),
    ['refraction', 'trueAltitude', 'apparentAltitude', 'horizonDip'],
  );
  const ext = facade.swe_refrac_extended(10, 0, 1013.25, 15, 0.0065, C.SE_TRUE_TO_APP);
  assert.equal(ext.refraction, ext.apparentAltitude, 'the reference names the resulting altitude "refraction"');
  assert.equal(C.SE_CALC_RISE | C.SE_CALC_SET | C.SE_CALC_MTRANSIT | C.SE_CALC_ITRANSIT, 15);
});

// ------------------------------------------------------------ kernel needed

test('azalt: the Sun at noon is high in the south, with south-based azimuth', needsKernel, () => {
  const engine = new Engine(de441);
  const facade = createSwissShapedFacade(engine);
  const jd = JD_MARCH_20_2024_0H + 0.5; // 12:00 UT
  const sun = facade.swe_calc_ut(jd, 0, C.SEFLG_JPLEPH);
  const r = facade.swe_azalt(jd, C.SE_ECL2HOR, ROME.lon, ROME.lat, 0, 1013.25, 15, sun.longitude, sun.latitude, 0);
  assert.deepEqual(Object.keys(r), ['azimuth', 'trueAltitude', 'apparentAltitude']);
  assert.ok(Math.abs(r.trueAltitude - 47.150371) < 1e-4, String(r.trueAltitude));
  assert.ok(r.apparentAltitude > r.trueAltitude && r.apparentAltitude - r.trueAltitude < 0.02);
  assert.ok(Math.abs(r.azimuth - 15.8024) < 1e-3, 'a little west of south: 15.8 degrees from the south toward the west');
  // Same direction through equatorial coordinates
  const eq = facade.swe_calc_ut(jd, 0, C.SEFLG_JPLEPH | C.SEFLG_EQUATORIAL);
  const r2 = facade.swe_azalt(jd, C.SE_EQU2HOR, ROME.lon, ROME.lat, 0, 1013.25, 15, eq.rectAscension, eq.declination, 0);
  assert.ok(Math.abs(r2.trueAltitude - r.trueAltitude) < 1e-6 && Math.abs(r2.azimuth - r.azimuth) < 1e-6);
});

test('azalt_rev inverts azalt for the true altitude', needsKernel, () => {
  const facade = createSwissShapedFacade(new Engine(de441));
  const jd = JD_MARCH_20_2024_0H + 0.3;
  for (const [flag, back] of [[C.SE_ECL2HOR, C.SE_HOR2ECL], [C.SE_EQU2HOR, C.SE_HOR2EQU]]) {
    const r = facade.swe_azalt(jd, flag, ROME.lon, ROME.lat, 0, 0, 0, 100, 20, 0);
    const x = facade.swe_azalt_rev(jd, back, ROME.lon, ROME.lat, 0, r.azimuth, r.trueAltitude);
    assert.deepEqual(Object.keys(x), ['azimuth', 'trueAltitude']);
    assert.ok(Math.abs(x.azimuth - 100) < 1e-8 && Math.abs(x.trueAltitude - 20) < 1e-8, JSON.stringify(x));
  }
});

test('Sun rise, set and transits at Rome on 2024-03-20 (engine baseline)', needsKernel, () => {
  const facade = createSwissShapedFacade(new Engine(de441), { acceptSwiephFlag: true });
  const at = (rsmi, ipl = 0) =>
    facade.swe_rise_trans(JD_MARCH_20_2024_0H, ipl, '', C.SEFLG_SWIEPH, rsmi, ROME.lon, ROME.lat, 0, 0, 0);
  const hours = (r) => (r.transitTime - JD_MARCH_20_2024_0H) * 24;
  const rise = at(C.SE_CALC_RISE);
  assert.deepEqual(Object.keys(rise), ['name', 'transitTime']);
  assert.equal(rise.name, '');
  const baseline = { rise: 2460389.71705, set: 2460390.224168, upper: 2460389.97036, lower: 2460390.470256 };
  assert.ok(Math.abs(rise.transitTime - baseline.rise) * 86400 < 2, 'rise');
  assert.ok(Math.abs(at(C.SE_CALC_SET).transitTime - baseline.set) * 86400 < 2, 'set');
  assert.ok(Math.abs(at(C.SE_CALC_MTRANSIT).transitTime - baseline.upper) * 86400 < 2, 'transit');
  assert.ok(Math.abs(at(C.SE_CALC_ITRANSIT).transitTime - baseline.lower) * 86400 < 2, 'lower transit');
  // Almanac sanity (Rome, equinox): sunrise about 06:12 CET, sunset about 18:24 CET.
  assert.ok(hours(rise) > 5.1 && hours(rise) < 5.3, `sunrise ${hours(rise)} h UT`);
  const set = at(C.SE_CALC_SET);
  assert.ok(hours(set) > 17.3 && hours(set) < 17.5, `sunset ${hours(set)} h UT`);
  // Options move the events the expected way.
  const noRefraction = at(C.SE_CALC_RISE | C.SE_BIT_NO_REFRACTION).transitTime;
  const centre = at(C.SE_CALC_RISE | C.SE_BIT_DISC_CENTER).transitTime;
  const bottom = at(C.SE_CALC_RISE | C.SE_BIT_DISC_BOTTOM).transitTime;
  assert.ok(rise.transitTime < centre && centre < bottom, 'upper limb first, then centre, then lower limb');
  assert.ok(noRefraction > rise.transitTime, 'without refraction the Sun rises later');
  const civil = at(C.SE_CALC_RISE | C.SE_BIT_CIVIL_TWILIGHT).transitTime;
  assert.ok(civil < rise.transitTime && (rise.transitTime - civil) * 1440 > 20, 'civil dawn is 25-30 minutes earlier');
});

test('planets and the Moon: events exist, are ordered after the start and bodies differ', needsKernel, () => {
  const facade = createSwissShapedFacade(new Engine(de441));
  const seen = new Set();
  for (const ipl of [1, 2, 3, 4, 5, 6, 7, 8, 9]) {
    for (const rsmi of [1, 2, 4, 8]) {
      const r = facade.swe_rise_trans(JD_MARCH_20_2024_0H, ipl, '', 1, rsmi, ROME.lon, ROME.lat, 0, 1013.25, 15);
      assert.ok(r.transitTime > JD_MARCH_20_2024_0H && r.transitTime < JD_MARCH_20_2024_0H + 26 / 24, `${ipl}/${rsmi}`);
      seen.add(r.transitTime.toFixed(3));
    }
  }
  assert.ok(seen.size > 30, 'distinct times for distinct bodies');
});

test('polar day and night give -2, like the reference; the engine gives null', needsKernel, () => {
  const engine = new Engine(de441);
  const facade = createSwissShapedFacade(engine);
  const midsummer = 2460482.5; // 2024-06-21 0h UT
  const r = facade.swe_rise_trans(midsummer, 0, '', 1, C.SE_CALC_RISE, 18.96, 69.65, 0, 0, 0);
  assert.deepEqual(r, { name: '', transitTime: -2 });
  assert.equal(engine.riseTransUt(midsummer, 0, C.SE_CALC_RISE, 18.96, 69.65, 0, 0, 0), null);
  // ...but the Sun still transits, and a longer window finds the next sunset in the engine.
  assert.ok(facade.swe_rise_trans(midsummer, 0, '', 1, C.SE_CALC_MTRANSIT, 18.96, 69.65, 0, 0, 0).transitTime > midsummer);
  const later = engine.riseTransUt(midsummer, 0, C.SE_CALC_SET, 18.96, 69.65, 0, 0, 0, 31);
  assert.ok(later === null || later > midsummer + 25, 'the midnight Sun lasts well over a month');
});

test('wrong call shapes and unsupported options are errors, never garbage', needsKernel, () => {
  const facade = createSwissShapedFacade(new Engine(de441));
  // The shape used by the backend helper mezzogiorno-astronomico.js: position array, no star name.
  const wrong = facade.swe_rise_trans(JD_MARCH_20_2024_0H, 0, [12.5, 41.9, 0], 2, C.SE_CALC_MTRANSIT, null);
  assert.equal(wrong.code, 'E_RANGE');
  assert.match(wrong.error, /different shape/);
  const star = facade.swe_rise_trans(JD_MARCH_20_2024_0H, -1, 'Regulus', 1, 1, ROME.lon, ROME.lat, 0, 0, 0);
  assert.equal(star.code, 'E_UNSUPPORTED_BODY');
  const hindu = facade.swe_rise_trans(JD_MARCH_20_2024_0H, 0, '', 1, 1 | 128 | 256 | 512, ROME.lon, ROME.lat, 0, 0, 0);
  assert.equal(hindu.code, 'E_UNSUPPORTED_FLAG');
  assert.equal(facade.swe_rise_trans(JD_MARCH_20_2024_0H, 0, '', 1, 3, ROME.lon, ROME.lat, 0, 0, 0).code, 'E_RANGE', 'two events at once');
  assert.equal(facade.swe_rise_trans(JD_MARCH_20_2024_0H, 0, '', 4, 1, ROME.lon, ROME.lat, 0, 0, 0).code, 'E_UNSUPPORTED_FLAG', 'Moshier flag');
  assert.equal(facade.swe_azalt(JD_MARCH_20_2024_0H, 7, ROME.lon, ROME.lat, 0, 0, 0, 0, 0, 0).code, 'E_RANGE');
  assert.equal(facade.swe_rise_trans(JD_MARCH_20_2024_0H, 0, '', 1, 1, ROME.lon, 90, 0, 0, 0).code, 'E_RANGE', 'latitude 90');
});

// ----------------------------------------------- local comparison (AGPL, never stored)

test('agreement with the installed swisseph: azalt, refraction, rise/set/transit', needsReference, () => {
  const swiss = require(swissPath);
  swiss.swe_set_ephe_path(dirname(de441));
  swiss.swe_set_jpl_file(basename(de441));
  const ours = createSwissShapedFacade(new Engine(de441), { acceptSwiephFlag: true });

  const jd = swiss.swe_julday(2024, 3, 20, 12, swiss.SE_GREG_CAL);
  const sun = swiss.swe_calc_ut(jd, 0, 1);
  for (const [p, t, alt] of [[0, 0, 0], [1013.25, 15, 0], [850, -5, 1500]]) {
    const a = swiss.swe_azalt(jd, swiss.SE_ECL2HOR, ROME.lon, ROME.lat, alt, p, t, sun.longitude, sun.latitude, 0);
    const b = ours.swe_azalt(jd, C.SE_ECL2HOR, ROME.lon, ROME.lat, alt, p, t, sun.longitude, sun.latitude, 0);
    assert.deepEqual(Object.keys(b), Object.keys(a));
    for (const key of Object.keys(a)) assert.ok(Math.abs(b[key] - a[key]) * 3600 < 0.05, `${key} ${p}/${t}`);
  }
  for (const alt of [-0.4, 0.3, 2, 10, 17.9, 18, 45, 89]) {
    const a = swiss.swe_refrac_extended(alt, 0, 1013.25, 15, 0.0065, swiss.SE_TRUE_TO_APP);
    const b = ours.swe_refrac_extended(alt, 0, 1013.25, 15, 0.0065, C.SE_TRUE_TO_APP);
    assert.deepEqual(Object.keys(b), Object.keys(a));
    assert.ok(Math.abs(b.apparentAltitude - a.apparentAltitude) * 3600 < 0.05, `extended ${alt}`);
    const c = swiss.swe_refrac(alt, 1013.25, 15, swiss.SE_TRUE_TO_APP);
    const d = ours.swe_refrac(alt, 1013.25, 15, C.SE_TRUE_TO_APP);
    assert.ok(Math.abs(d.refraction - c.refraction) * 3600 < 1e-6, `plain ${alt}`);
  }
  for (const [ipl, name] of [[0, 'Sun'], [1, 'Moon'], [3, 'Venus'], [5, 'Jupiter']]) {
    for (const [rsmi, tolerance] of [[1, 2], [2, 2], [4, 2], [8, 2]]) {
      const a = swiss.swe_rise_trans(JD_MARCH_20_2024_0H, ipl, '', 1, rsmi, ROME.lon, ROME.lat, 0, 0, 0);
      const b = ours.swe_rise_trans(JD_MARCH_20_2024_0H, ipl, '', 2, rsmi, ROME.lon, ROME.lat, 0, 0, 0);
      assert.deepEqual(Object.keys(b), Object.keys(a));
      assert.equal(b.name, a.name);
      assert.ok(Math.abs(b.transitTime - a.transitTime) * 86400 < tolerance, `${name} ${rsmi}`);
    }
  }
  // Polar day: both say "no event".
  const polar = [2460482.5, 0, '', 1, 1, 18.96, 69.65, 0, 0, 0];
  assert.equal(ours.swe_rise_trans(...polar).transitTime, swiss.swe_rise_trans(...polar).transitTime);
  // Options
  for (const bits of [256, 512, 768, 8192, 16384, 1024, 2048, 4096]) {
    const a = swiss.swe_rise_trans(JD_MARCH_20_2024_0H, 0, '', 1, 1 | bits, ROME.lon, ROME.lat, 0, 1013.25, 15);
    const b = ours.swe_rise_trans(JD_MARCH_20_2024_0H, 0, '', 2, 1 | bits, ROME.lon, ROME.lat, 0, 1013.25, 15);
    assert.ok(Math.abs(b.transitTime - a.transitTime) * 86400 < 2, `option ${bits}`);
  }
});
