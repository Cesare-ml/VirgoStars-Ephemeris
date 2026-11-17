// SPDX-License-Identifier: CPAL-1.0. Fictional entry and optional caller data.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const {Engine,prepareStarCatalog,createSwissShapedFacade,auditStarCatalogCoverage}=require('../index.js');
const astro={epochTdbJd:2451545,rightAscensionDeg:30,declinationDeg:15,properMotionRaCosDecMasPerYear:10,properMotionDecMasPerYear:-5,parallaxMas:10};
function manifest(){return {schema:'virgostars-star-manifest/2',reviewer:'fixture',astrometrySource:'fixture',dataUseDecision:'fixture',qualityPolicy:'experimental',stars:[
  {name:'Direct',aliases:['Direct alias'],sourceId:'fixture:1',astrometry:astro,astrometrySource:'fictional',qualityTier:'experimental',
    identitySource:'fictional',qualityDecision:'fictional',magnitude:1,magnitudeSource:'fictional',radialVelocityKmS:null,radialVelocityMissingReason:'fictional'}
]};}
test('schema /2 accepts explicit ICRS inputs and rejects ambiguous or invalid astrometry',()=>{
  const input=manifest(),prepared=prepareStarCatalog(input);assert.deepEqual(prepared.hips,[]);
  assert.equal(auditStarCatalogCoverage(input,['Direct','Direct alias']).uniqueMatchedStarCount,1);
  const fake={fixstarExplicitUt:(a,rv,jd,flags)=>({xx:[a.rightAscensionDeg,rv??0,jd,flags,0,0],rflag:1})};
  const facade=createSwissShapedFacade(fake,{starCatalog:prepared.stars});
  assert.equal(facade.swe_fixstar_ut('Direct alias',2451545,0).longitude,30);
  for(const mutate of [
    m=>{m.schema='virgostars-star-manifest/1';},m=>{m.stars[0].hip=1;},m=>{m.stars[0].sourceId='';},
    m=>{m.stars[0].astrometrySource='';},m=>{m.stars[0].qualityTier='strict';},
    m=>{m.stars[0].astrometry={...astro,parallaxMas:0};},m=>{m.stars[0].astrometry={...astro,rightAscensionDeg:360};},
  ]){const m=manifest();mutate(m);assert.throws(()=>prepareStarCatalog(m),e=>e.code==='E_CATALOG');}
});
test('explicit object center retains its warning and reports missing magnitude',()=>{
  const input=manifest(),star=input.stars[0];star.displayName='Fictional cluster center';star.displayNameSource='fixture';
  star.magnitude=null;delete star.magnitudeSource;star.magnitudeMissingReason='integrated magnitude unavailable';
  star.lookupWarnings=[{name:'Direct alias',code:'SWISS_POSITION_MISMATCH',detail:'fictional center differs from reference'}];
  const engine={fixstarExplicitUt:()=>({xx:[30,15,1,0,0,0],rflag:1})};
  const facade=createSwissShapedFacade(engine,{starCatalog:prepareStarCatalog(input).stars});
  const result=facade.swe_fixstar_ut('Direct alias',2451545,0);
  assert.equal(result.name,'Fictional cluster center');assert.equal(result.compatibilityWarning.code,'SWISS_POSITION_MISMATCH');
  let callbackResult;
  const missing=facade.swe_fixstar_mag('Direct alias',result=>{callbackResult=result;});
  assert.equal(missing.code,'E_STAR_MAGNITUDE');assert.equal(callbackResult,missing);
  assert.deepEqual(Object.keys(missing),['error']);
  assert.match(missing.error,/SWISS_POSITION_MISMATCH: fictional center differs from reference/);
  const withoutWarning=facade.swe_fixstar_mag('Direct');
  assert.equal(withoutWarning.code,'E_STAR_MAGNITUDE');
  assert.doesNotMatch(withoutWarning.error,/SWISS_POSITION_MISMATCH/);
});
const {DE441_PATH,HIP2_PATH}=process.env;
test('explicit astrometry follows the same Rust calculation as an equivalent HIP2 record',
  {skip:DE441_PATH&&HIP2_PATH?false:'DE441_PATH and HIP2_PATH required'},()=>{
    const line=readFileSync(HIP2_PATH,'utf8').split(/\r?\n/).find(row=>Number(row.slice(0,6))===32349);
    const explicit={epochTdbJd:2448349.0625,rightAscensionDeg:Number(line.slice(15,28))*180/Math.PI,
      declinationDeg:Number(line.slice(29,42))*180/Math.PI,parallaxMas:Number(line.slice(43,50)),
      properMotionRaCosDecMasPerYear:Number(line.slice(51,59)),properMotionDecMasPerYear:Number(line.slice(60,68))};
    const engine=new Engine(DE441_PATH);engine.loadHipparcos(HIP2_PATH,[32349],'experimental');
    for(const jd of [2415200.5,2451545,2487700.5]){
      const hip=engine.fixstarTdb(32349,null,jd,256),direct=engine.fixstarExplicitTdb(explicit,null,jd,256);
      for(let i=0;i<6;i++)assert.ok(Math.abs(hip.xx[i]-direct.xx[i])<1e-10);
      assert.match(direct.notes.join(' '),/caller-supplied ICRS/);
    }
    assert.throws(()=>engine.fixstarExplicitTdb({...explicit,parallaxMas:0},null,2451545,0),e=>e.code==='E_CATALOG');
  });
