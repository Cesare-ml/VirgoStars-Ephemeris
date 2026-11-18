// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { Engine, julday } from '../index.js';
const require = createRequire(import.meta.url);
const { nextDown } = require('../scripts/audit-de441-epoch-quantization.cjs');
const kernel = process.env.DE441_PATH;

test('explicit UT1 retains caller Delta T and matches the TDB candidate path', { skip: kernel ? false : 'DE441_PATH required' }, () => {
  const engine = new Engine(kernel);
  const dates = [julday(-12999, 1, 1, 0, true), julday(0, 7, 1, 12, true), julday(2000, 1, 1, 12, true), nextDown(julday(4000, 1, 1, 0, true))];
  for (const jd of dates) for (const body of [0, 1, 4, 9]) {
    const result = engine.experimentalThreeRateCandidateUt1(jd, body, 123.5, 20, ' example source ');
    const direct = engine.experimentalThreeRateCandidateTdb(result.jdTdb, body);
    const historicalBase = engine.experimentalLongTermDirectObliquityUt1(jd, body, 123.5, 20, 'example source');
    assert.equal(result.jdUt1, jd);
    assert.equal(result.deltaTSeconds, 123.5);
    assert.equal(result.deltaTUncertaintySeconds, 20);
    assert.equal(result.deltaTSource, 'example source');
    assert.ok(Math.abs((result.jdTt - jd) * 86400 - 123.5) < 0.0001);
    assert.ok(Math.abs((result.jdTdb - result.jdTt) * 86400) < 0.002);
    assert.deepEqual(result.baseXx, direct.base.xx);
    assert.deepEqual(result.baseXx, historicalBase.xx);
    assert.deepEqual(result.candidateXx, direct.candidateXx);
    assert.equal(result.radialCorrectionApplied, body !== 0);
    assert.match(result.notes.join(' '), /not UTC or local time/);
    assert.match(result.notes.join(' '), /not position derivatives/);
    assert.match(result.notes.join(' '), /not converted to a position bound/);
    const changedUncertainty = engine.experimentalThreeRateCandidateUt1(jd, body, 123.5, 0, 'example source');
    assert.equal(changedUncertainty.jdTdb, result.jdTdb);
    assert.deepEqual(changedUncertainty.candidateXx, result.candidateXx);
  }
});

test('explicit UT1 requires finite time, uncertainty, source and supported body', { skip: kernel ? false : 'DE441_PATH required' }, () => {
  const engine = new Engine(kernel), jd = 2451545;
  for (const [time, delta, uncertainty, source] of [[NaN, 1, 1, 'x'], [jd, Infinity, 1, 'x'], [jd, 1, -1, 'x'], [jd, 1, Infinity, 'x'], [jd, 1, 1, ' '], [jd, 1, 1, 'è'.repeat(101)]]) {
    assert.throws(() => engine.experimentalThreeRateCandidateUt1(time, 4, delta, uncertainty, source), e => e.code === 'E_RANGE');
  }
  assert.throws(() => engine.experimentalThreeRateCandidateUt1(jd, 15, 1, 1, 'x'), e => e.code === 'E_UNSUPPORTED_BODY');
  assert.throws(() => engine.experimentalThreeRateCandidateUt1(jd, 4, 1, 1), e => e.code === 'E_RANGE');
  assert.throws(() => engine.experimentalThreeRateCandidateUt1(jd, 4, 1), e => e.code === 'E_RANGE');
  assert.throws(() => engine.experimentalThreeRateCandidateUt1(jd, '4', 1, 1, 'x'), e => e.code === 'E_UNSUPPORTED_BODY');
  assert.throws(() => engine.experimentalThreeRateCandidateUt1(julday(5002, 7, 1, 12, true), 4, 0, 1, 'x'), e => e.code === 'E_CALC');
});
