// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { Engine, julday, constants } = require('../index.js');
const kernel = process.env.DE441_PATH;

test('experimental lunar rates remain consistent with positions over the long-range grid',
  { skip: kernel ? false : 'DE441_PATH required' }, () => {
    const engine = new Engine(kernel), h = 0.001;
    const signed = x => ((x % 360) + 540) % 360 - 180;
    for (let year = 0; year <= 5000; year += 250) {
      const jd = julday(year, 7, 1, 12, true), span = (jd+h)-(jd-h);
      const before = engine.experimentalLongTermApparentTdb(jd-h, constants.SE_MOON).xx;
      const middle = engine.experimentalLongTermApparentTdb(jd, constants.SE_MOON).xx;
      const after = engine.experimentalLongTermApparentTdb(jd+h, constants.SE_MOON).xx;
      assert.ok(Math.abs(middle[3]-signed(after[0]-before[0])/span)*3600 < 0.003, `longitude ${year}`);
      assert.ok(Math.abs(middle[4]-(after[1]-before[1])/span)*3600 < 0.003, `latitude ${year}`);
    }
  });

test('experimental apparent ICRF direction and derivative are explicit over the requested range',
  { skip: kernel ? false : 'DE441_PATH required' }, () => {
    const engine = new Engine(kernel);
    for (const year of [0, 1000, 2000, 3000, 4000, 5000]) {
      const jd = julday(year, 7, 1, 12, true);
      for (let body = 0; body <= 9; body++) {
        const state = engine.experimentalApparentIcrfTdb(jd, body);
        assert.equal(state.frame, 'ICRF'); assert.equal(state.center, 'Earth');
        assert.match(state.notes.join(' '), /light-time.*deflection.*aberration/);
        assert.ok([...state.positionKm, ...state.velocityKmPerDay].every(Number.isFinite));
        if (body >= 4) assert.match(state.target, /Barycenter$/);
      }
      const h = 0.001;
      const mid = engine.experimentalApparentIcrfTdb(jd, constants.SE_MARS);
      const left = engine.experimentalApparentIcrfTdb(jd - h, constants.SE_MARS);
      const right = engine.experimentalApparentIcrfTdb(jd + h, constants.SE_MARS);
      for (let i = 0; i < 3; i++) {
        const finite = (right.positionKm[i] - left.positionKm[i]) / (2*h);
        assert.ok(Math.abs(finite - mid.velocityKmPerDay[i]) < 10);
      }
    }
    assert.throws(() => engine.experimentalApparentIcrfTdb(NaN, 0), e => e.code === 'E_RANGE');
    assert.throws(() => engine.experimentalApparentIcrfTdb(2451545, 15), e => e.code === 'E_UNSUPPORTED_BODY');
    assert.throws(() => engine.experimentalApparentIcrfTdb(1e8, 0), e => e.code === 'E_CALC');
  });

test('geometric Earth-centered ICRF states span year 0 through 5000 in the supplied file',
  { skip: kernel ? false : 'DE441_PATH required' }, () => {
    const engine = new Engine(kernel);
    for (const year of [0, 1000, 2000, 3000, 4000, 5000]) {
      const jd = julday(year, 7, 1, 12, true);
      for (let body = 0; body <= 9; body++) {
        const r = engine.geometricStateTdb(jd, body);
        assert.equal(r.jdTdb, jd); assert.equal(r.center, 'Earth'); assert.equal(r.frame, 'ICRF');
        assert.equal(r.positionKm.length, 3); assert.equal(r.velocityKmPerDay.length, 3);
        assert.ok([...r.positionKm, ...r.velocityKmPerDay].every(Number.isFinite));
        if (body >= 4) assert.match(r.target, /Barycenter$/);
      }
    }
    assert.throws(() => engine.geometricStateTdb(NaN, constants.SE_MARS), e => e.code === 'E_RANGE');
    assert.throws(() => engine.geometricStateTdb(2451545, constants.SE_CHIRON), e => e.code === 'E_UNSUPPORTED_BODY');
    assert.throws(() => engine.geometricStateTdb(1e8, constants.SE_MARS), e => e.code === 'E_CALC');
    assert.throws(() => engine.calcTdb(3547454, constants.SE_MARS), e => e.code === 'E_CALC');
  });

