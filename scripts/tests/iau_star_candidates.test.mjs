// SPDX-License-Identifier: CPAL-1.0; fictional records only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { findCandidates } from '../iau_star_candidates.mjs';

function row(name, hip, mag = '1.20', band = 'V', component = 'A') {
  const a = Array(140).fill(' ');
  const put = (offset, s) => { for (let i = 0; i < s.length; i++) a[offset + i] = s[i]; };
  put(0, name); put(65, component); put(81, mag.padStart(5));
  put(87, band); put(90, String(hip).padStart(7)); put(126, '2020-01-02');
  return a.join('');
}
const catalog = (rows) => `#Name/ASCII Dec(J2000)\n${rows.join('\n')}\n`;
const source = 'https://example.org/fictional';

test('only exact names become review-required candidates; shared HIP is flagged', () => {
  const r = findCandidates(catalog([row('Alpha', 123), row('Beta', 123)]), [' alpha ', 'Beta', 'Alph'], source);
  assert.equal(r.catalogRecordCount, 2);
  assert.equal(r.exactNameCandidates, 2);
  assert.equal(r.unmatched, 1);
  assert.equal(r.results[0].status, 'CANDIDATE_REVIEW_REQUIRED');
  assert.deepEqual(r.results[0].candidates[0].sharedHipWith, ['Beta']);
  assert.equal(r.results[0].candidates[0].magnitude, 1.2);
  assert.equal(r.results[2].status, 'UNMATCHED');
});

test('ambiguous names, missing fields and malformed input fail closed', () => {
  const r = findCandidates(catalog([row('Alpha', 1), row('Alpha', 2)]), ['Alpha'], source);
  assert.equal(r.results[0].status, 'AMBIGUOUS');
  const empty = findCandidates(catalog([row('Alpha', '_', '_', '_')]), ['Alpha'], source);
  assert.equal(empty.results[0].candidates[0].hip, null);
  assert.equal(empty.results[0].candidates[0].magnitude, null);
  assert.throws(() => findCandidates('wrong header', ['Alpha'], source));
  assert.throws(() => findCandidates(catalog(['malformed']), ['Alpha'], source));
  assert.throws(() => findCandidates(catalog([row('Alpha', 1)]), ['Alpha', ' alpha '], source));
  assert.throws(() => findCandidates(catalog([row('Alpha', 1)]), ['Alpha'], 'http://example.org'));
});
