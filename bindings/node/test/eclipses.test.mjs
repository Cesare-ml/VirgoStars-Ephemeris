// Global solar and lunar eclipses: swe_sol_eclipse_when_glob and swe_lun_eclipse_when.
//
// Kernel tests skip without DE441_PATH; the comparison with an installed `swisseph`
// package is LOCAL USE ONLY (AGPL, never a dependency) and skips without SWISSEPH_PATH.
// Nothing the reference returns is stored; only agreement is asserted.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, basename } from 'node:path';

const require = createRequire(import.meta.url);
const { Engine, constants: C, createSwissShapedFacade } = require('../index.js');
const de441 = process.env.DE441_PATH;
const swissPath = process.env.SWISSEPH_PATH;
const needsKernel = { skip: de441 ? false : 'DE441_PATH is not set' };
const needsReference = { skip: de441 && swissPath ? false : 'DE441_PATH and SWISSEPH_PATH are required' };

const SOLAR_KEYS = ['rflag', 'maximum', 'noon', 'begin', 'end', 'totalBegin', 'totalEnd', 'centerBegin', 'centerEnd'];
const LUNAR_KEYS = ['rflag', 'maximum', 'partialBegin', 'partialEnd', 'totalBegin', 'totalEnd', 'penumbralBegin', 'penumbralEnd'];
const jd = (facade, y, m, d) => facade.swe_julday(y, m, d, 0, C.SE_GREG_CAL);
// Seconds past midnight UT of a Julian day number's fraction.
const secondsOfDay = (t) => (((t - 0.5) % 1) + 1) % 1 * 86400;

test('the eclipse constants are the reference values', () => {
  assert.equal(C.SE_ECL_TOTAL, 4);
  assert.equal(C.SE_ECL_ANNULAR_TOTAL, 32);
  assert.equal(C.SE_ECL_PENUMBRAL, 64);
  assert.equal(C.SE_ECL_ALLTYPES_LUNAR, 84);
});

test('well-known solar eclipses: kind and greatest-eclipse time', needsKernel, () => {
  const f = createSwissShapedFacade(new Engine(de441));
  // Published times of greatest eclipse (UT): 2024-04-08 18:17:16 total, 2023-10-14 17:59:32 annular,
  // 2023-04-20 04:16:44 hybrid. Agreement within a minute: the instants depend on DeltaT and the Moon's limb.
  const cases = [
    [jd(f, 2024, 4, 1), 5, 18 * 3600 + 17 * 60 + 16],
    [jd(f, 2023, 10, 1), 9, 17 * 3600 + 59 * 60 + 32],
    [jd(f, 2023, 4, 1), 33, 4 * 3600 + 16 * 60 + 44],
  ];
  for (const [start, flag, seconds] of cases) {
    const r = f.swe_sol_eclipse_when_glob(start, C.SEFLG_JPLEPH, 0, 0);
    assert.deepEqual(Object.keys(r), SOLAR_KEYS);
    assert.equal(r.rflag, flag);
    assert.ok(Math.abs(secondsOfDay(r.maximum) - seconds) < 60, `maximum ${secondsOfDay(r.maximum)} vs ${seconds}`);
    // contacts nest: partial phase > umbral phase >= central phase
    // (a few seconds of slack: on a hybrid the umbral and central contacts nearly coincide)
    const slack = 5 / 86400;
    assert.ok(r.begin < r.totalBegin && r.totalBegin <= r.centerBegin + slack && r.centerBegin < r.maximum);
    assert.ok(r.maximum < r.centerEnd && r.centerEnd <= r.totalEnd + slack && r.totalEnd < r.end);
  }
});

test('well-known lunar eclipses: kind, contacts and ordering', needsKernel, () => {
  const f = createSwissShapedFacade(new Engine(de441));
  const cases = [
    [jd(f, 2025, 3, 1), 4, 6 * 3600 + 58 * 60 + 46], // total, 2025-03-14
    [jd(f, 2024, 3, 1), 64, 7 * 3600 + 12 * 60 + 45], // penumbral, 2024-03-25
    [jd(f, 2022, 11, 1), 4, 10 * 3600 + 59 * 60 + 11], // total, 2022-11-08
  ];
  for (const [start, flag, seconds] of cases) {
    const r = f.swe_lun_eclipse_when(start, C.SEFLG_JPLEPH, 0, 0);
    assert.deepEqual(Object.keys(r), LUNAR_KEYS);
    assert.equal(r.rflag, flag);
    assert.ok(Math.abs(secondsOfDay(r.maximum) - seconds) < 60);
    assert.ok(r.penumbralBegin < r.maximum && r.maximum < r.penumbralEnd);
    if (flag === 4) {
      assert.ok(r.penumbralBegin < r.partialBegin && r.partialBegin < r.totalBegin && r.totalBegin < r.maximum);
      assert.ok(r.maximum < r.totalEnd && r.totalEnd < r.partialEnd && r.partialEnd < r.penumbralEnd);
    } else {
      assert.equal(r.partialBegin, 0);
      assert.equal(r.totalBegin, 0);
    }
  }
});

