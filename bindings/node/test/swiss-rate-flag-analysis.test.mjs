// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {projectedDelayAcceleration,regression}=createRequire(import.meta.url)('../scripts/audit-swiss-apparent-rate-flags.cjs');
test('projected acceleration has correct sign and unit scaling',()=>{
  const au=149597870.7,c=299792.458*86400;
  assert.ok(Math.abs(projectedDelayAcceleration([au,0,0],[2,0,0],[0,0,0],1)-2/c)<1e-20);
  assert.ok(Math.abs(projectedDelayAcceleration([-au,0,0],[2,0,0],[0,0,0],1)+2/c)<1e-20);
  assert.throws(()=>projectedDelayAcceleration([0,0,0],[0,0,0],[0,0,0],1));
  assert.throws(()=>projectedDelayAcceleration([1,0,0],[0,0,0],[0,0,0],0));
});
test('regression of exact synthetic prediction gives unit slope and zero error',()=>{
  const row=x=>({predictionAuDay:x,modes:{apparent:{residual:[0,0,x]}}});
  const r=regression([row(2),row(-1)]);assert.equal(r.slopeThroughOrigin,1);assert.equal(r.predictionErrorRmsAuDay,0);
  assert.throws(()=>regression([row(0)]));
});
