// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {checkSwiss,summarize}=createRequire(import.meta.url)('../scripts/audit-swiss-nutation-grid.cjs');

test('Swiss nutation fields and obliquity consistency are validated',()=>{
  const r={longitude:23.5,latitude:23.4,distance:-0.001,longitudeSpeed:0.1,latitudeSpeed:0,distanceSpeed:0,rflag:0};
  const s=checkSwiss(r,2451545);assert.equal(s.psi,-3.6);assert.equal(s.eps,360);
  for(const bad of [null,{...r,rflag:1},{...r,distance:NaN},{...r,longitudeSpeed:0.2},{...r,latitudeSpeed:1},{...r,error:'failure'}])assert.throws(()=>checkSwiss(bad,1),/invalid Swiss/);
});

test('summary preserves signed offsets and bias-subtracted residuals',()=>{
  const r=year=>({jdTt:year+3e6,year,localPsi:-0.000135,localEps:0.000388,aPsi:0,aEps:0,bPsi:0,bEps:0,swissPsi:0,swissEps:0});
  const s=summarize([r(-12000),r(2000)]);
  assert.equal(s.all.measures.localPsi.minArcsec,-0.000135);
  assert.equal(s.all.measures.localEps.maxArcsec,0.000388);
  assert.equal(s.all.measures.localPsiBiasResidual.maxAbsoluteArcsec,0);
  assert.equal(s.preHorizons.cases,1);assert.equal(s.modern.cases,1);
  assert.throws(()=>summarize([{...r(2000),swissPsi:NaN}]),/invalid/);
});
