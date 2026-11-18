// SPDX-License-Identifier: CPAL-1.0. Synthetic tangent-rate checks.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {fourthRate,tangentRates}=createRequire(import.meta.url)('../scripts/compare-horizons-near-sun-rates.cjs');
test('fourth-order unit-vector derivative preserves tangent speeds across wrap',()=>{
  const w=0.1,h=1/64,f=t=>[Math.cos(w*t),Math.sin(w*t),0],rates=tangentRates(fourthRate([-2*h,-h,h,2*h].map(f),h),0,0);
  assert.ok(Math.abs(rates[0]-w*180/Math.PI*3600)<1e-7);
  assert.ok(Math.abs(rates[1])<1e-10);
  assert.throws(()=>fourthRate([f(0)],h),/invalid/);
});
