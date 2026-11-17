// swe_houses / swe_houses_pos facade.
//
// The house systems live in astrological-functions and this package never depends on
// it: the tests load it from a caller-supplied checkout.
//   ASTROLOGICAL_FUNCTIONS_SRC=<.../astrological-functions/src> DE441_PATH=<de441.eph> npm test
// The comparison with an installed `swisseph` (AGPL, local use only, never a dependency)
// additionally needs SWISSEPH_PATH. Nothing the reference returns is stored.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { dirname, basename, join } from 'node:path';

const require = createRequire(import.meta.url);
const { Engine, constants: C, createSwissShapedFacade, createHouseAngleProvider } = require('../index.js');
const de441 = process.env.DE441_PATH;
const afSrc = process.env.ASTROLOGICAL_FUNCTIONS_SRC;
const swissPath = process.env.SWISSEPH_PATH;
const needs = { skip: de441 && afSrc ? false : 'DE441_PATH and ASTROLOGICAL_FUNCTIONS_SRC are required' };
const needsReference = { skip: de441 && afSrc && swissPath ? false : 'DE441_PATH, ASTROLOGICAL_FUNCTIONS_SRC and SWISSEPH_PATH are required' };

let methods;
async function load() {
  if (!methods) {
    const a = await import(pathToFileURL(join(afSrc, 'house-set-with-warnings.mjs')).href);
    const b = await import(pathToFileURL(join(afSrc, 'placidus-position.mjs')).href);
    methods = { calculateHouseSetWithWarnings: a.calculateHouseSetWithWarnings, calculatePlacidusHousePosition: b.calculatePlacidusHousePosition };
  }
  return methods;
}
const wrap = (d) => ((d + 540) % 360) - 180;
const JD = 2460390.0; // 2024-03-20 12:00 UT
const KEYS = ['house', 'ascendant', 'mc', 'armc', 'vertex', 'equatorialAscendant', 'kochCoAscendant', 'munkaseyCoAscendant', 'munkaseyPolarAscendant'];

test('the point constants and the provider carry the extra points', () => {
  assert.equal(C.SE_VERTEX, 3);
  assert.equal(C.SE_POLASC, 7);
  assert.ok(createHouseAngleProvider());
});

test('without house methods the call is an explicit error', needs, () => {
  const facade = createSwissShapedFacade(new Engine(de441));
  const r = facade.swe_houses(JD, 41.9, 12.5, 'P');
  assert.equal(r.code, 'E_UNSUPPORTED_BODY');
  assert.match(r.error, /astrological-functions/);
});

test('swe_houses returns exactly the reference keys, for the five supported systems', needs, async () => {
  const facade = createSwissShapedFacade(new Engine(de441), { houseMethods: await load() });
  for (const h of ['V', 'P', 'K', 'R', 'O']) {
    const r = facade.swe_houses(JD, 41.9, 12.5, h);
    assert.deepEqual(Object.keys(r), KEYS, h);
    assert.equal(r.house.length, 12);
    assert.ok(r.house.every((x) => Number.isFinite(x) && x >= 0 && x < 360));
    // Rome, 2024-03-20 12:00 UT: engine baseline (also within 0.1 arcsecond of the reference)
    assert.ok(Math.abs(wrap(r.ascendant - 118.462327)) < 1e-4, `asc ${r.ascendant}`);
    assert.ok(Math.abs(wrap(r.mc - 11.973241)) < 1e-4, `mc ${r.mc}`);
    assert.ok(Math.abs(wrap(r.vertex - 254.724055)) < 1e-4, `vertex ${r.vertex}`);
    assert.ok(Math.abs(wrap(r.equatorialAscendant - 100.121547)) < 1e-4);
    assert.ok(Math.abs(wrap(r.kochCoAscendant - 79.514664)) < 1e-4);
    assert.ok(Math.abs(wrap(r.munkaseyCoAscendant - 122.217004)) < 1e-4);
    assert.ok(Math.abs(wrap(r.munkaseyPolarAscendant - 259.514664)) < 1e-4);
    assert.ok(Math.abs(wrap(r.house[0] - (h === 'V' ? r.ascendant - 15 : r.ascendant))) < 1e-9, `cusp 1 of ${h}`);
  }
});

test('the Vehlow system puts the Ascendant in the middle of house 1', needs, async () => {
  const facade = createSwissShapedFacade(new Engine(de441), { houseMethods: await load() });
  const r = facade.swe_houses(JD, 41.9, 12.5, 'V');
  assert.ok(Math.abs(wrap(r.house[0] - (r.ascendant - 15))) < 1e-9);
  for (let i = 1; i < 12; i += 1) assert.ok(Math.abs(wrap(r.house[i] - r.house[i - 1]) - 30) < 1e-9);
});

