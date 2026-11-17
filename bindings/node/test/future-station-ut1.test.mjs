// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {legacyFutureSigma,ut1Offsets}=createRequire(import.meta.url)('../scripts/audit-future-station-ut1.cjs');
test('reproduces the rounded NASA future examples and rejects extrapolation outside scope',()=>{
  assert.equal(legacyFutureSigma(2005),0);
  assert.ok(Math.abs(legacyFutureSigma(2500)-612)<1);
  assert.ok(Math.abs(legacyFutureSigma(3000)-1885)<1);
  assert.throws(()=>legacyFutureSigma(2004));assert.throws(()=>legacyFutureSigma(NaN));
});
test('propagates Delta T offsets with opposite sign at fixed TT',()=>{
  assert.deepEqual(ut1Offsets(100,10),{lowerSeconds:-110,centerSeconds:-100,upperSeconds:-90});
  assert.throws(()=>ut1Offsets(1,-1));assert.throws(()=>ut1Offsets(Infinity,1));
});
