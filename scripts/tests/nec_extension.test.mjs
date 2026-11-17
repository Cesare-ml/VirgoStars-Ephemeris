// SPDX-License-Identifier: CPAL-1.0. Fictional data only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseCsv, prepareNecExtension } from '../extend_star_manifest_nec.mjs';
const entry = { name: 'Existing', hip: 1, aliases: [], identitySource: 'fixture', qualityDecision: 'fixture', magnitude: 1, magnitudeSource: 'fixture', radialVelocityKmS: null, radialVelocityMissingReason: 'unknown' };
const hip2Sha256 = 'a'.repeat(64);
const manifest = { schema: 'virgostars-star-manifest/1', reviewer: 'fixture', astrometrySource: 'fixture', dataUseDecision: 'fixture', qualityPolicy: 'strict', sourceHashes: { hip2Sha256 }, stars: [entry] };
const swiss = '# sefstars.txt\nExample,des1,ICRS,01,00,00,+30,00,00,0,0,0,1,1,0,0\nDistant,des2,ICRS,01,00,00,+30,00,00,0,0,0,1,1,0,0\n';
const nec = 'Name,HIP,RA2000,DE2000,Vmag\nExample,HIP 2,15,30,2.3\nDistant,HIP 3,16,30,2.3\n';
const inspect = (_path, hips) => hips.map(hip => ({ hip, strictEligible: true }));
test('NEC extension requires exact name, matching coordinates and strict HIP quality', () => {
  const result = prepareNecExtension({ manifest, swissText: swiss, necText: nec, hip2Sha256, hip2Path: 'mock', inspect });
  assert.equal(result.report.selectedCount, 1); assert.equal(result.manifest.stars[1].name, 'Example');
  assert.equal(result.manifest.stars[1].hip, 2); assert.equal(result.manifest.stars[1].radialVelocityKmS, null);
  assert.equal(result.manifest.stars.length, 2); assert.equal(manifest.stars.length, 1);
  const rejected = prepareNecExtension({ manifest, swissText: swiss, necText: nec, hip2Sha256, hip2Path: 'mock', inspect: (_p, hips) => hips.map(hip => ({ hip, strictEligible: false })) });
  assert.equal(rejected.report.selectedCount, 0);
  assert.throws(() => prepareNecExtension({ manifest, swissText: swiss, necText: nec, hip2Sha256: 'b'.repeat(64), hip2Path: 'mock', inspect }), /source mismatch/);
});
test('CSV reader handles quoted delimiters and rejects malformed input', () => {
  assert.deepEqual(parseCsv('a,b\n"x,y","z""q"\n'), [['a', 'b'], ['x,y', 'z"q']]);
  assert.throws(() => parseCsv('a,"broken'));
});
