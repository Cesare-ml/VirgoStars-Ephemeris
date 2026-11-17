// SPDX-License-Identifier: CPAL-1.0. Fictional records only.
import {test} from 'node:test';import assert from 'node:assert/strict';import {createHash} from 'node:crypto';
import {verifySwissCoverage} from '../verify_swiss_coverage.mjs';
const sha=b=>createHash('sha256').update(b).digest('hex'), bytes=x=>Buffer.from(JSON.stringify(x));
const swiss=Buffer.from('# sefstars.txt\nAlpha,A1,ICRS,1,2,3,4,5,6,7,8,9,10,11,0,0\n');
function manifest(){return {schema:'virgostars-star-manifest/1',reviewer:'fixture',astrometrySource:'fixture',dataUseDecision:'fixture',qualityPolicy:'strict',
  sourceHashes:{swissCatalogSha256:sha(swiss)},stars:[{name:'Alpha',aliases:[],hip:1,identitySource:'fixture',qualityDecision:'fixture',
    magnitude:1,magnitudeSource:'fixture',radialVelocityKmS:null,radialVelocityMissingReason:'fixture'}]};}
test('distinguishes exact name coverage from missing designation',()=>{
  const m=manifest();let r=verifySwissCoverage(bytes(m),swiss);
  assert.equal(r.status,'INCOMPLETE');assert.deepEqual(r.missingDesignations,[',A1']);assert.equal(r.counts.coveredNames,1);
  m.stars[0].aliases=[',a1'];r=verifySwissCoverage(bytes(m),swiss);
  assert.equal(r.status,'COMPLETE_LOOKUP_COVERAGE_ONLY');assert.equal(r.counts.coveredDesignations,1);
});
test('rejects a source mismatch',()=>{
  assert.throws(()=>verifySwissCoverage(bytes(manifest()),Buffer.from(swiss+' ')),/hash mismatch/);
});
