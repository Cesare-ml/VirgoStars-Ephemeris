// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {separationDegrees,summarize}=createRequire(import.meta.url)('../scripts/analyze-rate-solar-separation.cjs');
test('great-circle separation handles longitude wrap, poles and antipodes',()=>{
  assert.ok(Math.abs(separationDegrees({raDeg:359,decDeg:0},{raDeg:1,decDeg:0})-2)<1e-12);
  assert.ok(Math.abs(separationDegrees({raDeg:0,decDeg:90},{raDeg:180,decDeg:90}))<1e-12);
  assert.ok(Math.abs(separationDegrees({raDeg:0,decDeg:0},{raDeg:180,decDeg:0})-180)<1e-12);
  assert.throws(()=>separationDegrees({raDeg:0,decDeg:91},{raDeg:0,decDeg:0}));
  assert.throws(()=>summarize({records:[]},{output:[]}));
});
