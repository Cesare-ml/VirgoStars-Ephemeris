// Offline contract tests: they need the built addon but no ephemeris file.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const require = createRequire(import.meta.url);
const { Engine, julday, revjul, constants, createSwissShapedFacade } = require('../index.js');

test('julday matches the published J2000 and calendar anchors', () => {
  assert.equal(julday(2000, 1, 1, 12, true), 2451545.0);
  assert.equal(julday(1858, 11, 17, 0, true), 2400000.5);
  // The proleptic calendars are explicit: no 1582 changeover is applied.
  assert.equal(julday(1582, 10, 15, 0, true), 2299160.5);
  assert.equal(julday(1582, 10, 5, 0, false), 2299160.5);
  assert.equal(julday(-4712, 1, 1, 12, false), 0);
});

test('revjul inverts julday on a grid, including fractional hours and both calendars', () => {
  for (const gregorian of [true, false]) {
    for (let jd = 2299160.5; jd < 2488069.5; jd += 4093.317) {
      const r = revjul(jd, gregorian);
      const back = julday(r.year, r.month, r.day, r.hour, gregorian);
      assert.ok(Math.abs(back - jd) < 1e-8, `${jd} ${gregorian}`);
      assert.ok(r.hour >= 0 && r.hour < 24);
    }
  }
  assert.deepEqual(revjul(2451545.0, true), { year: 2000, month: 1, day: 1, hour: 12 });
});

test('invalid calendar input is an error with a stable code, never a silent value', () => {
  const bad = [
    [2001, 2, 30, 0, true],
    [2001, 13, 1, 0, true],
    [2001, 1, 1, 24, true],
    [2001, 1, 1, -1, true],
    [2001, 1, 1, NaN, true],
  ];
  for (const args of bad) {
    assert.throws(() => julday(...args), (e) => e.code === 'E_RANGE', JSON.stringify(args));
  }
  assert.throws(() => revjul(NaN, true), (e) => e.code === 'E_RANGE');
});

test('an engine requires a real DE441 file: missing, wrong format and empty paths are rejected', () => {
  assert.throws(() => new Engine(''), (e) => e.code === 'E_RANGE');
  assert.throws(
    () => new Engine(join(tmpdir(), 'definitely-missing-de441.eph')),
    (e) => e.code === 'E_EPHEMERIS_FILE',
  );
  const dir = mkdtempSync(join(tmpdir(), 'vse-'));
  try {
    const fake = join(dir, 'not-an-ephemeris.bsp');
    writeFileSync(fake, Buffer.alloc(4096, 0x20));
    assert.throws(() => new Engine(fake), (e) => e.code === 'E_EPHEMERIS_FILE');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('constants carry the documented public values and are frozen', () => {
  assert.equal(constants.SE_SUN, 0);
  assert.equal(constants.SE_PLUTO, 9);
  assert.equal(constants.SEFLG_SPEED, 256);
  assert.equal(constants.SE_GREG_CAL, 1);
  assert.ok(Object.isFrozen(constants));
});

test('the facade julday/revjul use the Swiss calendar flag values', () => {
  const swe = createSwissShapedFacade(null);
  assert.equal(swe.swe_julday(2000, 1, 1, 12, swe.SE_GREG_CAL), 2451545.0);
  assert.equal(swe.swe_julday(1582, 10, 5, 0, swe.SE_JUL_CAL), 2299160.5);
  assert.deepEqual(swe.swe_revjul(2451545.0, swe.SE_GREG_CAL), {
    year: 2000,
    month: 1,
    day: 1,
    hour: 12,
  });
});
