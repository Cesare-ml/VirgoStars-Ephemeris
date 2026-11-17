// SPDX-License-Identifier: CPAL-1.0. Synthetic coordinate checks only.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {unit,effectiveObliquity,parse}=createRequire(import.meta.url)('../scripts/compare-horizons-orientation-components.cjs');
function lonlat(v){return [Math.atan2(v[1],v[0])*180/Math.PI,Math.atan2(v[2],Math.hypot(v[0],v[1]))*180/Math.PI];}
test('obliquity inference recovers an independent rotation and rejects singular input',()=>{
  const a=unit(123,18),eps=23.4*Math.PI/180,c=Math.cos(eps),s=Math.sin(eps);
  const b=[a[0],c*a[1]+s*a[2],-s*a[1]+c*a[2]], [lon,lat]=lonlat(b);
  const result=effectiveObliquity(123,18,lon,lat);
  assert.ok(Math.abs(result.degrees-23.4)<1e-12);
  assert.ok(Math.abs(result.xMismatchArcsec)<1e-9);
  assert.throws(()=>effectiveObliquity(0,0,0,0),/ill-conditioned/);
});

test('multi-frame parser checks source, epochs and coordinate columns',()=>{
  const raw=`Target body name: Sun (10) {source: DE441}
Center body name: Earth (399) {source: DE441}
Atmos refraction: NO (AIRLESS)
Date_________JDTT,R.A.__(a-app),DEC___(a-app),ObsEcLon,ObsEcLat,RA_(ICRF-a-app),DEC_(ICRF-a-app)
$$SOE
2451545,123,18,120,2,122,19
$$EOE`;
  const row=parse(raw,'Sun','10',[2451545])[0];
  assert.equal(row.ra,123);
  assert.equal(row.raDeg,122);
  assert.throws(()=>parse(raw,'Sun','10',[2451546]),/epoch/);
  assert.throws(()=>parse(raw.replace('NO (AIRLESS)','YES'),'Sun','10',[2451545]),/refraction/);
  assert.throws(()=>parse(raw.replace('ObsEcLat','absent'),'Sun','10',[2451545]),/columns/);
});
