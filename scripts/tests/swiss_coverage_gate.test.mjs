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
test('reports unavailable magnitudes and assumed velocities for names and aliases separately',()=>{
  const m=manifest();m.stars[0].aliases=[',a1'];m.stars[0].magnitude=null;m.stars[0].magnitudeMissingReason='fixture';
  const r=verifySwissCoverage(bytes(m),swiss);
  assert.equal(r.status,'COMPLETE_LOOKUP_COVERAGE_ONLY');
  assert.equal(r.fieldCoverage.starsWithoutMagnitude,1);
  assert.equal(r.fieldCoverage.starsAssumingZeroRadialVelocity,1);
  assert.deepEqual(r.fieldCoverage.namesWithoutMagnitude,['Alpha']);
  assert.deepEqual(r.fieldCoverage.designationsWithoutMagnitude,[',A1']);
  assert.deepEqual(r.fieldCoverage.namesAssumingZeroRadialVelocity,['Alpha']);
  assert.deepEqual(r.fieldCoverage.designationsAssumingZeroRadialVelocity,[',A1']);
  m.stars[0].radialVelocityKmS=0;m.stars[0].radialVelocitySource='fixture';
  assert.equal(verifySwissCoverage(bytes(m),swiss).fieldCoverage.starsAssumingZeroRadialVelocity,0);
});
test('does not treat unmatched lookups as missing fields on known stars',()=>{
  const r=verifySwissCoverage(bytes(manifest()),swiss);
  assert.deepEqual(r.missingDesignations,[',A1']);
  assert.deepEqual(r.fieldCoverage.designationsWithoutMagnitude,[]);
  assert.deepEqual(r.fieldCoverage.designationsAssumingZeroRadialVelocity,[]);
});