test('experimental extrapolated apparent path covers the requested sample and keeps the standard range guard',
  { skip: kernel ? false : 'DE441_PATH required' }, () => {
    const engine = new Engine(kernel);
    const signed = d => ((d + 540) % 360) - 180;
    for (const year of [0, 1000, 2000, 3000, 4000, 5000]) {
      const jd = julday(year, 7, 1, 12, true);
      for (let body = 0; body <= 9; body++) {
        const result = engine.experimentalLongTermApparentTdb(jd, body);
        assert.equal(result.xx.length, 6); assert.ok(result.xx.every(Number.isFinite));
        assert.match(result.notes.join(' '), /extrapolated nutation/);
      }
      const h = 0.001;
      const before = engine.experimentalLongTermApparentTdb(jd - h, constants.SE_MARS).xx;
      const middle = engine.experimentalLongTermApparentTdb(jd, constants.SE_MARS).xx;
      const after = engine.experimentalLongTermApparentTdb(jd + h, constants.SE_MARS).xx;
      assert.ok(Math.abs(signed(after[0] - before[0]) / (2 * h) - middle[3]) < 0.0001);
    }
    const date = julday(2000, 7, 1, 12, true);
    const candidate = engine.experimentalLongTermApparentTdb(date, constants.SE_MARS).xx;
    const existing = engine.calcTdb(date, constants.SE_MARS, constants.SEFLG_SPEED).xx;
    assert.ok(Math.abs(signed(candidate[0] - existing[0])) < 0.000001);
    assert.throws(() => engine.calcTdb(julday(5000, 7, 1, 12, true), constants.SE_MARS), e => e.code === 'E_CALC');
  });

test('experimental corrected mean-ecliptic direction works over the requested range without changing calcTdb',
  { skip: kernel ? false : 'DE441_PATH required' }, () => {
    const engine = new Engine(kernel);
    const signed = d => ((d + 540) % 360) - 180;
    for (const year of [0, 1000, 2000, 3000, 4000, 5000]) {
      const jd = julday(year, 7, 1, 12, true);
      for (let body = 0; body <= 9; body++) {
        const result = engine.experimentalLongTermMeanEclipticTdb(jd, body);
        assert.equal(result.xx.length, 6); assert.ok(result.xx.every(Number.isFinite));
        assert.equal(result.timeSource, 'TDB_INPUT'); assert.match(result.notes.join(' '), /nutation omitted/);
      }
      const h = 0.001;
      const center = engine.experimentalLongTermMeanEclipticTdb(jd, constants.SE_MARS).xx;
      const before = engine.experimentalLongTermMeanEclipticTdb(jd - h, constants.SE_MARS).xx;
      const after = engine.experimentalLongTermMeanEclipticTdb(jd + h, constants.SE_MARS).xx;
      assert.ok(Math.abs(signed(after[0] - before[0]) / (2 * h) - center[3]) < 0.0001);
      assert.ok(Math.abs((after[1] - before[1]) / (2 * h) - center[4]) < 0.0001);
    }
    const date = julday(2000, 7, 1, 12, true);
    const mean = engine.experimentalLongTermMeanEclipticTdb(date, constants.SE_MARS).xx;
    const actual = engine.calcTdb(date, constants.SE_MARS, constants.SEFLG_SPEED).xx;
    assert.ok(Math.abs(signed(mean[0] - actual[0])) < 0.01);
    assert.throws(() => engine.calcTdb(julday(5000, 7, 1, 12, true), constants.SE_MARS), e => e.code === 'E_CALC');
  });
