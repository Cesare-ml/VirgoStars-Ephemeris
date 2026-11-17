// SPDX-License-Identifier: CPAL-1.0. Formula and invalid-input checks only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {parabolas}=createRequire(import.meta.url)('../scripts/audit-ancient-delta-models.cjs');
const {swissAtTt}=createRequire(import.meta.url)('../scripts/compare-horizons-ancient-deltat.cjs');
test('published exploratory parabolas preserve their mathematical anchors',()=>{
  assert.equal(parabolas(1820).espenakMeeus2006Seconds,-20);
  assert.equal(parabolas(1825).stephenson2016Seconds,-320);
  assert.equal(parabolas(1720).espenakMeeus2006Seconds,12);
  assert.equal(parabolas(1725).stephenson2016Seconds,-287.5);
  assert.throws(()=>parabolas(NaN),/finite/);
  assert.throws(()=>parabolas(Infinity),/finite/);
});

test('fixed-TT inversion evaluates Delta T at the implied UT1 epoch',()=>{
  const jd=2451545, swiss={swe_deltat:ut=>({delta:(3600+10*(ut-jd))/86400})};
  const result=swissAtTt(jd,swiss);
  assert.ok(Math.abs(result-3600/(1+10/86400))<1e-5);
  assert.throws(()=>swissAtTt(jd,{swe_deltat:()=>({delta:NaN})}),/invalid/);
});
