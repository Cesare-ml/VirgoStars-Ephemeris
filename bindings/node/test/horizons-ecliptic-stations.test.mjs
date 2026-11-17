// SPDX-License-Identifier: CPAL-1.0
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {parseEcliptic,estimate}=createRequire(import.meta.url)('../scripts/compare-horizons-ecliptic-stations.cjs');
const raw=`Target body name: Mercury (199) {source: DE441}
Center body name: Earth (399) {source: DE441}
Atmos refraction: NO (AIRLESS)
Date_________JDTT, , , ObsEcLon, ObsEcLat,
$$SOE
1721242.000000000, , , 359.9999999, -2.0,
$$EOE
Observer-centered IAU76/80 ecliptic-of-date`;
test('verifies JPL frame, epoch and numeric values',()=>{
  assert.deepEqual(parseEcliptic(raw,'Mercury','199',[1721242]),[{lon:359.9999999,lat:-2}]);
  assert.throws(()=>parseEcliptic(raw.replace('IAU76/80','ICRF'),'Mercury','199',[1721242]));
  assert.throws(()=>parseEcliptic(raw,'Mercury','199',[1721243]));
  assert.throws(()=>parseEcliptic(raw.replace('359.9999999','359.9999'),'Mercury','199',[1721242]));
  assert.throws(()=>parseEcliptic(raw.replace('-2.0','n.a.'),'Mercury','199',[1721242]));
});
test('estimates signed stationary point through longitude wrap',()=>{
  const f=t=>((359.9+0.2*t+0.5*t*t)%360+360)%360,h=0.1,a=[-2,-1,0,1,2].map(k=>f(k*h));
  const e=estimate(a,0,h);assert.ok(Math.abs(e.rateArcsecDay-720)<1e-7);
  assert.ok(Math.abs(e.curvatureArcsecDay2-3600)<1e-6);assert.ok(Math.abs(e.newtonShiftSeconds+0.2*86400)<1e-5);
  assert.ok(Math.abs(e.assumedRateRoundingBoundArcsecDay-0.0027)<1e-12);
  assert.throws(()=>estimate([1,1,1,1,1],0,h));
});
