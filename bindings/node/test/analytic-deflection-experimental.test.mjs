// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const { Engine, julday, constants } = createRequire(import.meta.url)('../index.js');
const kernel = process.env.DE441_PATH;
const options = { skip: kernel ? false : 'DE441_PATH required' };

test('analytic deflection is opt-in, exposes its limitations and preserves ordinary output', options, () => {
  const engine = new Engine(kernel);
  const ordinary = engine.calcTdb(2451545, 5, constants.SEFLG_SPEED);
  for (const year of [-12999, -10000, -5000, 0, 1000, 2000, 3000, 3999]) {
    const jd = julday(year, 7, 1, 12, true);
    for (let body = 0; body <= 9; body++) {
      const r = engine.experimentalAnalyticDeflectionIcrfTdb(jd, body);
      assert.equal(r.jdTdb, jd);
      assert.equal(r.center, 'Earth');
      assert.equal(r.frame, 'ICRF');
      assert.equal(r.positionKm.length, 3);
      assert.equal(r.velocityKmPerDay.length, 3);
      assert.ok([...r.positionKm, ...r.velocityKmPerDay].every(Number.isFinite));
      assert.match(r.notes.join(' '), /EXPERIMENTAL.*accuracy not established/);
      assert.match(r.notes.join(' '), /unrefined.*emission JD/);
      assert.match(r.notes.join(' '), /E_DEFLECTION_BOUNDARY/);
    }
  }
  assert.deepEqual(engine.calcTdb(2451545, 5, constants.SEFLG_SPEED), ordinary);
  assert.throws(() => engine.experimentalAnalyticDeflectionIcrfTdb(NaN, 0), e => e.code === 'E_RANGE');
  assert.throws(() => engine.experimentalAnalyticDeflectionIcrfTdb(2451545, 14), e => e.code === 'E_UNSUPPORTED_BODY');
  assert.throws(() => engine.experimentalAnalyticDeflectionIcrfTdb(1e8, 0), e => e.code === 'E_CALC');
});

test('analytic apparent angular rate agrees with a fourth-order position difference', options, () => {
  const engine = new Engine(kernel);
  // A coarser stencil at this close approach has substantial truncation error.
  const jd = -1474550, body = constants.SE_JUPITER, h = 1 / 128;
  const get = t => engine.experimentalAnalyticDeflectionIcrfTdb(t, body);
  const unit = p => { const r = Math.hypot(...p); return p.map(x => x / r); };
  const center = get(jd), u = unit(center.positionKm), r = Math.hypot(...center.positionKm);
  const radial = u.reduce((sum, x, i) => sum + x * center.velocityKmPerDay[i], 0);
  const rate = u.map((x, i) => (center.velocityKmPerDay[i] - x * radial) / r);
  const samples = [-2, -1, 1, 2].map(x => unit(get(jd + x * h).positionKm));
  const finite = u.map((_, i) => (samples[0][i] - 8*samples[1][i] + 8*samples[2][i] - samples[3][i]) / (12*h));
  const error = Math.hypot(...rate.map((x, i) => x - finite[i])) * 206264.80624709636;
  assert.ok(error < 0.00005, `${error} arcsec per day`);
});

test('the real DE441 boundary is reported rather than silently falling back', options, () => {
  const engine = new Engine(kernel), jd = -1474169.0782648325;
  assert.throws(() => engine.experimentalAnalyticDeflectionIcrfTdb(jd, constants.SE_URANUS), e => e.code === 'E_DEFLECTION_BOUNDARY');
  for (const offset of [-0.001, 0.001]) {
    const result = engine.experimentalAnalyticDeflectionIcrfTdb(jd + offset, constants.SE_URANUS);
    assert.ok(result.velocityKmPerDay.every(Number.isFinite));
  }
});
