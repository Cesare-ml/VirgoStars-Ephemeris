// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {stencilOffsets}=createRequire(import.meta.url)('../scripts/compare-horizons-station-windows.cjs');
test('sorts every stencil and retains both sides without duplicates',()=>{
  const x=stencilOffsets([30,60,120],0.5);assert.equal(x.length,24);
  assert.ok(x.every((v,i)=>i===0||v.offset>x[i-1].offset));
  for(const radius of [30,60,120])for(const side of [-1,1])assert.equal(x.filter(e=>e.radius===radius&&e.side===side).length,4);
  assert.throws(()=>stencilOffsets([30,30],0.5));assert.throws(()=>stencilOffsets([0],0.5));
});
