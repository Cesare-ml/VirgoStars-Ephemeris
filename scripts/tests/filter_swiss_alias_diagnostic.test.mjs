// SPDX-License-Identifier: CPAL-1.0. Fictional data only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { filterAliases } from '../filter_swiss_alias_diagnostic.mjs';
const bytes = value => Buffer.from(JSON.stringify(value));
const sha = value => createHash('sha256').update(value).digest('hex');
const sourceHashes = {hip2Sha256:'a', swissCatalogSha256:'b'};
const m = bytes({schema:'virgostars-star-manifest/1',reviewer:'fixture',astrometrySource:'fixture',dataUseDecision:'fixture',qualityPolicy:'strict',sourceHashes,
  stars:[{name:'Star',aliases:['Good','Bad'],hip:123,identitySource:'fixture',qualityDecision:'fixture',magnitude:1,magnitudeSource:'fixture',radialVelocityKmS:null,radialVelocityMissingReason:'unknown'}]});
const report = {schema:'virgostars-swiss-alias-audit/1',status:'REVIEW_REQUIRED',manifestSha256:sha(m),hip2Sha256:'a',swissCatalogSha256:'b',kernelSha256:'c'.repeat(64),
  epochs:[{year:1950,jd:2433417},{year:2000,jd:2451545},{year:2024,jd:2460390}],thresholdArcsec:1,aliases:2,comparisons:6,within:5,
  failures:[{name:'Star',hip:123,alias:'Bad',year:2000,reason:'POSITION_MISMATCH',longitudeArcsec:2,latitudeArcsec:0}]};
test('withholds a bad alias without removing the star or the passing alias', () => {
  const result = filterAliases(m,bytes(report));
  assert.equal(result.removedAliases,1); assert.deepEqual(result.manifest.stars[0].aliases,['Good']);
  assert.equal(result.manifest.stars[0].name,'Star');
});
test('rejects altered manifests and invented or duplicate failures', () => {
  assert.throws(()=>filterAliases(m,bytes({...report,manifestSha256:'bad'})),/source mismatch/);
  assert.throws(()=>filterAliases(m,bytes({...report,failures:[{...report.failures[0],alias:'Unknown'}]})),/invalid alias failure/);
  assert.throws(()=>filterAliases(m,bytes({...report,within:4,failures:[report.failures[0],report.failures[0]]})),/invalid alias failure/);
});
