// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {checkedDateReference,inferredObliquity,summarize}=createRequire(import.meta.url)('../scripts/audit-swiss-ancient-frame-decomposition.cjs');

test('of-date reference rejects fallback, wrong frames and invalid coordinates',()=>{
  const jpl=1,eq=2048,j2000=32,icrs=131072,nonut=64,xyz=4096,radians=8192,forbidden=eq|j2000|icrs|nonut|xyz|radians;
  const ecl={longitude:45,latitude:2,rflag:jpl},equ={rectAscension:45,declination:2,rflag:jpl|eq};
  assert.deepEqual(checkedDateReference(ecl,jpl,forbidden,'ecliptic',0,123),[45,2]);
  assert.deepEqual(checkedDateReference(equ,jpl|eq,j2000|icrs|nonut,'equatorial',0,123),[45,2]);
  for(const r of [null,{...ecl,rflag:0},{...ecl,rflag:jpl|j2000},{...ecl,rflag:jpl|nonut},{...ecl,rflag:jpl|xyz},{...ecl,rflag:jpl|radians},{...ecl,latitude:91},{...ecl,longitude:360}])
    assert.throws(()=>checkedDateReference(r,jpl,forbidden,'ecliptic',0,123),/reference unavailable/);
  assert.throws(()=>checkedDateReference(ecl,jpl,0,'unknown',0,123),/invalid reference kind/);
});

test('obliquity inference verifies a known rotation and rejects ill-conditioned geometry',()=>{
  const value=inferredObliquity([90,0],[90,-23.4]);
  assert.ok(Math.abs(value.degrees-23.4)<1e-12);
  assert.ok(Math.abs(value.xMismatchArcsec)<1e-8);
  assert.equal(inferredObliquity([0,0],[0,0]),null);
  assert.throws(()=>inferredObliquity([NaN,0],[0,0]),/invalid obliquity/);
});

test('summaries distinguish epochs and missing obliquity samples',()=>{
  const make=(year,id,missing)=>({year,month:1,id,fixedArcsec:0.001,poleFrameArcsec:id+1,directFrameArcsec:0.01,poleActualArcsec:id+1,directActualArcsec:0.02,
    poleObliquityDifferenceArcsec:missing?null:2,directObliquityDifferenceArcsec:missing?null:0.1,swissEquinoxXMismatchArcsec:missing?null:0});
  const rows=[-11000,2000].flatMap(year=>Array.from({length:10},(_,id)=>make(year,id,year===2000)));
  const s=summarize(rows);
  assert.equal(s.all.cases,20);assert.equal(s.preHorizons.cases,10);assert.equal(s.all.poleFrame.absolute,10);
  assert.equal(s.obliquity.valid,10);assert.equal(s.obliquity.illConditioned,10);
  const missing=summarize(rows.map(r=>({...r,poleObliquityDifferenceArcsec:null,directObliquityDifferenceArcsec:null,swissEquinoxXMismatchArcsec:null})));
  assert.equal(missing.obliquity.valid,0);assert.equal(missing.obliquity.pole,null);
  assert.throws(()=>summarize([{...rows[0],directObliquityDifferenceArcsec:NaN}]),/invalid rows/);
});
