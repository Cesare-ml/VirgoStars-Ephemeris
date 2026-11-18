// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { epochGrid, urlFor } = require('../scripts/validate-horizons-planet-holdout.cjs');
const { epochGrid: priorGrid } = require('../scripts/sample-horizons-frame-epochs.cjs');

test('holdout epochs are unique and disjoint from the earlier Horizons grid', () => {
  const grid = epochGrid(), old = new Set(priorGrid().map(r => r.jdTt));
  assert.equal(grid.length, 30);
  assert.equal(new Set(grid.map(r => r.jdTt)).size, 30);
  assert.ok(grid.every(r => !old.has(r.jdTt) && r.day === 17 && r.hourTt === 6));
  assert.equal(grid[0].year, -9950);
  assert.equal(grid.at(-1).year, 3975);
});

test('Horizons query fixes time, origin, frame and correction contract', () => {
  const params = new URL(urlFor('199', epochGrid().map(r => r.jdTt))).searchParams;
  for (const [key, value] of Object.entries({ COMMAND: "'199'", CENTER: "'500@399'", TIME_TYPE: "'TT'", REF_SYSTEM: "'ICRF'", APPARENT: "'AIRLESS'", QUANTITIES: "'2,31,45'" })) assert.equal(params.get(key), value);
  assert.equal(params.get('TLIST').slice(1, -1).split(' ').length, 30);
  assert.throws(() => urlFor('invalid', epochGrid().map(r => r.jdTt)), /invalid query/);
  assert.throws(() => urlFor('10', [NaN]), /invalid query/);
});
