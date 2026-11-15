// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

import test from 'node:test';
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {createExplicitUtcInstant as create} from '../src/experimental-utc-instant.mjs';
import {createExperimentalSwissSubset} from '../src/experimental-swiss-subset.mjs';

test('explicit UTC adapter rejects ambiguous dates and unsupported offset inputs',()=>{
  const base={executablePath:'x',datetimeUtc:'2026-11-10T00:00:00.000Z',ut1MinusUtcSeconds:0,taiMinusUtcSeconds:37,tdbMinusTtSeconds:0,sourceId:'test'};
  assert.throws(()=>create(base),/Absolute/);
  const absolute=process.platform==='win32'?'C:/example.exe':'/example';
  assert.throws(()=>create({...base,executablePath:absolute,datetimeUtc:'2016-12-31T23:59:60.000Z'}),/instant|leap/i);
  assert.throws(()=>create({...base,executablePath:absolute,datetimeUtc:'2026-02-30T00:00:00.000Z'}),/instant/);
  assert.throws(()=>create({...base,executablePath:absolute,datetimeUtc:'2026-11-10T00:00:00Z'}),/milliseconds/);
  assert.throws(()=>create({...base,executablePath:absolute,taiMinusUtcSeconds:0}),/offsets/);
  assert.throws(()=>create({...base,executablePath:absolute,ut1MinusUtcSeconds:1}),/offsets/);
  assert.throws(()=>create({...base,executablePath:absolute,sourceId:''}),/source/);
});

test('real UTC instant adapter matches ERFA anchors and rejects a different instant',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI,
},()=>{
  const executablePath=process.env.VIRGOSTARS_EPHEMERIS_CLI;
  const old=create({executablePath,datetimeUtc:'2016-12-31T23:59:59.000Z',ut1MinusUtcSeconds:-.4087,taiMinusUtcSeconds:36,tdbMinusTtSeconds:0,sourceId:'ERFA fixture'});
  const next=create({executablePath,datetimeUtc:'2017-01-01T00:00:00.000Z',ut1MinusUtcSeconds:.5913,taiMinusUtcSeconds:37,tdbMinusTtSeconds:0,sourceId:'ERFA fixture'});
  assert.ok(Math.abs(old.ut1JulianDay-2457754.499983696)<1e-9);
  assert.ok(Math.abs(old.ttJulianDay-2457754.5007775924)<1e-9);
  assert.ok(Math.abs(next.ttJulianDay-2457754.500800741)<1e-9);
  assert.ok(Math.abs((next.ttJulianDay-old.ttJulianDay)*86400-2)<.0001);
  assert.equal(next.convertUt1ToUtc(next.ut1JulianDay),'2017-01-01T00:00:00.000Z');
  assert.deepEqual(next.convertUt1ToTtTdb(next.ut1JulianDay),{ttJulianDay:next.ttJulianDay,tdbJulianDay:next.tdbJulianDay});
  assert.throws(()=>next.convertUt1ToTtTdb(next.ut1JulianDay+1),/only/);
  assert.ok(next.warnings.includes('OFFSETS_CALLER_SUPPLIED_NOT_VERIFIED'));
  assert.ok(Object.isFrozen(next));
});

test('explicit UTC instant composes with actual Sun and house providers',{
  skip:['VIRGOSTARS_EPHEMERIS_CLI','DE441_PATH','VIRGOSTARS_HOUSE_SET_MODULE'].some(key=>!process.env[key]),
},async()=>{
  const instant=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,datetimeUtc:'2026-11-10T00:00:00.000Z',ut1MinusUtcSeconds:.1,taiMinusUtcSeconds:37,tdbMinusTtSeconds:-.0014,sourceId:'test-only offsets'});
  const {calculateHouseSet}=await import(pathToFileURL(process.env.VIRGOSTARS_HOUSE_SET_MODULE));
  const api=createExperimentalSwissSubset({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,
    convertUt1ToTtTdb:instant.convertUt1ToTtTdb,houseSupport:{calculateHouseSet,convertUt1ToUtc:instant.convertUt1ToUtc,
      engineVersion:'test',kernelSetId:'test',timeModel:'CALLER_SUPPLIED_OFFSETS'}});
  const sun=api.swe_calc_ut(instant.ut1JulianDay,api.SE_SUN,api.SEFLG_JPLEPH|api.SEFLG_SPEED);
  const houses=api.swe_houses(instant.ut1JulianDay,41.9028,12.4964,'P');
  assert.ok(sun.longitude>=0 && sun.longitude<360);assert.equal(houses.house.length,12);
  assert.throws(()=>api.swe_calc_ut(instant.ut1JulianDay+.1,api.SE_SUN,api.SEFLG_JPLEPH),/only/);
});
