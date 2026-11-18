// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const {ttToTdb,direction,checkCoordinates,checkConsistency,summarize}=createRequire(import.meta.url)('../scripts/validate-horizons-planet-frame-grid.cjs');
const {separation}=createRequire(import.meta.url)('../scripts/compare-horizons-frame-isolation.cjs');

test('fixed-TT conversion and directions reject invalid values',()=>{
  assert.ok(Math.abs((ttToTdb(2451545)-2451545)*86400)<0.002);
  assert.throws(()=>ttToTdb(NaN),/invalid TT/);
  assert.deepEqual(direction([3,4,0]),[0.6,0.8,0]);
  assert.throws(()=>direction([0,0,0]),/invalid Cartesian range/);
  assert.throws(()=>direction([1,NaN,0]),/invalid Cartesian position/);
  assert.ok(separation(direction([3,4,0]),direction([30,40,0]))<1e-8);
});

test('frame decomposition preserves separation and respects triangle bounds',()=>{
  const valid={icrfArcsec:0.01,samePoleArcsec:0.01,sameDirectArcsec:0.01,poleArcsec:200,poleFrameArcsec:200.005,directArcsec:210,directFrameArcsec:209.995};
  assert.doesNotThrow(()=>checkConsistency(valid));
  assert.throws(()=>checkConsistency({...valid,samePoleArcsec:0.1}),/inconsistent/);
  assert.throws(()=>checkConsistency({...valid,poleArcsec:201}),/inconsistent/);
  assert.throws(()=>checkConsistency({...valid,icrfArcsec:NaN}),/invalid/);
});

test('reference angles must be finite and in their coordinate ranges',()=>{
  const row={raDeg:45,decDeg:-30,ra:46,dec:-29,lon:50,lat:3};
  assert.deepEqual(checkCoordinates([row]),[row]);
  assert.throws(()=>checkCoordinates([{...row,raDeg:360}]),/invalid Horizons angles/);
  assert.throws(()=>checkCoordinates([{...row,lat:91}]),/invalid Horizons angles/);
  assert.throws(()=>checkCoordinates([{...row,lon:NaN}]),/invalid Horizons angles/);
});

test('summary retains the body and epoch of each maximum',()=>{
  const rows=Array.from({length:10},(_,id)=>({id,year:0,month:1,icrfArcsec:id+1,poleArcsec:2,directArcsec:3,samePoleArcsec:4,sameDirectArcsec:5}));
  const out=summarize(rows);
  assert.equal(out.cases,10);
  assert.deepEqual(out.all.icrfArcsec,{arcsec:10,year:0,month:1,id:9});
  assert.equal(out.byBody[1].name,'Moon'); assert.equal(out.byBody[1].icrf.arcsec,2);
  assert.throws(()=>summarize([]),/invalid results/);
  assert.throws(()=>summarize(rows.slice(1)),/missing Sun/);
  assert.throws(()=>summarize([{...rows[0],icrfArcsec:NaN}]),/invalid results/);
});
