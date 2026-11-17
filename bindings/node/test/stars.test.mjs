// Star tests need DE441_PATH and a caller-supplied genuine hip2.dat (HIP2_PATH).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { join } from 'node:path';

const require = createRequire(import.meta.url);
const { Engine, constants, createSwissShapedFacade, inspectHipparcosQuality } = require('../index.js');
const de441 = process.env.DE441_PATH;
const hip2 = process.env.HIP2_PATH;
const opts = { skip: de441 && hip2 ? false : 'DE441_PATH and HIP2_PATH are required' };

test('HIP2 quality inspection reports numerical eligibility without changing an engine', { skip: hip2 ? false : 'HIP2_PATH is required' }, () => {
  const rows = inspectHipparcosQuality(hip2, [49669, 21421]);
  assert.deepEqual(rows.map((r) => r.hip), [49669, 21421]);
  assert.equal(rows[0].strictEligible, true);
  assert.equal(rows[1].strictEligible, false);
  assert.equal(rows[1].traditionalEligible, true);
  assert.throws(() => inspectHipparcosQuality(hip2, [49669, 49669]), (e) => e.code === 'E_RANGE');
  assert.throws(() => inspectHipparcosQuality(hip2, [-1]), (e) => e.code === 'E_RANGE');
  assert.throws(() => inspectHipparcosQuality(hip2, [0xffffffff]), (e) => e.code === 'E_CATALOG_FILE');
});

test('traditional stars load under their policy; strict rejects a stochastic solution', opts, () => {
  const e = new Engine(de441);
  assert.equal(e.loadHipparcos(hip2, [49669, 21421], 'traditional'), 2);
  assert.throws(() => e.loadHipparcos(hip2, [21421], 'strict'), (x) => x.code === 'E_STAR_QUALITY');
  assert.equal(e.fixstarTdb(49669, 5.9, 2451545.0, constants.SEFLG_SPEED).xx[0] > 149.8, true);
  assert.throws(() => e.loadHipparcos(hip2, [49669], 'bogus'), (x) => x.code === 'E_RANGE');
  assert.throws(() => e.fixstarTdb(1, null, 2451545.0, 0), (x) => x.code === 'E_STAR_NOT_LOADED');
});

test('a failed load keeps the previous selection, and a missing radial velocity is reported', opts, () => {
  const e = new Engine(de441);
  e.loadHipparcos(hip2, [49669], 'strict');
  assert.throws(() => e.loadHipparcos(hip2, [21421], 'strict'));
  const r = e.fixstarTdb(49669, null, 2451545.0, constants.SEFLG_SPEED);
  assert.ok(r.notes.some((n) => n.includes('assumed to be 0 km/s')));
  assert.ok(Math.abs(r.xx[0] - 149.829) < 0.001);
});

test('fixed-star UT conversion uses the same time policy as planetary calculations', opts, () => {
  const e = new Engine(de441);
  e.loadHipparcos(hip2, [49669], 'strict');
  for (const jd of [2433417.5, 2451545.0]) {
    const star = e.fixstarUt(49669, null, jd, constants.SEFLG_SPEED);
    assert.equal(star.timeSource, e.calcUt(jd, 0, constants.SEFLG_SPEED).timeSource);
    assert.ok(star.notes.some((n) => n.includes('assumed to be 0 km/s')));
    assert.ok(Number.isFinite(star.xx[0]));
  }
  assert.throws(() => e.fixstarUt(49669, null, 3000000, 0), (x) => x.code === 'E_RANGE');
});

test('fixed-star UT honors a loaded measured Delta T table', { skip: de441 && hip2 && process.env.USNO_DELTAT_DIR ? false : 'DE441_PATH, HIP2_PATH and USNO_DELTAT_DIR are required' }, () => {
  const e = new Engine(de441);
  const dir = process.env.USNO_DELTAT_DIR;
  e.loadHipparcos(hip2, [49669], 'strict');
  e.loadDeltaT(join(dir, 'deltat.data'), join(dir, 'historic_deltat.data'), join(dir, 'deltat.preds'));
  const star = e.fixstarUt(49669, null, 2451545.0, constants.SEFLG_SPEED);
  assert.match(star.timeSource, /^DELTA_T_TABLE:MEASURED:/);
  assert.equal(star.timeSource, e.calcUt(2451545.0, 0, constants.SEFLG_SPEED).timeSource);
});

test('star facade uses only caller-supplied metadata and preserves callbacks and errors', () => {
  const calls = [];
  const engine = { fixstarUt: (...args) => { calls.push(args); return { xx: [1, 2, 3, 4, 5, 6], rflag: 1, timeSource: 'test', notes: [] }; } };
  const entry = { name: 'Example', aliases: ['Alpha'], hip: 123, radialVelocityKmS: null, magnitude: 1.2 };
  const facade = createSwissShapedFacade(engine, { starCatalog: [entry], provenance: true });
  let callback;
  const result = facade.swe_fixstar_ut(' alpha ', 2451545, constants.SEFLG_SPEED, (value) => { callback = value; });
  assert.deepEqual(calls, [[123, null, 2451545, constants.SEFLG_SPEED]]);
  assert.equal(result.name, 'Example');
  assert.equal(result.longitude, 1);
  assert.equal(result.timeSource, 'test');
  assert.equal(callback, result);
  assert.deepEqual(facade.swe_fixstar_mag('EXAMPLE'), { name: 'Example', magnitude: 1.2 });
  assert.equal(facade.swe_fixstar_ut('missing', 2451545, 0).code, 'E_STAR_NOT_FOUND');
  assert.equal(facade.swe_fixstar_ut('Example', 2451545, constants.SEFLG_SWIEPH).code, 'E_UNSUPPORTED_FLAG');
  const missingMag = createSwissShapedFacade(engine, { starCatalog: [{ ...entry, magnitude: null }] });
  assert.equal(missingMag.swe_fixstar_mag('Example').code, 'E_STAR_MAGNITUDE');
  assert.throws(() => createSwissShapedFacade(engine, { starCatalog: [{ ...entry, radialVelocityKmS: undefined }] }), (x) => x.code === 'E_CATALOG');
  assert.throws(() => createSwissShapedFacade(engine, { starCatalog: [entry, { ...entry, name: 'Other' }] }), (x) => x.code === 'E_CATALOG');
  entry.magnitude = 99;
  assert.equal(facade.swe_fixstar_mag('Example').magnitude, 1.2, 'registry is a snapshot');
});