test('polar latitudes return values; provenance adds warnings, never default keys', needs, async () => {
  const plain = createSwissShapedFacade(new Engine(de441), { houseMethods: await load() });
  const loud = createSwissShapedFacade(new Engine(de441), { houseMethods: await load(), provenance: true });
  const r = plain.swe_houses(JD, 69.65, 18.96, 'P');
  assert.deepEqual(Object.keys(r), KEYS, 'the backend loops over every key: no extra ones by default');
  assert.equal(r.house.length, 12);
  assert.equal(r.status, 'FALLBACK_PORPHYRY', 'fallback remains available as non-enumerable metadata');
  assert.ok(r.warnings.some((x) => x.code === 'SYSTEM_FALLBACK_PORPHYRY'));
  assert.equal(Object.prototype.propertyIsEnumerable.call(r, 'warnings'), false);
  assert.equal(Object.prototype.propertyIsEnumerable.call(r, 'status'), false);
  assert.equal(Object.prototype.propertyIsEnumerable.call(r, 'timeSource'), false);
  assert.equal(Object.hasOwn(JSON.parse(JSON.stringify(r)), 'warnings'), false);
  const w = loud.swe_houses(JD, 69.65, 18.96, 'P');
  assert.equal(w.status, 'FALLBACK_PORPHYRY');
  assert.ok(w.warnings.some((x) => x.code === 'SYSTEM_FALLBACK_PORPHYRY'));
  assert.ok(w.warnings.some((x) => x.code === 'POLAR_CIRCLE_LATITUDE'));
  assert.deepEqual(w.house, r.house);
});

test('unsupported systems, bad input and missing methods are errors, never silent Placidus', needs, async () => {
  const facade = createSwissShapedFacade(new Engine(de441), { houseMethods: await load() });
  assert.equal(facade.swe_houses(JD, 41.9, 12.5, 'Z').code, 'E_UNSUPPORTED_FLAG');
  assert.equal(facade.swe_houses(JD, 41.9, 12.5, 'E').code, 'E_UNSUPPORTED_FLAG');
  assert.equal(facade.swe_houses(JD, 91, 12.5, 'P').code, 'E_RANGE');
  assert.equal(facade.swe_houses(JD, NaN, 12.5, 'P').code, 'E_RANGE');
  assert.equal(facade.swe_houses_pos(100, 41.9, 23.44, 'K', 120, 1).code, 'E_UNSUPPORTED_FLAG');
});

test('swe_houses_pos (Placidus) is a house position between 1 and 13', needs, async () => {
  const facade = createSwissShapedFacade(new Engine(de441), { houseMethods: await load() });
  const r = facade.swe_houses_pos(100, 41.9, 23.44, 'P', 120, 1);
  assert.deepEqual(Object.keys(r), ['housePosition']);
  assert.ok(Math.abs(r.housePosition - 10.6095501398) < 1e-6, String(r.housePosition));
});

test('a circumpolar Placidus position is a specific error rather than a fabricated number', () => {
  const facade = createSwissShapedFacade(null, { houseMethods: {
    calculatePlacidusHousePosition() {
      const error = new Error('No distinct rising and setting semi-arcs');
      error.code = 'CIRCUMPOLAR_POSITION_UNAVAILABLE';
      throw error;
    },
  } });
  const result = facade.swe_houses_pos(100, 69.65, 23.44, 'P', 120, 1);
  assert.equal(result.code, 'E_POSITION_UNAVAILABLE');
  assert.equal(result.housePosition, undefined);
});

test('the real house method reports an unavailable circumpolar position explicitly', needs, async () => {
  const facade = createSwissShapedFacade(new Engine(de441), { houseMethods: await load() });
  const result = facade.swe_houses_pos(100, 69.65, 23.44, 'P', 90, 0);
  assert.equal(result.code, 'E_POSITION_UNAVAILABLE');
  assert.equal(result.housePosition, undefined);
});

test('invalid injected house positions fail explicitly', () => {
  for (const value of [undefined, NaN, Infinity, 0, 13]) {
    const facade = createSwissShapedFacade(null, { houseMethods: { calculatePlacidusHousePosition: () => ({ housePosition: value }) } });
    assert.equal(facade.swe_houses_pos(100, 30, 23.44, 'P', 90, 0).code, 'E_CALC');
  }
});

test('agreement with the installed swisseph: cusps, angles and extra points', needsReference, async () => {
  const swiss = require(swissPath);
  swiss.swe_set_ephe_path(dirname(de441));
  swiss.swe_set_jpl_file(basename(de441));
  const ours = createSwissShapedFacade(new Engine(de441), { houseMethods: await load() });
  for (const [lat, lon] of [[41.9, 12.5], [-33.9, 151.2], [64.1, -21.9], [-0.2, -78.5], [10, 0], [-10, 100], [55, 8]]) {
    for (const h of ['V', 'P', 'K', 'R', 'O']) {
      const a = swiss.swe_houses(JD, lat, lon, h);
      const b = ours.swe_houses(JD, lat, lon, h);
      assert.deepEqual(Object.keys(b), Object.keys(a));
      for (const key of Object.keys(a)) {
        if (key === 'house') {
          a.house.forEach((v, i) => assert.ok(Math.abs(wrap(b.house[i] - v)) * 3600 < 0.1, `${h} cusp ${i + 1} at ${lat}`));
        } else {
          assert.ok(Math.abs(wrap(b[key] - a[key])) * 3600 < 0.1, `${h} ${key} at ${lat}`);
        }
      }
    }
  }
  // Placidus house position
  const pa = swiss.swe_houses_pos(100, 41.9, 23.44, 'P', 120, 1);
  const pb = ours.swe_houses_pos(100, 41.9, 23.44, 'P', 120, 1);
  assert.deepEqual(Object.keys(pb), Object.keys(pa));
  assert.ok(Math.abs(pb.housePosition - pa.housePosition) < 1e-7);
  // Where the reference refuses (polar Placidus/Koch) this engine returns values.
  assert.ok(swiss.swe_houses(JD, 69.65, 18.96, 'P').error);
  assert.equal(ours.swe_houses(JD, 69.65, 18.96, 'P').house.length, 12);
});
