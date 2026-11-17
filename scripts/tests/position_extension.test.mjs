// SPDX-License-Identifier: CPAL-1.0. Fictional astronomical records.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { extendByPosition } from '../extend_star_manifest_positions.mjs';
const hash = text => createHash('sha256').update(text).digest('hex');
const row = (name, id, h, mag = 2) => `${name},${id},ICRS,${h},00,00,+30,00,00,0,0,0,1,${mag},0,0`;
const swiss = `# sefstars.txt\n${row('Alpha', 'A1', '01')}\n${row('', 'B1', '02')}\n${row('Close', 'C1', '03')}\n`;
const nec = 'HIP,RA2000,DE2000,Vmag\nHIP 2,15,30,2\nHIP 3,30,30,2\nHIP 4,45,30,2\nHIP 5,45.00001,30,2\n';
const hip2Sha256 = 'a'.repeat(64);
const manifest = { schema: 'virgostars-star-manifest/1', reviewer: 'fixture', astrometrySource: 'fixture', dataUseDecision: 'fixture', qualityPolicy: 'strict',
  sourceHashes: { hip2Sha256, swissCatalogSha256: hash(swiss), necSha256: hash(nec) },
  stars: [{ name: 'Existing', hip: 1, aliases: [], identitySource: 'fixture', qualityDecision: 'fixture', magnitude: 1, magnitudeSource: 'fixture', radialVelocityKmS: null, radialVelocityMissingReason: 'unknown' }] };
const inspect = (_path, hips) => hips.map(hip => ({ hip, strictEligible: hip === 2 }));
test('selects only unique position and strict quality, preserving candidate aliases', () => {
  const result = extendByPosition({ manifest, swissText: swiss, necText: nec, hip2Sha256, hip2Path: 'mock', inspect });
  assert.equal(result.report.oneToOneCandidates, 2);
  assert.equal(result.report.selectedCount, 1);
  assert.equal(result.manifest.stars[1].name, ',A1');
  assert.deepEqual(result.manifest.stars[1].aliases, ['Alpha']);
  assert.equal(manifest.stars.length, 1);
});
test('rejects source mismatches and incomplete quality records', () => {
  assert.throws(() => extendByPosition({ manifest, swissText: `${swiss} `, necText: nec, hip2Sha256, hip2Path: 'mock', inspect }), /source hash/);
  assert.throws(() => extendByPosition({ manifest, swissText: swiss, necText: nec, hip2Sha256, hip2Path: 'mock', inspect: () => [] }), /inconsistent HIP2/);
});
test('traditional policy is explicit and preserves per-star quality labels', () => {
  const mixed = (_path, hips) => hips.map(hip => ({ hip, strictEligible: hip === 1, traditionalEligible: true }));
  const result = extendByPosition({ manifest, swissText: swiss, necText: nec, hip2Sha256, hip2Path: 'mock', inspect: mixed, qualityPolicy:'traditional' });
  assert.equal(result.manifest.qualityPolicy,'traditional');
  assert.equal(result.manifest.stars[0].qualityTier,'strict');
  assert.equal(result.manifest.stars[1].qualityTier,'traditional');
  assert.match(result.manifest.stars[1].qualityDecision,/traditional/);
  assert.throws(() => extendByPosition({ manifest, swissText: swiss, necText: nec, hip2Sha256, hip2Path:'mock', inspect:mixed, qualityPolicy:'automatic' }), /explicit/);
});