test('type filters, direction and the result callback behave like the reference', needsKernel, () => {
  const f = createSwissShapedFacade(new Engine(de441), { acceptSwiephFlag: true });
  const start = jd(f, 2020, 1, 1);
  // First total solar eclipse after 2020: the one of 2020-12-14.
  const total = f.swe_sol_eclipse_when_glob(start, C.SEFLG_SWIEPH, C.SE_ECL_TOTAL, 0);
  assert.ok(total.rflag & C.SE_ECL_TOTAL);
  assert.ok(Math.abs(total.maximum - jd(f, 2020, 12, 14)) < 1);
  // The eclipse before 2020-01-01 is the annular one of 2019-12-26 (backward search).
  const before = f.swe_sol_eclipse_when_glob(start, C.SEFLG_JPLEPH, 0, 1);
  assert.ok(Math.abs(before.maximum - jd(f, 2019, 12, 26)) < 1 && before.maximum < start);
  // A lunar penumbral-only filter skips partial and total ones.
  const penumbral = f.swe_lun_eclipse_when(start, C.SEFLG_JPLEPH, C.SE_ECL_PENUMBRAL, 0);
  assert.equal(penumbral.rflag, C.SE_ECL_PENUMBRAL);
  // A trailing callback is invoked synchronously and the result is also returned.
  let seen;
  const returned = f.swe_lun_eclipse_when(start, C.SEFLG_JPLEPH, 0, 0, (r) => { seen = r; });
  assert.deepEqual(seen, returned);
});

test('errors are objects, never silent: bad flags and dates outside the supported range', needsKernel, () => {
  const f = createSwissShapedFacade(new Engine(de441));
  assert.equal(f.swe_sol_eclipse_when_glob(jd(f, 2020, 1, 1), C.SEFLG_MOSEPH, 0, 0).code, 'E_UNSUPPORTED_FLAG');
  assert.equal(f.swe_lun_eclipse_when(jd(f, 2020, 1, 1), C.SEFLG_SWIEPH, 0, 0).code, 'E_UNSUPPORTED_FLAG', 'SWIEPH needs acceptSwiephFlag');
  assert.equal(f.swe_sol_eclipse_when_glob(jd(f, 2030, 1, 1), C.SEFLG_JPLEPH, 0, 0).code, 'E_RANGE');
  assert.equal(f.swe_lun_eclipse_when(jd(f, 1800, 1, 1), C.SEFLG_JPLEPH, 0, 0).code, 'E_RANGE');
});

test('agreement with the installed swisseph over 1990-2026', needsReference, () => {
  const swiss = require(swissPath);
  swiss.swe_set_ephe_path(dirname(de441));
  swiss.swe_set_jpl_file(basename(de441));
  const ours = createSwissShapedFacade(new Engine(de441));
  const end = swiss.swe_julday(2026, 1, 1, 0, 1);
  for (const [fn, keys, contactTolerance] of [
    ['swe_lun_eclipse_when', LUNAR_KEYS, 60],
    ['swe_sol_eclipse_when_glob', SOLAR_KEYS, 240],
  ]) {
    let t = swiss.swe_julday(1990, 3, 1, 0, 1);
    let count = 0;
    while (t < end) {
      const a = swiss[fn](t, 1, 0, 0);
      const b = ours[fn](t, 1, 0, 0);
      assert.deepEqual(Object.keys(b), Object.keys(a));
      assert.equal(b.rflag, a.rflag, `${fn} kind at ${a.maximum}`);
      assert.ok(Math.abs(b.maximum - a.maximum) * 86400 < 15, `${fn} maximum at ${a.maximum}`);
      for (const key of keys.slice(2)) {
        if (a[key] && b[key] && a.begin !== 0 && key !== 'noon') {
          assert.ok(Math.abs(b[key] - a[key]) * 86400 < contactTolerance, `${fn} ${key} at ${a.maximum}`);
        }
      }
      count += 1;
      t = a.maximum + 20;
    }
    assert.ok(count > 60, `${fn}: ${count} eclipses compared`);
  }
});
