// SPDX-License-Identifier: CPAL-1.0. Fictional records only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {attachCenturyWarnings} from '../attach_century_diagnostic_warnings.mjs';

const bytes=value=>Buffer.from(JSON.stringify(value));
const hash=value=>createHash('sha256').update(value).digest('hex');
const fixture=()=>({
  schema:'virgostars-star-manifest/2',reviewer:'fixture',astrometrySource:'fixture',
  dataUseDecision:'fixture',qualityPolicy:'experimental',
  sourceHashes:{hip2Sha256:'a'.repeat(64),swissCatalogSha256:'b'.repeat(64)},
  stars:[{name:'Direct',aliases:['Alias'],sourceId:'fixture:1',astrometrySource:'fixture',qualityTier:'experimental',
    astrometry:{epochTdbJd:2451545,rightAscensionDeg:30,declinationDeg:15,properMotionRaCosDecMasPerYear:10,properMotionDecMasPerYear:-5,parallaxMas:10},
    identitySource:'fixture',qualityDecision:'fixture',magnitude:1,magnitudeSource:'fixture',radialVelocityKmS:null,radialVelocityMissingReason:'fixture'}]
});
function report(manifest){
  return {schema:'virgostars-star-century-diagnostic/1',status:'REVIEW_REQUIRED',manifestSha256:hash(manifest),
    hip2Sha256:'a'.repeat(64),swissCatalogSha256:'b'.repeat(64),
    epochs:[1900,1925,1950,1975,2000,2025,2050,2075,2099].map(year=>({year})),
    summary:{stars:1,thresholdArcsec:1,failedCases:1},failures:[
      {sourceId:'fixture:1',name:'Alias',year:1900,reason:'POSITION_MISMATCH',longitudeArcsec:1.25,latitudeArcsec:0.5}
    ]};
}
test('attaches a warning only to the affected direct-star lookup',()=>{
  const input=bytes(fixture()),diagnostic=bytes(report(input));
  const result=attachCenturyWarnings(input,diagnostic);
  assert.equal(result.added,1);
  assert.equal(result.manifest.stars[0].lookupWarnings[0].name,'Alias');
  assert.equal(result.manifest.stars[0].lookupWarnings[0].code,'SWISS_POSITION_MISMATCH');
  assert.match(result.manifest.stars[0].lookupWarnings[0].detail,/1\.250 arcsec/);
  assert.equal(result.manifest.sourceHashes.centuryDiagnosticSha256,hash(diagnostic));
});
test('rejects stale reports and failures for another object or below threshold',()=>{
  const input=bytes(fixture());
  for(const change of [
    r=>{r.manifestSha256='0'.repeat(64);},
    r=>{r.failures[0].sourceId='fixture:other';},
    r=>{r.failures[0].longitudeArcsec=0.5;},
    r=>{r.failures[0].name='Unknown';},
  ]){const r=report(input);change(r);assert.throws(()=>attachCenturyWarnings(input,bytes(r)));}
});
