// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {robustSign,expectedSigns}=createRequire(import.meta.url)('../scripts/compare-horizons-station-signs.cjs');
test('only calls a direction when the rate exceeds the rounding allowance',()=>{
  assert.equal(robustSign(0.1,0.1),0);assert.equal(robustSign(0.11,0.1),1);assert.equal(robustSign(-0.11,0.1),-1);
  assert.deepEqual(expectedSigns('retrograde'),[1,-1]);assert.deepEqual(expectedSigns('direct'),[-1,1]);
  assert.throws(()=>robustSign(NaN,0));assert.throws(()=>robustSign(1,-1));assert.throws(()=>expectedSigns('other'));
});
