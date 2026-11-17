// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { Engine, julday, constants, experimentalEspenakMeeus2006DeltaT } = require('../index.js');
const kernel = process.env.DE441_PATH;

test('Uranus at the 1100 regression epoch is finite in both experimental frames',
  { skip: kernel ? false : 'DE441_PATH required' }, () => {
    const engine = new Engine(kernel), jd = julday(1100, 7, 1, 12, true);
    const icrf = engine.experimentalApparentIcrfTdb(jd, constants.SE_URANUS);
    const ecliptic = engine.experimentalLongTermApparentTdb(jd, constants.SE_URANUS);
    assert.equal(icrf.positionKm.length, 3);
    assert.ok(icrf.positionKm.every(Number.isFinite));
    assert.equal(ecliptic.xx.length, 6);
    assert.ok(ecliptic.xx.every(Number.isFinite));
  });

test('explicit UT1 Delta T and uncertainty flow to the same experimental TDB calculation',
  { skip: kernel ? false : 'DE441_PATH required' }, () => {
    const engine = new Engine(kernel);
    for (const year of [-12999, -10000, -5000, 0, 1000, 2000, 3000, 4000, 5000]) {
      const jd = julday(year, 7, 1, 12, true);
      const result = engine.experimentalLongTermApparentUt1(jd, constants.SE_MARS, 123.5, 4.2, ' example estimate ');
      assert.equal(result.jdUt1, jd); assert.equal(result.deltaTSeconds, 123.5);
      assert.equal(result.deltaTUncertaintySeconds, 4.2); assert.equal(result.deltaTSource, 'example estimate');
      assert.ok(Math.abs((result.jdTt - jd) * 86400 - 123.5) < 0.0001);
      assert.ok(Math.abs((result.jdTdb - result.jdTt) * 86400) < 0.002);
      assert.deepEqual(result.xx, engine.experimentalLongTermApparentTdb(result.jdTdb, constants.SE_MARS).xx);
      assert.match(result.notes.join(' '), /not UTC/);
      const direct = engine.experimentalLongTermDirectObliquityUt1(jd, constants.SE_MARS, 123.5, 4.2, ' example estimate ');
      assert.equal(direct.deltaTSeconds, 123.5);assert.equal(direct.deltaTUncertaintySeconds,4.2);
      assert.equal(direct.deltaTSource,'example estimate');assert.equal(direct.jdTdb,result.jdTdb);
      assert.deepEqual(direct.xx,engine.experimentalLongTermDirectObliquityTdb(direct.jdTdb,constants.SE_MARS).xx);
      assert.match(direct.notes.join(' '),/MIXED FRAME.*accuracy not established/);
    }
    const jd = julday(2000, 1, 1, 12, true);
    const result = engine.experimentalLongTermApparentUt1(jd, constants.SE_MARS, 64.184, 1, 'UT1 approximately UTC').xx;
    const existing = engine.calcUt(jd, constants.SE_MARS, constants.SEFLG_SPEED).xx;
    const wrap = d => ((d + 540) % 360) - 180;
    assert.ok(Math.abs(wrap(result[0] - existing[0])) < 0.000001);
    for (const [dt, uncertainty, source] of [[NaN, 1, 'x'], [1, -1, 'x'], [1, Infinity, 'x'], [1, 1, ' ']]) {
      assert.throws(() => engine.experimentalLongTermApparentUt1(jd, constants.SE_MARS, dt, uncertainty, source), e => e.code === 'E_RANGE');
      assert.throws(() => engine.experimentalLongTermDirectObliquityUt1(jd, constants.SE_MARS, dt, uncertainty, source), e => e.code === 'E_RANGE');
    }
    assert.throws(() => engine.experimentalLongTermApparentUt1(jd, constants.SE_CHIRON, 1, 1, 'x'), e => e.code === 'E_UNSUPPORTED_BODY');
  });

test('historical NASA polynomial is opt-in, labeled and rejects extrapolation past 3000', () => {
  for (const [year, seconds] of [[0, 10583.6], [1000, 1574.2], [2000, 63.86], [3000, 4435.68]]) {
    const r = experimentalEspenakMeeus2006DeltaT(year);
    assert.ok(Math.abs(r.seconds - seconds) < 1e-8);
    assert.equal(r.model, 'ESPENAK_MEEUS_2006_UNCORRECTED');
    assert.match(r.notes.join(' '), /uncertainty not supplied/);
  }
  assert.throws(() => experimentalEspenakMeeus2006DeltaT(3001), e => e.code === 'E_RANGE');
  assert.throws(() => experimentalEspenakMeeus2006DeltaT(-1), e => e.code === 'E_RANGE');
});

test('Delta T perturbation changes longitude consistently with the reported TDB rate',
  { skip: kernel ? false : 'DE441_PATH required' }, () => {
    const engine = new Engine(kernel);
    const wrap = d => ((d + 540) % 360) - 180;
    for (const year of [-12999,-10000,-5000,0,1000,2000,3000,4000,5000]) {
      const jd = julday(year,7,1,12,true);
      for (const body of [constants.SE_MOON, constants.SE_MARS, constants.SE_SATURN]) {
        const center = engine.experimentalLongTermApparentUt1(jd,body,1000,60,'synthetic sensitivity test');
        const before = engine.experimentalLongTermApparentUt1(jd,body,940,60,'synthetic sensitivity test');
        const after = engine.experimentalLongTermApparentUt1(jd,body,1060,60,'synthetic sensitivity test');
        const observed = wrap(after.xx[0]-before.xx[0]);
        const expected = center.xx[3] * (after.jdTdb-before.jdTdb);
        assert.ok(Math.abs(observed-expected)<0.00002);
      }
    }
  });
