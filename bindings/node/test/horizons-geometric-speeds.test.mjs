// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {parseVectors}=createRequire(import.meta.url)('../scripts/compare-horizons-geometric-speeds.cjs');
const fixture=`Target body name: Mars Barycenter (4)             {source: DE441}
Center body name: Earth (399)                     {source: DE441}
Output units    : KM-S
Output type     : GEOMETRIC cartesian states
Reference frame : ICRF
JDTDB, Calendar Date (TDB), X, Y, Z, VX, VY, VZ,
$$SOE
1721242.000000000, B.C. 0001-Jul-03, 1, 2, 3, -4, 5, -6,
$$EOE`;
test('checks frame, center, solution, time and velocity columns',()=>{
  assert.deepEqual(parseVectors(fixture,'Mars Barycenter','4',[1721242]),[[1,2,3,-4,5,-6]]);
  assert.throws(()=>parseVectors(fixture.replace('GEOMETRIC cartesian','APPARENT cartesian'),'Mars Barycenter','4',[1721242]));
  assert.throws(()=>parseVectors(fixture.replace('Earth (399)','Sun (10)'),'Mars Barycenter','4',[1721242]));
  assert.throws(()=>parseVectors(fixture.replace('DE441','DE440'),'Mars Barycenter','4',[1721242]));
  assert.throws(()=>parseVectors(fixture,'Mars Barycenter','4',[1721243]));
  assert.throws(()=>parseVectors(fixture.replace('-4, 5, -6','-4, n.a., -6'),'Mars Barycenter','4',[1721242]));
});
