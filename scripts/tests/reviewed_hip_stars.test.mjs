// SPDX-License-Identifier: CPAL-1.0. Fictional records only.
import {test} from 'node:test';import assert from 'node:assert/strict';import {createHash} from 'node:crypto';
import {addReviewedHipStars} from '../add_reviewed_hip_stars.mjs';
const sha=b=>createHash('sha256').update(b).digest('hex'),bytes=x=>Buffer.from(JSON.stringify(x));
const swiss=Buffer.from('# sefstars.txt\nAlpha,A1,ICRS,1,2,3,4,5,6,7,8,9,10,11,0,0\nBeta,B1,ICRS,1,2,3,4,5,6,7,8,9,10,11,0,0\n'),hip=Buffer.from('fictional hip');
const star=(name,id)=>({name,aliases:[],hip:id,qualityTier:'experimental',identitySource:'fixture',qualityDecision:'fixture',magnitude:1,magnitudeSource:'fixture',radialVelocityKmS:null,radialVelocityMissingReason:'fixture'});
function fixture(){const m=bytes({schema:'virgostars-star-manifest/1',reviewer:'fixture',astrometrySource:'fixture',dataUseDecision:'fixture',qualityPolicy:'experimental',sourceHashes:{swissCatalogSha256:sha(swiss),hip2Sha256:sha(hip)},stars:[star('Alpha',1)]});
  return {m,d:{schema:'virgostars-reviewed-hip-stars/1',manifestSha256:sha(m),swissCatalogSha256:sha(swiss),hip2Sha256:sha(hip),reviewer:'fixture',stars:[{...star('Beta',2),aliases:[',B1']}]}};}
const inspect=()=>[{hip:2,strictEligible:false,traditionalEligible:false,experimentalEligible:true}];
test('adds a source-bound eligible HIP candidate',()=>{const {m,d}=fixture(),r=addReviewedHipStars(m,swiss,hip,bytes(d),'fixture',inspect);assert.equal(r.manifest.stars.length,2);assert.equal(r.report.status,'CALLER_ASSERTED');});
test('rejects changed source, wrong tier, duplicate or unknown lookup',()=>{const {m,d}=fixture();
  assert.throws(()=>addReviewedHipStars(m,swiss,Buffer.from('changed'),bytes(d),'fixture',inspect),/hash/);
  for(const update of [{hip:1},{qualityTier:'strict'},{name:'Unknown'},{aliases:['Alpha']}]){const bad=structuredClone(d);Object.assign(bad.stars[0],update);assert.throws(()=>addReviewedHipStars(m,swiss,hip,bytes(bad),'fixture',inspect));}
  assert.throws(()=>addReviewedHipStars(m,swiss,hip,bytes(d),'fixture',()=>[]),/quality/);
  assert.throws(()=>addReviewedHipStars(m,swiss,hip,bytes(d),'fixture',()=>[{hip:2,experimentalEligible:true}]),/quality/);
});
