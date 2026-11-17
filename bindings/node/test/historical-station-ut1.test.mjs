// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url),{julday}=require('../index.js');
const {decimalGregorian,deltaAtTt,ut1DifferenceSeconds,oldIllustrativeSigma}=require('../scripts/compare-historical-station-ut1.cjs');
test('fixed TT gives earlier UT1 for larger Delta T',()=>{
  assert.equal(ut1DifferenceSeconds(120,100),-20);assert.equal(ut1DifferenceSeconds(100,120),20);
  const jd=julday(1000,7,1,12,true),r=deltaAtTt(jd,()=>100);
  assert.equal(r.seconds,100);assert.ok(r.decimalYearUt1<decimalGregorian(jd));
  assert.throws(()=>deltaAtTt(NaN,()=>0));assert.throws(()=>ut1DifferenceSeconds(Infinity,1));
});
test('old illustrative uncertainty is bounded to its stated historical range',()=>{
  assert.ok(Math.abs(oldIllustrativeSigma(500)-139.392)<1e-9);
  assert.ok(Math.abs(oldIllustrativeSigma(1000)-53.792)<1e-9);
  assert.equal(oldIllustrativeSigma(1201),null);assert.throws(()=>oldIllustrativeSigma(NaN));
});
