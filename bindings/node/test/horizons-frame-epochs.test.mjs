// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const { rotate, fitRotation, heldoutFit, epochGrid, summarize } = createRequire(import.meta.url)('../scripts/sample-horizons-frame-epochs.cjs');
const { fitSmallRotation } = createRequire(import.meta.url)('../scripts/analyze-horizons-frame-rotation.cjs');

test('held-out bodies detect an inconsistency not used to fit the rotation', () => {
  const root = 1 / Math.sqrt(3);
  const from = [[1,0,0], [0,1,0], [0,1,0], [0,0,1], [0,0,1], [root,root,root]];
  const reference = from.map(v => rotate(v, [3,-2,4]));
  const matched = heldoutFit(from, reference);
  assert.ok(matched.heldoutRmsArcsec < 1e-7);
  for (let i=0;i<3;i++) assert.ok(Math.abs(matched.rotationArcsec[i]-[3,-2,4][i]) < 1e-7);
  const inconsistent = reference.map(v=>[...v]);
  inconsistent[1] = rotate(inconsistent[1], [1,0,0]);
  const held = heldoutFit(from, inconsistent);
  assert.ok(held.heldoutResidualArcsec[0] > 0.99);
  assert.ok(held.heldoutRmsArcsec > 0.70);
  assert.ok(held.rmsResidualArcsec < 1e-7);
  assert.throws(() => heldoutFit(from, [...reference.slice(0,5),[NaN,0,0]]), /unit directions/);
  assert.throws(() => rotate([1,0,0],[0,0,Infinity]), /invalid rotation/);
  assert.throws(() => fitRotation([[1,0,0],[0,1,0]],[[-1,0,0],[0,-1,0]]), /diagnostic range/);
  assert.deepEqual(rotate([1,0,0],[0,0,0]),[1,0,0]);
});

test('iterated fit removes linearization residual for a large synthetic rotation', () => {
  const root = 1 / Math.sqrt(3);
  const from = [[1,0,0], [0,1,0], [0,1,0], [0,0,1], [0,0,1], [root,root,root]];
  const reference = from.map(v => rotate(v, [40,-80,190]));
  const train=[0,2,4,5];
  const onePass=fitSmallRotation(train.map(i=>from[i]),train.map(i=>reference[i]));
  const result=heldoutFit(from,reference);
  assert.ok(onePass.rmsResidualArcsec > 0.01);
  assert.ok(result.rmsResidualArcsec < 1e-7);
  assert.ok(result.heldoutRmsArcsec < 1e-7);
  for (let i=0;i<3;i++) assert.ok(Math.abs(result.rotationArcsec[i]-[40,-80,190][i]) < 1e-7);
});

test('quarterly sample grid is ordered and limited to the declared service range', () => {
  const grid = epochGrid();
  assert.equal(grid.length,60);
  assert.equal(grid[0].year,-9997); assert.equal(grid.at(-1).year,3999);
  assert.deepEqual(grid.slice(0,4).map(x=>x.month),[1,4,7,10]);
  assert.ok(grid.every((x,i)=>i===0||x.jdTt>grid[i-1].jdTt));
  assert.throws(()=>summarize([]),/empty/);
  assert.throws(()=>summarize([{year:-9997}]),/wrong count/);
});
