// SPDX-License-Identifier: CPAL-1.0. Synthetic time-series summary checks.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {seriesSummary}=createRequire(import.meta.url)('../scripts/sample-horizons-frame-stability.cjs');
test('frame series summary retains vector means and maximum deviations',()=>{
  const rows=[{fit:{rotationArcsec:[1,2,3],rmsResidualArcsec:0.1,minPivot:0.5}},{fit:{rotationArcsec:[3,2,3],rmsResidualArcsec:0.2,minPivot:0.3}}];
  const result=seriesSummary(rows,'fit');
  assert.deepEqual(result.meanArcsec,[2,2,3]);
  assert.equal(result.maxDeviationArcsec,1);
  assert.equal(result.maxFitRmsArcsec,0.2);
  assert.equal(result.minPivot,0.3);
  assert.throws(()=>seriesSummary([],'fit'),/empty/);
  assert.throws(()=>seriesSummary([{fit:{rotationArcsec:[NaN,0,0],rmsResidualArcsec:0,minPivot:1}}],'fit'),/invalid/);
});
