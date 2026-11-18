// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { validateTolerance, summarizeRows, validateReport } = require('../scripts/assess-mvp1-swiss-positions.cjs');

test('caller supplies an explicit positive finite angular tolerance', () => {
  assert.equal(validateTolerance(1), 1);
  for (const invalid of [0, -1, NaN, Infinity, '1', 648001]) assert.throws(() => validateTolerance(invalid), /invalid tolerance/);
});

test('conditional assessment counts failures per body and ancient subset', () => {
  const rows = [-12000, 2000].flatMap(year => Array.from({ length: 10 }, (_, id) => ({ year, month: 1, id, directActualArcsec: id === 1 && year === -12000 ? 0.5 : 0.01 })));
  const pass = summarizeRows(rows, 0.5);
  assert.equal(pass.all.cases, 20);
  assert.equal(pass.beforeHorizons.cases, 10);
  assert.equal(pass.all.maximumArcsec, 0.5);
  assert.equal(pass.all.meetsTolerance, true);
  const fail = summarizeRows(rows, 0.1);
  assert.equal(fail.all.failures, 1);
  assert.equal(fail.byBody[1].failures, 1);
  assert.equal(fail.all.meetsTolerance, false);
  assert.throws(() => summarizeRows(rows.map(r => ({ ...r, directActualArcsec: NaN })), 1), /invalid evidence/);
  assert.throws(() => summarizeRows(rows.slice(10), 1), /empty evidence group/);
  assert.throws(() => validateReport({}, 'baseline'), /invalid reference report/);
});
