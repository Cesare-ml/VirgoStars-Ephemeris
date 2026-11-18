// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const { assess } = createRequire(import.meta.url)('../scripts/assess-mvp1-swiss-stations.cjs');

test('station gate requires complete matrices, matching signs and the explicit time threshold', () => {
  const hash = 'a'.repeat(64);
  const reports = [[-12500, 0, 3900], [-11000, 1500, 3500]].map(years => ({
    schema: 'experimental-longitude-stations/1', status: 'EXPERIMENTAL_NOT_ACCEPTED', kernelSha256: hash,
    rows: years.flatMap(year => Array.from({ length: 8 }, (_, i) => ['retrograde', 'direct'].map(kind => {
      const jdTdb = 2451545 + year * 365.25, delta = 120 / 86400;
      return { year, body: i + 2, kind, jdTdb, candidateDeltaSeconds: 0, candidateMatch: true, samples: [{ jdTdb: jdTdb - delta, candidate: 1, published: 1 }, { jdTdb: jdTdb + delta, candidate: -1, published: -1 }] };
    })).flat()),
  }));
  assert.equal(assess(reports, hash, 15).timeFailures, 0);
  reports[0].rows[0].candidateDeltaSeconds = -15.1;
  assert.equal(assess(reports, hash, 15).timeFailures, 1);
  reports[0].rows[0].candidateDeltaSeconds = 15;
  assert.equal(assess(reports, hash, 15).timeFailures, 0);
  reports[1].rows[0].samples[0].candidate = -1;
  assert.throws(() => assess(reports, hash, 15));
  reports[1].rows[0].candidateMatch = false;
  assert.equal(assess(reports, hash, 15).signFailures, 1);
  reports[1].rows.pop();
  assert.throws(() => assess(reports, hash, 15));
});
