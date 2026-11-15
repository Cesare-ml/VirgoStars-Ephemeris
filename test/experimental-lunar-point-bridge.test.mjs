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
import {dirname,join} from 'node:path';
import {tmpdir} from 'node:os';
import {createExperimentalLegacyBridge} from '../src/experimental-legacy-bridge.mjs';

const root=tmpdir();
const basic={executablePath:join(root,'unused'),kernelPath:join(root,'unused'),ephemerisDirectory:root,flagPolicy:'MAP_SWISS_AND_DEFAULT_TO_JPL',defaultCalendar:'GREGORIAN',mutableResults:'FROZEN',convertUt1ToTtTdb:jd=>({ttJulianDay:jd,tdbJulianDay:jd})};
test('lunar point support is opt-in and unsupported frames fail explicitly',()=>{
  const off=createExperimentalLegacyBridge(basic);
  assert.throws(()=>off.swe_calc_ut(2451545,off.SE_MEAN_NODE,off.SEFLG_JPLEPH),/opt-in/);
  assert.throws(()=>createExperimentalLegacyBridge({...basic,lunarPointPolicy:'AUTO'}),/policy/);
  const on=createExperimentalLegacyBridge({...basic,lunarPointPolicy:'EXPERIMENTAL_GEOCENTRIC'});
  assert.equal(on.swe_get_planet_name(on.SE_MEAN_NODE).name,'mean Node');
  assert.throws(()=>on.swe_calc_ut(2451545,on.SE_MEAN_NODE,on.SEFLG_SWIEPH|on.SEFLG_HELCTR),/geocentric/);
  assert.throws(()=>on.swe_calc_ut(2451545,on.SE_TRUE_NODE,on.SEFLG_SWIEPH|on.SEFLG_EQUATORIAL),/geocentric/);
});

test('explicit epochs expose four existing geocentric lunar models with labelled rates',{
  skip:['VIRGOSTARS_EPHEMERIS_CLI','DE441_PATH'].some(key=>!process.env[key]),
},()=>{
  const api=createExperimentalLegacyBridge({...basic,executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,ephemerisDirectory:dirname(process.env.DE441_PATH),lunarPointPolicy:'EXPERIMENTAL_GEOCENTRIC',convertUt1ToTtTdb:()=>({ttJulianDay:2461354.500800741,tdbJulianDay:2461354.500800725})});
  // Two-epoch Swiss comparisons already documented in the existing Rust tests;
  // this single sample tests the bridge, not independent scientific accuracy.
  const fixtures=[[10,'mean Node',325.5968857875848,0,0.0025695552898,-0.05293949325348264],[11,'true Node',326.0305909519062,0,0.0026645118450,-0.24050703343076724],[12,'mean Apogee',276.2819455302067,-3.9062359911988915,0.0027106251317,0.11151843478988532],[13,'osc. Apogee',278.40526692736915,-3.693729929709786,0.0027057726868,1.477587673426814]];
  for(const [id,name,expected,latitude,distance,speed] of fixtures){
    let called;
    const value=api.swe_calc_ut(2461354.5,id,api.SEFLG_SWIEPH|api.SEFLG_SPEED,v=>{called=v;});
    assert.equal(called,value);assert.equal(api.swe_get_planet_name(id).name,name);
    assert.ok(Math.abs(value.longitude-expected)<.002);
    assert.ok(Math.abs(value.latitude-latitude)<.001);assert.ok(Math.abs(value.distance-distance)<1e-7);
    assert.ok(Math.abs(value.longitudeSpeed-speed)<.0002);
    assert.equal(value.rflag,api.SEFLG_JPLEPH|api.SEFLG_SPEED);
    assert.equal(value.model,'EXPERIMENTAL_GEOCENTRIC_LUNAR_POINT');
    assert.ok(value.warnings.includes('GEOCENTRIC_MATHEMATICAL_POINT'));
    assert.ok(value.warnings.includes('LEGACY_SWIEPH_REQUEST_REPLACED_BY_JPL'));
    const noSpeed=api.swe_calc_ut(2461354.5,id,api.SEFLG_JPLEPH);
    assert.equal(noSpeed.longitudeSpeed,0);assert.equal(noSpeed.latitudeSpeed,0);assert.equal(noSpeed.distanceSpeed,0);
  }
});
