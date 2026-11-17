// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {extrapolatedRate}=createRequire(import.meta.url)('../scripts/compare-swiss-seasonal-speeds.cjs');
test('fourth-order difference recovers cubic derivative and wraps longitude',()=>{
  const f=t=>2+3*t-5*t*t+7*t*t*t,h=0.02;
  assert.ok(Math.abs(extrapolatedRate(f(-2*h),f(-h),f(h),f(2*h),0,h)-3)<1e-11);
  assert.ok(Math.abs(extrapolatedRate(359.6,359.8,0.2,0.4,0,1,true)-0.2)<1e-12);
  assert.throws(()=>extrapolatedRate(1,2,3,4,2451545,Number.MIN_VALUE));
  assert.throws(()=>extrapolatedRate(1,2,3,4,2451545,0));
});
