// SPDX-License-Identifier: CPAL-1.0. Fictional data only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {attachRadialVelocities} from '../attach_sourced_radial_velocities.mjs';
const manifest={schema:'virgostars-star-manifest/1',reviewer:'fixture',astrometrySource:'fixture',dataUseDecision:'fixture',qualityPolicy:'strict',
  stars:[{name:'Example',aliases:[],hip:1,identitySource:'fixture',qualityDecision:'fixture',magnitude:1,magnitudeSource:'fixture',radialVelocityKmS:null,radialVelocityMissingReason:'unknown'}]};
const bytes=x=>Buffer.from(JSON.stringify(x));
const input={schema:'virgostars-radial-velocities/1',status:'REVIEW_REQUIRED',measurements:[{hip:1,kmPerSecond:-20,source:'https://example.org',evidence:'fictional'}]};
test('records a sourced radial velocity without altering the input',()=>{
  const r=attachRadialVelocities(bytes(manifest),bytes(input));
  assert.equal(r.manifest.stars[0].radialVelocityKmS,-20);
  assert.match(r.manifest.stars[0].radialVelocitySource,/https:\/\/example.org/);
  assert.equal(manifest.stars[0].radialVelocityKmS,null);
});
test('rejects duplicate, unknown and unprovenanced measurements and overwrites',()=>{
  assert.throws(()=>attachRadialVelocities(bytes(manifest),bytes({...input,measurements:[input.measurements[0],input.measurements[0]]})),/invalid/);
  assert.throws(()=>attachRadialVelocities(bytes(manifest),bytes({...input,measurements:[{...input.measurements[0],hip:2}]})),/invalid/);
  assert.throws(()=>attachRadialVelocities(bytes(manifest),bytes({...input,measurements:[{...input.measurements[0],source:'no source'}]})),/invalid/);
  assert.throws(()=>attachRadialVelocities(bytes({...manifest,stars:[{...manifest.stars[0],radialVelocityKmS:10,radialVelocitySource:'old'}]}),bytes(input)),/overwrite/);
});
