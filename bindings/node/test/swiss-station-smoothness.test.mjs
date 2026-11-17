// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {detrend}=createRequire(import.meta.url)('../scripts/audit-swiss-station-smoothness.cjs');
test('linear data has zero residual while perturbations and crossings remain visible',()=>{
  const a=detrend([-2,-1,0,1,2],[-3,-1,1,3,5]);assert.equal(a.rms,0);assert.equal(a.signChanges,1);
  assert.equal(a.slopePerSecond,2);assert.equal(a.interceptAtZero,1);
  const b=detrend([-2,-1,0,1,2],[-3,-1,2,3,5]);assert.ok(b.rms>0);
  assert.throws(()=>detrend([1,1,1],[1,2,3]));assert.throws(()=>detrend([1,2,3],[1,NaN,3]));
});
