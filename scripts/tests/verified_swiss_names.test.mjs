// SPDX-License-Identifier: CPAL-1.0. Fictional records only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {attachVerifiedNames} from '../attach_verified_swiss_names.mjs';
const sha=b=>createHash('sha256').update(b).digest('hex');
const row=(name,designation,n=3)=>`${name},${designation},ICRS,1,2,${n},4,5,6,7,8,9,10,11,0,0`;
function iauRecord(name,hip){
  const a=Array(140).fill(' '),put=(o,s)=>{for(let i=0;i<s.length;i++)a[o+i]=s[i];};
  put(0,name);put(65,'A');put(81,' 1.20');put(87,'V');put(90,String(hip).padStart(7));put(126,'2020-01-02');
  return a.join('');
}
function fixture({conflict=false,extraSwiss=''}={}){
  const swissBytes=Buffer.from(`# sefstars.txt\n${row('Alias','a1')}\n${extraSwiss}`);
  const iauBytes=Buffer.from(`#Name/ASCII Dec(J2000)\n${iauRecord(conflict?'Alias':'Other',conflict?5:2)}\n`);
  const manifest={schema:'virgostars-star-manifest/1',reviewer:'fixture',astrometrySource:'fixture',dataUseDecision:'fixture',qualityPolicy:'strict',
    sourceHashes:{swissCatalogSha256:sha(swissBytes),iauSha256:sha(iauBytes),hip2Sha256:'a'.repeat(64)},
    stars:[{name:',a1',aliases:[],hip:1,identitySource:'fixture',qualityDecision:'fixture',magnitude:1,magnitudeSource:'fixture',radialVelocityKmS:null,radialVelocityMissingReason:'unknown'}]};
  const manifestBytes=Buffer.from(JSON.stringify(manifest));
  const gap={schema:'virgostars-swiss-name-gap-audit/1',status:'REVIEW_REQUIRED',manifestSha256:sha(manifestBytes),
    swissCatalogSha256:sha(swissBytes),hip2Sha256:'a'.repeat(64),thresholdArcsec:1,epochs:[{year:1950},{year:2000},{year:2024}],
    results:[{name:'Alias',status:'EXISTING_POSITION_REVIEW',matches:[{name:',a1',hip:1}]}]};
  return {manifestBytes,swissBytes,iauBytes,gapBytes:Buffer.from(JSON.stringify(gap))};
}
test('attaches only an exact designation match after the three-epoch diagnostic',()=>{
  const args=fixture(),r=attachVerifiedNames(args);
  assert.deepEqual(r.manifest.stars[0].aliases,['Alias']);
  assert.equal(r.report.additions.length,1);
  assert.deepEqual(JSON.parse(args.manifestBytes).stars[0].aliases,[]);
});
test('withholds IAU conflicts and multiple designations',()=>{
  let r=attachVerifiedNames(fixture({conflict:true}));
  assert.equal(r.report.additions.length,0);assert.equal(r.report.withheld[0].reason,'IAU_CONFLICT_OR_AMBIGUITY');
  r=attachVerifiedNames(fixture({extraSwiss:row('Alias','a2')+'\n'}));
  assert.equal(r.report.additions.length,0);assert.equal(r.report.withheld[0].reason,'MULTIPLE_DESIGNATIONS');
});
test('mixed source rows require an exact match to an already selected named anchor',()=>{
  const args=fixture({extraSwiss:row('Existing','a1')+'\n'+row('Other','a1',8)+'\n'});
  const manifest=JSON.parse(args.manifestBytes);manifest.stars[0].aliases=['Existing'];
  const manifestBytes=Buffer.from(JSON.stringify(manifest));
  const gap=JSON.parse(args.gapBytes);gap.manifestSha256=sha(manifestBytes);
  const adjusted={...args,manifestBytes,gapBytes:Buffer.from(JSON.stringify(gap))};
  assert.equal(attachVerifiedNames(adjusted).report.additions.length,1);
  manifest.stars[0].aliases=[];const noAnchor=Buffer.from(JSON.stringify(manifest));
  gap.manifestSha256=sha(noAnchor);
  assert.equal(attachVerifiedNames({...adjusted,manifestBytes:noAnchor,gapBytes:Buffer.from(JSON.stringify(gap))}).report.withheld[0].reason,'DIFFERENT_ASTROMETRY');
});
test('nearby source rows can identify the same star but not different components',()=>{
  const args=fixture({extraSwiss:row('Existing','a2')+'\n'+row('Alias','a3')+'\n'});
  const manifest=JSON.parse(args.manifestBytes);manifest.stars[0].aliases=['Existing'];
  const manifestBytes=Buffer.from(JSON.stringify(manifest)),gap=JSON.parse(args.gapBytes);gap.manifestSha256=sha(manifestBytes);
  const adjusted={...args,manifestBytes,gapBytes:Buffer.from(JSON.stringify(gap))};
  assert.equal(attachVerifiedNames(adjusted).report.additions.length,1);
  const distant=fixture({extraSwiss:row('Existing','a2',5)+'\n'+row('Alias','a3')+'\n'});
  const distantManifest=JSON.parse(distant.manifestBytes);distantManifest.stars[0].aliases=['Existing'];
  const distantBytes=Buffer.from(JSON.stringify(distantManifest)),dg=JSON.parse(distant.gapBytes);dg.manifestSha256=sha(distantBytes);
  assert.equal(attachVerifiedNames({...distant,manifestBytes:distantBytes,gapBytes:Buffer.from(JSON.stringify(dg))}).report.additions.length,0);
});
test('rejects stale and incomplete diagnostics',()=>{
  const args=fixture();
  assert.throws(()=>attachVerifiedNames({...args,swissBytes:Buffer.from(args.swissBytes+' ')}),/mismatch/);
  const gap=JSON.parse(args.gapBytes);gap.epochs.pop();
  assert.throws(()=>attachVerifiedNames({...args,gapBytes:Buffer.from(JSON.stringify(gap))}),/mismatch/);
});
