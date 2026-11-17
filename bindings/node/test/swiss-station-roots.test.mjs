// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {bisect,separation}=createRequire(import.meta.url)('../scripts/compare-swiss-station-roots.cjs');
test('bisects signed retrograde crossing and rejects an unbracketed interval',()=>{
  const b=bisect(x=>0.3-x,0,1,1e-6);assert.ok(b.lo<=0.3&&b.hi>=0.3&&b.hi-b.lo<=1e-6);
  assert.throws(()=>bisect(x=>x*x+1,-1,1,1e-6));
  assert.ok(Math.abs(separation([1,0,0],[0,1,0])-90)<1e-12);
});
