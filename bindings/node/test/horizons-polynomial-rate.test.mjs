// SPDX-License-Identifier: CPAL-1.0. Synthetic fitting checks only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {polynomialSlope,tangentOffsets,heldoutRms}=createRequire(import.meta.url)('../scripts/fit-horizons-near-sun-rates.cjs');
test('polynomial fit recovers derivative on scaled sample times',()=>{
  const t=Array.from({length:31},(_,i)=>(i-15)/256),y=t.map(x=>4+3*x-7*x*x+2*x**3-8*x**5+4*x**7),fit=polynomialSlope(t,y,7);
  assert.ok(Math.abs(fit.rate-3)<1e-9);
  assert.ok(fit.rms<1e-12);
  assert.ok(heldoutRms(t,y,7).rms<1e-12);
  assert.ok(Math.abs(heldoutRms(t,y,7).rate-3)<1e-9);
  assert.throws(()=>polynomialSlope([0,0,0],[1,2,3],1),/window/);
  assert.throws(()=>polynomialSlope([1,1,1],[1,2,3],1),/singular/);
  assert.throws(()=>heldoutRms([0,1,2,3],[0,NaN,2,3],1),/invalid/);
});
test('tangent offsets are continuous across right-ascension zero',()=>{
  const values=tangentOffsets([{raDeg:359.999,decDeg:0},{raDeg:0.001,decDeg:0}],{raDeg:0,decDeg:0});
  assert.ok(Math.abs(values[0][0]+3.6)<1e-8);
  assert.ok(Math.abs(values[1][0]-3.6)<1e-8);
});
test('interleaved holdout exposes an underfit seventh-degree signal',()=>{
  const t=Array.from({length:61},(_,i)=>(i-30)/30),y=t.map(x=>2*x+0.01*x**7);
  assert.ok(heldoutRms(t,y,5).rms>1e-5);
  assert.ok(heldoutRms(t,y,7).rms<1e-12);
});
