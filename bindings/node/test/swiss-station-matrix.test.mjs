// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {nearestBrackets,stats}=createRequire(import.meta.url)('../scripts/compare-swiss-station-matrix.cjs');
test('finds nearest entering and leaving retrograde crossings',()=>{
  const b=nearestBrackets(x=>Math.cos(x),0.1,5,1);
  assert.ok(b.retrograde[0]<Math.PI/2&&b.retrograde[1]>Math.PI/2);
  assert.ok(b.direct[0]<-Math.PI/2&&b.direct[1]>-Math.PI/2);
  assert.equal(nearestBrackets(()=>1,0,5,1).direct,null);
  assert.throws(()=>nearestBrackets(()=>NaN,0,5,1));
});
test('reports absolute percentile distribution',()=>{
  assert.deepEqual(stats([-4,1,2,3]),{count:4,median:2,p95:4,p99:4,max:4});
  assert.equal(stats([]),null);
});
