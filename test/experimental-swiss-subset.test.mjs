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
import {createExperimentalSwissSubset as create} from '../src/experimental-swiss-subset.mjs';

test('rejects absent converters, unsupported bodies, flags and incoherent epochs without fallback',()=>{
  assert.throws(()=>create({executablePath:'x',kernelPath:'y'}),TypeError);
  const api=create({executablePath:'x',kernelPath:'y',convertUt1ToTtTdb:()=>({ttJulianDay:0,tdbJulianDay:0})});
  assert.throws(()=>api.swe_calc_ut(2461354.5,api.SE_MARS,api.SEFLG_JPLEPH),/physical centers/);
  assert.throws(()=>api.swe_calc_ut(2461354.5,api.SE_SUN,api.SEFLG_SWIEPH),/flags/);
  assert.throws(()=>api.swe_calc_ut(2461354.5,api.SE_SUN,api.SEFLG_JPLEPH),/same instant/);
  assert.throws(()=>api.swe_julday(2000,1,1,12),/calendar/);
});

test('real DE441 subset supports calendar, physical targets and sync callbacks',{
  skip:!process.env.VIRGOSTARS_EPHEMERIS_CLI || !process.env.DE441_PATH,
},()=>{
  let conversions=0;
  const api=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,
    convertUt1ToTtTdb:ut1=>{conversions++;assert.equal(ut1,2461354.5);return {ttJulianDay:2461354.500800741,tdbJulianDay:2461354.500800725};}});
  assert.equal(api.swe_julday(2000,1,1,12,api.SE_GREG_CAL),2451545);
  assert.equal(api.swe_julday(2000,1,1,12,api.SE_JUL_CAL),2451558);
  assert.deepEqual(api.swe_revjul(2451545,api.SE_GREG_CAL),{year:2000,month:1,day:1,hour:12});
  const refs=[
    [227.60006397597988,-0.00011258772084865995,1.0049844397937353],
    [235.61310848307508,-4.994070016873262,12.279083261707692],
    [216.32320925612183,1.3285538374668426,-0.7035704023233343],
    [203.19241664190434,-2.6211204487140023,-0.16419868548715205],
  ];
  for(let body=0;body<4;body++){
    let callbackResult;
    const result=api.swe_calc_ut(2461354.5,body,api.SEFLG_JPLEPH|api.SEFLG_SPEED,value=>{callbackResult=value;});
    assert.equal(result,callbackResult);
    assert.ok(Math.abs(result.longitude-refs[body][0])<1e-5);
    assert.ok(Math.abs(result.latitude-refs[body][1])<1e-5);
    assert.ok(Math.abs(result.longitudeSpeed-refs[body][2])<5e-5);
    assert.equal(result.rflag,257);
    assert.ok(result.warnings.includes('TDB_DAY_RATES_NOT_EXACT_UT_RATES'));
  }
  const noSpeed=api.swe_calc_ut(2461354.5,api.SE_MOON,api.SEFLG_JPLEPH);
  assert.deepEqual([noSpeed.longitudeSpeed,noSpeed.latitudeSpeed,noSpeed.distanceSpeed],[0,0,0]);
  assert.equal(conversions,5);
});
