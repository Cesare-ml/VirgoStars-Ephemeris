// SPDX-License-Identifier: CPAL-1.0. Synthetic first-order fit tests.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {fitSmallRotation}=createRequire(import.meta.url)('../scripts/analyze-horizons-frame-rotation.cjs');
test('fits a small known rotation and rejects degenerate geometry',()=>{
  const w=1e-4,c=Math.cos(w),s=Math.sin(w),from=[[1,0,0],[0,1,0],[0,0,1]],to=[[c,s,0],[-s,c,0],[0,0,1]],fit=fitSmallRotation(from,to);
  assert.ok(Math.abs(fit.rotationArcsec[2]-w*180/Math.PI*3600)<1e-6);
  assert.ok(fit.rmsResidualArcsec<1e-6);
  assert.throws(()=>fitSmallRotation([[1,0,0],[1,0,0]],[[1,0,0],[1,0,0]]),/ill-conditioned/);
});
