// SPDX-License-Identifier: CPAL-1.0. Formula and invalid-input checks only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {parabolas,yearsForGrid:ut1Years}=createRequire(import.meta.url)('../scripts/audit-ancient-delta-models.cjs');
const {swissAtTt,yearsForGrid}=createRequire(import.meta.url)('../scripts/compare-horizons-ancient-deltat.cjs');
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

test('dense Horizons grid is ordered, covers both endpoints, and rejects unknown grids',()=>{
  const years=yearsForGrid('dense');
  assert.equal(years.length,39);
  assert.equal(years[0],-9998);
  assert.equal(years.at(-1),-719);
  assert.ok(years.slice(1).every((y,i)=>y>years[i]));
  assert.equal(yearsForGrid('unknown'),null);
});

test('dense UT1 sensitivity grid covers the ancient range in order',()=>{
  const years=ut1Years('dense');
  assert.equal(years.length,26);
  assert.equal(years[0],-12999);
  assert.equal(years.at(-1),-719);
  assert.ok(years.slice(1).every((y,i)=>y>years[i]));
  assert.equal(ut1Years('unknown'),null);
});
