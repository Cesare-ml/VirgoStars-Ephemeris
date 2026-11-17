// SPDX-License-Identifier: CPAL-1.0
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {validationYears}=createRequire(import.meta.url)('../scripts/planet-validation-years.cjs');
test('validation grids include the requested endpoints and reject unknown policies',()=>{
  const grid=validationYears('0-3000-step-50');
  assert.equal(grid.length,61);assert.equal(grid[0],0);assert.equal(grid.at(-1),3000);
  assert.ok(grid.every((year,i)=>year===i*50));
  const dense=validationYears('0-3000-step-10');
  assert.equal(dense.length,301);assert.equal(dense[0],0);assert.equal(dense.at(-1),3000);
  assert.ok(dense.every((year,i)=>year===i*10));
  assert.deepEqual(validationYears(),validationYears('0-5000-step-250'));
  assert.throws(()=>validationYears('other'),/unknown validation grid/);
});
