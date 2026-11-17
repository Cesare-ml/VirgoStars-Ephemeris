// SPDX-License-Identifier: CPAL-1.0. Input-contract checks, not Horizons validation.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const { parse } = createRequire(import.meta.url)('../scripts/compare-horizons-ancient-ecliptic.cjs');
const data = `Target body name: Sun (10) {source: DE441}
Center body name: Earth (399) {source: DE441}
Atmos refraction: NO (AIRLESS)
Date_________JDTT,ObsEcLon,ObsEcLat
$$SOE
2451545.0,123.4,-0.005
$$EOE`;
test('Horizons ecliptic parser requires matching frame inputs and epochs',()=>{
  assert.deepEqual(parse(data,[2451545]),[[123.4,-0.005]]);
  assert.throws(()=>parse(data.replace('DE441','DE440'),[2451545]),/ephemeris/);
  assert.throws(()=>parse(data.replace('NO (AIRLESS)','YES'),[2451545]),/refraction/);
  assert.throws(()=>parse(data,[2451546]),/row/);
  assert.throws(()=>parse(data+'\n$$EOE',[2451545]),/table/);
});
