// SPDX-License-Identifier: CPAL-1.0. Fictional records only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {extendHipMain} from '../extend_star_manifest_hip_main.mjs';
const sha=s=>createHash('sha256').update(s).digest('hex');
function fixed(length,fields){const a=Array(length).fill(' ');for(const [offset,width,value] of fields){const v=String(value).padStart(width);for(let i=0;i<width;i++)a[offset+i]=v[i];}return a.join('');}
const hip2=fixed(276,[[0,6,2],[15,13,(Math.PI/12).toFixed(10)],[29,13,(Math.PI/6).toFixed(10)],[51,8,'0.00'],[60,8,'0.00']])+'\n';
function mainRecord(mag,ra='15.00000000'){return fixed(450,[[0,1,'H'],[8,6,2],[41,5,mag],[51,12,ra],[64,12,'30.00000000']])+'\n';}
const swiss='# sefstars.txt\nAlpha,A1,ICRS,01,00,00,+30,00,00,0,0,0,1,2,0,0\n';
const main=mainRecord('2.02');
const manifest={schema:'virgostars-star-manifest/1',reviewer:'fixture',astrometrySource:'fixture',dataUseDecision:'fixture',qualityPolicy:'experimental',
  sourceHashes:{hip2Sha256:sha(hip2),swissCatalogSha256:sha(swiss)},
  stars:[{name:'Existing',aliases:[],hip:1,qualityTier:'strict',identitySource:'fixture',qualityDecision:'fixture',magnitude:1,magnitudeSource:'fixture',radialVelocityKmS:null,radialVelocityMissingReason:'unknown'}]};
const inspect=(_path,hips)=>hips.map(hip=>({hip,solutionType:15,components:2,strictEligible:false,traditionalEligible:false,experimentalEligible:true}));
const args={manifest,swissText:swiss,hip2Text:hip2,hipMainText:main,hipMainSha256:sha(main),hipPath:'mock',inspect,expectedCount:1};
test('requires consistent HIP, epoch position and independent magnitude',()=>{
  const r=extendHipMain(args);assert.equal(r.manifest.stars[1].name,',A1');assert.equal(r.manifest.stars[1].qualityTier,'experimental');
  const changed=mainRecord('4.00');
  assert.equal(extendHipMain({...args,hipMainText:changed,hipMainSha256:sha(changed)}).report.selected.length,0);
  const wrongPosition=mainRecord('2.02','16.00000000');
  assert.equal(extendHipMain({...args,hipMainText:wrongPosition,hipMainSha256:sha(wrongPosition)}).report.selected.length,0);
});
test('rejects invalid source hashes and truncated catalogs',()=>{
  assert.throws(()=>extendHipMain({...args,hipMainSha256:'a'.repeat(64)}),/mismatch/);
  assert.throws(()=>extendHipMain({...args,expectedCount:2}),/record count/);
});
