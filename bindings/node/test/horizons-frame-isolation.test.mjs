// SPDX-License-Identifier: CPAL-1.0. Angular-separation arithmetic only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {unit,separation}=createRequire(import.meta.url)('../scripts/compare-horizons-frame-isolation.cjs');

test('ICRF separation remains stable at RA wrap and microarcsecond scale',()=>{
  assert.ok(separation(unit(0,0),unit(360,0))<1e-9);
  assert.ok(Math.abs(separation(unit(0,0),unit(1/3600000000,0))-1e-6)<1e-12);
  assert.ok(Math.abs(separation(unit(0,0),unit(90,0))-90*3600)<1e-8);
});
