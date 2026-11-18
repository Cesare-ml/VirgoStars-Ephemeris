// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {yearsFor,tdbToTt,checkReference,summarize}=createRequire(import.meta.url)('../scripts/validate-swiss-fixed-icrs-grid.cjs');

test('grids cover requested endpoints and the interleaved dates are distinct',()=>{
  const baseline=yearsFor('baseline'),interleaved=yearsFor('interleaved');
  assert.equal(baseline[0],-12999);assert.equal(baseline.at(-1),3999);
  assert.equal(baseline.length,69);assert.equal(interleaved.length,68);
  assert.ok(interleaved.every(y=>!baseline.includes(y)));
  assert.throws(()=>yearsFor('unknown'),/unknown grid/);
  assert.throws(()=>tdbToTt(Infinity),/invalid TDB/);
  assert.ok(Math.abs((tdbToTt(2451545)-2451545)*86400)<0.002);
});

test('reference validation refuses fallback and missing coordinates',()=>{
  const flags=1|32|2048|131072,ok={rectAscension:1,declination:2,rflag:flags|64};
  assert.equal(checkReference(ok,flags,0,123),ok);
  for(const r of [null,{...ok,rflag:flags&~1},{...ok,rflag:flags&~131072},{...ok,declination:NaN},{...ok,declination:90.1},{...ok,rectAscension:360},{...ok,error:'failure'}])
    assert.throws(()=>checkReference(r,flags,0,123),/reference unavailable/);
});

test('summary separates lunar and pre-Horizons cases',()=>{
  const rows=[-11000,2000].flatMap(year=>Array.from({length:10},(_,id)=>({year,month:1,id,solarElongationDeg:0,differenceArcsec:id===1?(year===2000?2:1):0.1})));
  const s=summarize(rows);
  assert.equal(s.cases,20);assert.equal(s.moon.arcsec,2);assert.equal(s.preHorizons.arcsec,1);assert.equal(s.overlap.arcsec,2);assert.equal(s.byBody[1].cases,2);
  assert.throws(()=>summarize([]),/invalid/);
});
