// SPDX-License-Identifier: CPAL-1.0. Fictional test records only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {attachReviewedAliases} from '../attach_reviewed_star_aliases.mjs';
import {createRequire} from 'node:module';
const {createSwissShapedFacade,prepareStarCatalog}=createRequire(import.meta.url)('../../bindings/node/index.js');
const sha=b=>createHash('sha256').update(b).digest('hex'),bytes=x=>Buffer.from(JSON.stringify(x));
const swiss=Buffer.from('# sefstars.txt\nAlpha,A1,ICRS,1,2,3,4,5,6,7,8,9,10,11,0,0\nAlternate,A1,ICRS,1,2,3,4,5,6,7,8,9,10,11,0,0\n');
function fixture(){
  const manifest=bytes({schema:'virgostars-star-manifest/1',reviewer:'fixture',astrometrySource:'fixture',dataUseDecision:'fixture',qualityPolicy:'strict',
    sourceHashes:{swissCatalogSha256:sha(swiss)},stars:[{name:'Alpha',aliases:[],hip:1,identitySource:'fixture',qualityDecision:'fixture',magnitude:1,magnitudeSource:'fixture',radialVelocityKmS:null,radialVelocityMissingReason:'fixture'}]});
  return {manifest,decisions:{schema:'virgostars-reviewed-alias-decisions/1',manifestSha256:sha(manifest),swissCatalogSha256:sha(swiss),reviewer:'fixture',aliases:[{hip:1,canonicalName:'Alpha',lookup:'Alternate',sourceUrl:'https://example.org/fixture',rationale:'fictional identity',warning:{code:'SWISS_POSITION_MISMATCH',detail:'fixture mismatch'}}]}};
}
test('applies a verified caller assertion and exposes a lookup-specific warning',()=>{
  const {manifest,decisions}=fixture(),r=attachReviewedAliases(manifest,swiss,bytes(decisions));
  const facade=createSwissShapedFacade({}, {starCatalog:prepareStarCatalog(r.manifest).stars});
  assert.equal(facade.swe_fixstar_mag('alternate').compatibilityWarning.code,'SWISS_POSITION_MISMATCH');
  assert.equal(facade.swe_fixstar_mag('Alpha').compatibilityWarning,undefined);
  assert.equal(r.report.status,'CALLER_ASSERTED');
});
test('rejects stale or conflicting decisions instead of attaching them',()=>{
  const {manifest,decisions}=fixture();
  assert.throws(()=>attachReviewedAliases(manifest,Buffer.from(swiss+' '),bytes(decisions)),/hash mismatch/);
  for(const change of [{lookup:'unknown'},{lookup:'Alpha'},{hip:2},{canonicalName:'Other'},{sourceUrl:'http://example.org/'},{warning:undefined},{warning:{name:'Alpha',code:'SWISS_POSITION_MISMATCH',detail:'misdirected'}}]){
    const bad=structuredClone(decisions);Object.assign(bad.aliases[0],change);
    assert.throws(()=>attachReviewedAliases(manifest,swiss,bytes(bad)));
  }
  decisions.aliases.push({...decisions.aliases[0],lookup:'ALTERNATE'});
  assert.throws(()=>attachReviewedAliases(manifest,swiss,bytes(decisions)),/duplicate/);
});
