// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const { Engine, julday, constants } = createRequire(import.meta.url)('../index.js');
const kernel = process.env.DE441_PATH;
const opts = { skip: kernel ? false : 'DE441_PATH required' };

test('three fixed-UT1 scenarios reproduce explicit single calculations in both frames', opts, () => {
  const engine = new Engine(kernel);
  for (const year of [-12999, -5000, 0, 2000, 3999]) {
    const jd = julday(year, 7, 1, 12, true);
    for (const [method, single, frame] of [
      ['experimentalLongTermApparentUt1Scenarios', 'experimentalLongTermApparentUt1', 'POLE_BASED'],
      ['experimentalLongTermDirectObliquityUt1Scenarios', 'experimentalLongTermDirectObliquityUt1', 'MIXED_DIRECT_OBLIQUITY'],
    ]) {
      for (const body of [constants.SE_MOON, constants.SE_MARS]) {
        const result = engine[method](jd, body, 1234, 60, ' test source ');
        assert.equal(result.jdUt1, jd); assert.equal(result.frame, frame);
        assert.equal(result.deltaTSource, 'test source');
        assert.equal(result.deltaTSeconds, 1234); assert.equal(result.deltaTUncertaintySeconds, 60);
        assert.match(result.notes.join(' '), /not position bounds.*confidence interval/);
        for (const [name, value] of [['lower', 1174], ['nominal', 1234], ['upper', 1294]]) {
          const expected = engine[single](jd, body, value, 60, 'test source');
          assert.equal(result[name].deltaTSeconds, value);
          assert.equal(result[name].jdTt, expected.jdTt);
          assert.equal(result[name].jdTdb, expected.jdTdb);
          assert.deepEqual(result[name].xx, expected.xx);
          assert.ok(result[name].xx.every(Number.isFinite));
        }
        assert.ok(result.lower.jdTt < result.nominal.jdTt && result.nominal.jdTt < result.upper.jdTt);
      }
    }
  }
});

test('zero perturbation is explicit; invalid and unrepresentable scenarios fail', opts, () => {
  const engine = new Engine(kernel), jd = julday(2000, 7, 1, 12, true);
  const calc = engine.experimentalLongTermApparentUt1Scenarios.bind(engine);
  const zero = calc(jd, constants.SE_MOON, 64, 0, 'example');
  assert.deepEqual(zero.lower, zero.nominal); assert.deepEqual(zero.upper, zero.nominal);
  for (const [time, dt, width, source] of [[NaN, 1, 1, 'x'], [jd, Infinity, 1, 'x'], [jd, 1, -1, 'x'], [jd, 1, Infinity, 'x'], [jd, 1, 1e-12, 'x'], [jd, 1, 1, ' ']]) {
    assert.throws(() => calc(time, constants.SE_MOON, dt, width, source), e => e.code === 'E_RANGE');
  }
  assert.throws(() => calc(jd, constants.SE_EARTH, 1, 1, 'x'), e => e.code === 'E_UNSUPPORTED_BODY');
  // The nominal and lower samples are in range; the upper one is beyond the
  // experimental orientation guard. No partial result should be returned.
  const late = julday(3999, 7, 1, 12, true), width = 1500 * 365.25 * 86400;
  assert.throws(() => calc(late, constants.SE_MARS, 0, width, 'synthetic'),
    e => e.code === 'E_CALC' && /Upper Delta T scenario/.test(e.message));
});

test('ancient Moon example exposes sensitivity to a one-hour perturbation', opts, () => {
  const engine = new Engine(kernel), jd = julday(-12999, 7, 1, 12, true);
  // Deliberately illustrative, not a claim that this is the historical Delta T or its uncertainty.
  const r = engine.experimentalLongTermDirectObliquityUt1Scenarios(jd, constants.SE_MOON, 197.95 * 3600, 3600, 'illustrative scenario');
  const signed = x => ((x % 360) + 540) % 360 - 180;
  const change = Math.abs(signed(r.upper.xx[0] - r.nominal.xx[0]));
  assert.ok(change > 0.4 && change < 0.8, `${change} degrees`);
});

test('calendar edge samples work and zodiac wrap does not imply longitude bounds', opts, () => {
  const engine = new Engine(kernel);
  for (const jd of [julday(-12999, 1, 1, 0, true), julday(3999, 12, 31, 0, true)]) {
    for (let body = 0; body <= 9; body++) {
      const result = engine.experimentalLongTermApparentUt1Scenarios(jd, body, 1000, 60, 'synthetic');
      assert.ok([result.lower, result.nominal, result.upper].every(s => s.xx.every(Number.isFinite)));
    }
  }
  const crossing = engine.experimentalLongTermApparentUt1Scenarios(2451556.25, constants.SE_MOON, 64, 3600, 'synthetic');
  assert.ok(crossing.lower.xx[0] > 359 && crossing.upper.xx[0] < 1);
  assert.match(crossing.notes.join(' '), /not position bounds/);
});
