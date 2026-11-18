// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { Engine, julday } from '../index.js';

const kernel = process.env.DE441_PATH;
test('the longitude candidate preserves original positions and derivatives', { skip: kernel ? false : 'DE441_PATH required' }, () => {
  const engine = new Engine(kernel);
  for (const year of [-12999, 0, 2000, 3999]) {
    const jd = julday(year, 7, 1, 12, true);
    for (const body of [0, 1, 4, 9]) {
      const base = engine.experimentalLongTermDirectObliquityTdb(jd, body);
      const candidate = engine.experimentalLongitudeRateCandidateTdb(jd, body);
      assert.deepEqual(candidate.base.xx, base.xx);
      assert.equal(candidate.candidateDegPerTdbDay, base.xx[3] + candidate.correctionDegPerTdbDay);
      assert.ok(Number.isFinite(candidate.candidateDegPerTdbDay));
      assert.match(candidate.notes.join(' '), /not a position derivative/);
    }
  }
  assert.throws(() => engine.experimentalLongitudeRateCandidateTdb(NaN, 0), e => e.code === 'E_RANGE');
  assert.throws(() => engine.experimentalLongitudeRateCandidateTdb(2451545, 15), e => e.code === 'E_UNSUPPORTED_BODY');
});

test('the four observed Pluto station sign disagreements are reduced against local Swiss',
  { skip: kernel && process.env.SWISSEPH_PATH ? false : 'DE441_PATH and SWISSEPH_PATH required' }, () => {
    const swiss = createRequire(import.meta.url)(process.env.SWISSEPH_PATH);
    const { tdbToTt } = createRequire(import.meta.url)('../scripts/validate-mvp1-swiss-rates.cjs');
    const { dirname, basename } = createRequire(import.meta.url)('node:path');
    swiss.swe_set_ephe_path(dirname(kernel)); swiss.swe_set_jpl_file(basename(kernel));
    const engine = new Engine(kernel), flags = swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED;
    const roots = [-2844137.7764983177, -2844341.0428538322, 1721424.058371067, 1721212.5611605644];
    for (const root of roots) {
      const signs = [-1, 1].map(k => {
        const jd = root + k * 120 / 86400;
        const ref = swiss.swe_calc(tdbToTt(jd), 9, flags);
        assert.equal(ref.rflag, flags); assert.ok(Number.isFinite(ref.longitudeSpeed));
        const candidate = engine.experimentalLongitudeRateCandidateTdb(jd, 9);
        return { expected: Math.sign(ref.longitudeSpeed), original: Math.sign(candidate.base.xx[3]), candidate: Math.sign(candidate.candidateDegPerTdbDay) };
      });
      assert.ok(signs.some(s => s.original !== s.expected));
      assert.ok(signs.every(s => s.expected !== 0 && s.candidate === s.expected));
    }
  });
