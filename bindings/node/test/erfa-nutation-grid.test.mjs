// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {epochGrid,summarize,checkLinearConformance}=createRequire(import.meta.url)('../scripts/audit-erfa-nutation-grid.cjs');

test('TT grid spans requested epochs, preserves distinct days and includes daily bursts',()=>{
  const g=epochGrid();
  assert.equal(g.length,3887);
  assert.equal(g[0].year,-12999);assert.equal(g.at(-1).year,3999);
  assert.ok(g.some(r=>r.jdTt===2451545&&r.cohorts.includes('seasonal')));
  assert.ok(g.every((r,i)=>i===0||r.jdTt>g[i-1].jdTt));
  for(const year of [-12999,2000,3999]) assert.equal(g.filter(r=>r.year===year&&r.cohorts.includes('daily')).length,365);
});

test('summary distinguishes components and epoch groups without inventing accuracy bounds',()=>{
  const row=(year,localPsi,localEps)=>({year,jdTt:year+3e6,month:1,dayOffset:null,cohorts:['test'],localPsi,localEps,linearPsi:1,linearEps:-1,aPsi:0,aEps:0,bPsi:1,bEps:-1});
  const result=summarize([row(-12000,3,-4),row(2000,0.1,0.2)]);
  assert.equal(result.all.comparisons.localMinusA.longitude.absoluteArcsec,3);
  assert.equal(result.all.comparisons.localMinusA.obliquity.absoluteArcsec,4);
  assert.equal(result.preHorizons.cases,1);assert.equal(result.modern.cases,1);
  assert.equal(result.all.comparisons.bMinusA.obliquity.absoluteArcsec,1);
  assert.equal(result.all.comparisons.linearMinusB.longitude.absoluteArcsec,0);
  assert.doesNotThrow(()=>checkLinearConformance(result));
  assert.throws(()=>checkLinearConformance({all:{comparisons:{linearMinusB:{longitude:{absoluteArcsec:1e-7},obliquity:{absoluteArcsec:0}}}}}),/conformance failed/);
  assert.throws(()=>summarize([]),/invalid/);
  assert.throws(()=>summarize([row(2000,NaN,0)]),/invalid/);
});
