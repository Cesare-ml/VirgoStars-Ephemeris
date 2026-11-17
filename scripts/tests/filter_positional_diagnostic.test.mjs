// SPDX-License-Identifier: CPAL-1.0. Fictional data only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { filterByDiagnostic } from '../filter_positional_swiss_diagnostic.mjs';
const bytes = value => Buffer.from(JSON.stringify(value));
const sha = value => createHash('sha256').update(value).digest('hex');
const entry = { name: ',a', hip: 2, aliases: [], identitySource: 'Swiss-NEC position <=1 arcsec fixture', qualityDecision: 'fixture', magnitude: 1, magnitudeSource: 'fixture', radialVelocityKmS: null, radialVelocityMissingReason: 'unknown' };
const m = bytes({ schema: 'virgostars-star-manifest/1', reviewer: 'fixture', astrometrySource: 'fixture', dataUseDecision: 'fixture', qualityPolicy: 'strict', sourceHashes: { hip2Sha256: 'a', swissCatalogSha256: 'b' }, stars: [entry] });
const report = { schema: 'virgostars-positional-swiss-diagnostic/1', status: 'REVIEW_REQUIRED', manifestSha256: sha(m), hip2Sha256: 'a', swissCatalogSha256: 'b', stars: 1,
  kernelSha256: 'c'.repeat(64), epochs: [{ year: 1950, jd: 2433417 }, { year: 2000, jd: 2451545 }, { year: 2024, jd: 2460390 }], thresholdArcsec: 1, refErrors: 0, engineErrors: 0, comparisons: 3, within: 3, outliers: [] };
test('retains passing candidates and rejects failed or mismatched evidence', () => {
  assert.equal(filterByDiagnostic(m, bytes(report)).manifest.stars.length, 1);
  assert.throws(() => filterByDiagnostic(m, bytes({ ...report, refErrors: 1 })), /invalid diagnostic/);
  assert.throws(() => filterByDiagnostic(m, bytes({ ...report, manifestSha256: 'wrong' })), /source mismatch/);
  assert.throws(() => filterByDiagnostic(m, bytes({ ...report, within: 2, outliers: [{ name: ',bad', hip: 2, year: 1950, longitudeArcsec: 2, latitudeArcsec: 0 }] })), /invalid outlier/);
  assert.throws(() => filterByDiagnostic(m, bytes({ ...report, mode: 'unknown' })), /invalid diagnostic mode/);
});

test('full-catalog diagnostic withholds an outlier without dropping passing entries', () => {
  const original = JSON.parse(m);
  original.stars.push({ ...entry, name: 'Other', hip: 3, identitySource: 'independent source' });
  const both = bytes(original);
  const full = { ...report, mode: 'all', manifestSha256: sha(both), stars: 2, comparisons: 6, within: 5,
    outliers: [{ name: 'Other', hip: 3, year: 1950, longitudeArcsec: 1.01, latitudeArcsec: 0.1 }] };
  const result = filterByDiagnostic(both, bytes(full));
  assert.deepEqual(result.removed, [3]);
  assert.deepEqual(result.manifest.stars.map(s => s.name), [',a']);
  assert.equal(result.manifest.sourceHashes.allStarSwissDiagnosticSha256, sha(bytes(full)));
});
