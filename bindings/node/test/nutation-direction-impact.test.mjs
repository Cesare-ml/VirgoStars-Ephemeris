// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { checkedVectors, summarize, validateSource } = require('../scripts/audit-nutation-direction-impact.cjs');

test('checks vector norms and agreement with the existing projection', () => {
  const angle = Math.PI / 180;
  const vectors = [1, 0, 0, Math.cos(angle), Math.sin(angle), 0, 1, 0, 0, Math.cos(angle), Math.sin(angle), 0];
  const reference = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
  const out = checkedVectors(vectors, reference);
  assert.ok(Math.abs(out.equatorialArcsec - 3600) < 1e-9);
  assert.ok(Math.abs(out.eclipticArcsec - 3600) < 1e-9);
  assert.throws(() => checkedVectors([...vectors.slice(0, 11), NaN], reference));
  assert.throws(() => checkedVectors([2, ...vectors.slice(1)], reference));
  assert.throws(() => checkedVectors(vectors, [0, 0, 10, 0, 0, 0, 0, 0, 0, 0]));
  assert.throws(() => validateSource({ schema: 'swiss-fixed-icrs-grid/1', grid: 'baseline', flags: 0, rows: [] }));
});

test('reports maxima by axis, body and epoch subgroup', () => {
  const rows = Array.from({ length: 10 }, (_, id) => ({ id, year: -12000, month: 1, equatorialArcsec: id, eclipticArcsec: 10 - id }));
  const summary = summarize(rows);
  assert.equal(summary.all.equatorial.id, 9);
  assert.equal(summary.all.ecliptic.id, 0);
  assert.equal(summary.preHorizons.cases, 10);
  assert.equal(summary.perBody.length, 10);
  assert.throws(() => summarize([{ ...rows[0], equatorialArcsec: NaN }]));
});
