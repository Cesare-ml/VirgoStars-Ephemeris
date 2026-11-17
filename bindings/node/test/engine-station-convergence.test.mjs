// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {signChanges}=createRequire(import.meta.url)('../scripts/audit-engine-station-convergence.cjs');
const {Engine,julday}=createRequire(import.meta.url)('../index.js');
const {nearestBrackets}=createRequire(import.meta.url)('../scripts/compare-swiss-station-matrix.cjs');
const {bisect}=createRequire(import.meta.url)('../scripts/compare-swiss-station-roots.cjs');
const {extrapolatedRate}=createRequire(import.meta.url)('../scripts/compare-swiss-seasonal-speeds.cjs');
test('counts changes through exact zeros and rejects invalid samples',()=>{
  assert.equal(signChanges([1,0,0,-1,-2,0,3]),2);
  assert.equal(signChanges([1,2,0,3]),0);
  assert.throws(()=>signChanges([1,NaN]));assert.throws(()=>signChanges([1]));
});
test('DE441 apparent longitude rates agree with their own position derivatives at sampled stations',
  {skip:process.env.DE441_PATH?false:'DE441_PATH is not set'},()=>{
    const engine=new Engine(process.env.DE441_PATH),seed=julday(2000,7,1,12,true),h=0.02;
    for(let body=2;body<=9;body++){
      const state=jd=>engine.experimentalLongTermApparentTdb(jd,body).xx;
      const speed=jd=>state(jd)[3],brackets=nearestBrackets(speed,seed,500,5);
      for(const kind of ['retrograde','direct']){
        assert.ok(brackets[kind],`missing ${kind} bracket for ${body}`);
        const interval=bisect(speed,...brackets[kind],0.00001),jd=(interval.lo+interval.hi)/2;
        const lon=t=>state(t)[0],fd=extrapolatedRate(lon(jd-2*h),lon(jd-h),lon(jd+h),lon(jd+2*h),jd,h,true);
        assert.ok(Math.abs(speed(jd)-fd)*3600<1e-5,`rate inconsistency for ${body}, ${kind}`);
      }
    }
  });
