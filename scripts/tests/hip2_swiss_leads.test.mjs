// SPDX-License-Identifier: CPAL-1.0. Synthetic records; no external data.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { findLeads } from '../audit_hip2_swiss_unmatched.mjs';
import { extendHipLeads } from '../extend_star_manifest_hip2_leads.mjs';
const sha=s=>createHash('sha256').update(s).digest('hex');
function hip(id,ra,dec) {
  const chars=Array(276).fill(' ');
  for (const [start,end,value] of [[0,6,String(id)],[15,28,ra.toFixed(10)],[29,42,dec.toFixed(10)],[51,59,'0.00'],[60,68,'0.00']]) {
    const v=value.padStart(end-start); for(let i=start;i<end;i++) chars[i]=v[i-start];
  }
  return chars.join('');
}
const swiss='# sefstars.txt\nAlpha,A1,ICRS,01,00,00,+30,00,00,0,0,0,1,2,0,0\n';
const hipText=hip(2,Math.PI/12,Math.PI/6)+'\n';
const nec='HIP,Name,RA2000,DE2000,Vmag\nHIP 2,Alpha,15,30,2.02\n';
const manifest={schema:'virgostars-star-manifest/1',reviewer:'fixture',astrometrySource:'fixture',dataUseDecision:'fixture',qualityPolicy:'traditional',
  sourceHashes:{swissCatalogSha256:sha(swiss),hip2Sha256:sha(hipText),necSha256:sha(nec)},
  stars:[{name:'Existing',aliases:[],hip:1,qualityTier:'strict',identitySource:'fixture',qualityDecision:'fixture',magnitude:1,magnitudeSource:'fixture',radialVelocityKmS:null,radialVelocityMissingReason:'unknown'}]};
const inspect=(_path,hips)=>hips.map(hip=>({hip,strictEligible:false,traditionalEligible:true}));
test('unique positional lead requires independent photometry and explicit quality',()=>{
  const lead=findLeads(manifest,swiss,hipText).leads[0];
  assert.equal(lead.matches.length,1); assert.equal(lead.matches[0].hip,2);
  const result=extendHipLeads({manifest,swissText:swiss,hipText,necText:nec,hipPath:'fixture',inspect});
  assert.equal(result.manifest.stars.length,2);
  assert.deepEqual(result.manifest.stars[1].aliases,['Alpha']);
  assert.equal(result.manifest.stars[1].qualityTier,'traditional');
  assert.equal(manifest.stars.length,1);
});
test('rejects changed sources and missing or inconsistent quality results',()=>{
  const args={manifest,swissText:swiss,hipText,necText:nec,hipPath:'fixture',inspect};
  assert.throws(()=>extendHipLeads({...args,necText:nec+' '}),/source hash/);
  assert.throws(()=>extendHipLeads({...args,inspect:()=>[]}),/bad quality/);
  assert.throws(()=>extendHipLeads({...args,inspect:()=>[{hip:2,strictEligible:true}]}),/bad quality/);
});
test('ambiguous nearby HIP records do not become leads',()=>{
  const result=findLeads(manifest,swiss,hipText+hip(3,Math.PI/12,Math.PI/6)+'\n');
  assert.equal(result.leads[0].matches.length,2);
});
test('experimental selection is explicit and independent photometry must be unique',()=>{
  const inspectBroad=(_path,hips)=>hips.map(hip=>({hip,solutionType:15,components:2,strictEligible:false,traditionalEligible:false,experimentalEligible:true}));
  const args={manifest,swissText:swiss,hipText,necText:nec,hipPath:'fixture',inspect:inspectBroad};
  const broad=extendHipLeads({...args,selectionPolicy:'experimental'});
  assert.equal(broad.manifest.stars[1].qualityTier,'experimental');
  assert.equal(broad.manifest.qualityPolicy,'experimental');
  const ambiguousNec=nec+'HIP 2,Other,15.000001,30,2.03\n';
  const ambiguousManifest={...manifest,sourceHashes:{...manifest.sourceHashes,necSha256:sha(ambiguousNec)}};
  assert.equal(extendHipLeads({...args,manifest:ambiguousManifest,necText:ambiguousNec,selectionPolicy:'experimental'}).report.selected.length,0);
});
