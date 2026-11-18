// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { julday } from '../index.js';
const { assess } = createRequire(import.meta.url)('../scripts/assess-mvp1-angular-speeds.cjs');

test('finite angular gate checks coverage, threshold and the separate lunar longitude rule', () => {
  const hash = 'a'.repeat(64), rows = [];
  for (const year of [-12500, -10000, -5000, 0, 1000, 2000, 3000, 3900]) for (const month of [1, 4, 7, 10]) for (let body = 0; body < 10; body++) rows.push({ year, month, body, jdTdb: julday(year, month, 15, 12, true), latitudeCandidate: 0, longitudeCandidate: 0 });
  const report = { schema: 'experimental-latitude-moon-rates/1', status: 'EXPERIMENTAL_NOT_ACCEPTED', inputSha256: [hash], rows };
  assert.equal(assess(report, hash, 0.1).componentChecks, 608);
  rows[1].longitudeCandidate = 9; // Moon longitude is evaluated by a separate gate.
  assert.equal(assess(report, hash, 0.1).failures, 0);
  rows[1].latitudeCandidate = -0.11;
  assert.equal(assess(report, hash, 0.1).failures, 1);
  rows[0].body = 1;
  assert.throws(() => assess(report, hash, 0.1));
});
