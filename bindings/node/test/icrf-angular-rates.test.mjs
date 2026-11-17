// SPDX-License-Identifier: CPAL-1.0. Synthetic kinematic fixtures.
import {test} from 'node:test';import assert from 'node:assert/strict';import {createRequire} from 'node:module';
const require=createRequire(import.meta.url),{icrfAngularRates,icrfAngles,icrfFiniteRates}=require('../scripts/icrf-angular-rates.cjs');
const {parseIcrfRatesTable}=require('../scripts/horizons-icrf-parser.cjs');
const raw=['Target body name: Sun (10) {source: DE441}','Center body name: Earth (399) {source: DE441}',
  'Date_________JDTT, , RA_(ICRF-a-app), DEC_(ICRF-a-app), I_dRA*cosD, I_d(DEC)/dt,',
  '$$SOE','2451545.000000000, , 10, -20, 12.345, -5.678,','$$EOE'].join('\n');
test('extracts Horizons ICRF rate columns and rejects missing or nonfinite values',()=>{
  assert.deepEqual(parseIcrfRatesTable(raw,'Sun','10',[2451545]),[{raDeg:10,decDeg:-20,raCosDecArcsecHour:12.345,decArcsecHour:-5.678}]);
  assert.throws(()=>parseIcrfRatesTable(raw.replace('12.345','NaN'),'Sun','10',[2451545]),/rates/);
  assert.throws(()=>parseIcrfRatesTable(raw.replace('I_dRA*cosD','missing'),'Sun','10',[2451545]),/rate columns/);
});
test('finite differences cross the 0/360 seam and apply the declination factor',()=>{
  assert.deepEqual(icrfAngles([1,0,0]),{raDeg:0,decDeg:0});
  const rates=icrfFiniteRates({raDeg:359,decDeg:59},{raDeg:0,decDeg:60},{raDeg:1,decDeg:61},1);
  assert.ok(Math.abs(rates.raCosDecArcsecHour-150)<1e-10);
  assert.ok(Math.abs(rates.decArcsecHour-300)<1e-10);
  assert.throws(()=>icrfFiniteRates({raDeg:1,decDeg:0},{raDeg:1,decDeg:0},{raDeg:2,decDeg:0},0),/invalid/);
});
test('converts Cartesian daily velocities to tangent-plane angular rates per hour',()=>{
  const f=180/Math.PI*3600/24,got=icrfAngularRates([1,0,0],[0,2,3]);
  assert.ok(Math.abs(got.raCosDecArcsecHour-2*f)<1e-10);
  assert.ok(Math.abs(got.decArcsecHour-3*f)<1e-10);
  const spherical=icrfAngularRates([1,0,1],[1,0,-1]);
  assert.ok(Math.abs(spherical.decArcsecHour+f)<1e-10);
  assert.throws(()=>icrfAngularRates([0,0,1],[1,1,1]),/undefined/);
  assert.throws(()=>icrfAngularRates([1,0,0],[NaN,0,0]),/invalid/);
});
