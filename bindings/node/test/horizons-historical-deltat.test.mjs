// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {parseHistoricalDeltaT}=createRequire(import.meta.url)('../scripts/compare-horizons-historical-deltat.cjs');
const raw=`Target body name: Sun (10)
Center body name: Earth (399)
Date_________JDTT, , , TDB-UT,
$$SOE
1721242.000000000, , , 10476.276268,
$$EOE
Prior to 1962, the difference is with
respect to UT1`;
test('parses historical TT epochs and UT1 values, and rejects wrong scales',()=>{
  assert.deepEqual(parseHistoricalDeltaT(raw,[1721242]),[10476.276268]);
  assert.throws(()=>parseHistoricalDeltaT(raw.replace('respect to UT1','respect to UTC'),[1721242]));
  assert.throws(()=>parseHistoricalDeltaT(raw,[1721243]));
  assert.throws(()=>parseHistoricalDeltaT(raw.replace('10476.276268','n.a.'),[1721242]));
  assert.throws(()=>parseHistoricalDeltaT(raw.replace('$$EOE','$$EOE\n$$EOE'),[1721242]));
});
