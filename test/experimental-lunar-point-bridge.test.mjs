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
  assert.throws(()=>on.swe_calc_ut(2451545,on.SE_MEAN_NODE,on.SEFLG_SWIEPH|on.SEFLG_HELCTR),/sentinel/);
  assert.throws(()=>on.swe_calc_ut(2451545,on.SE_TRUE_NODE,on.SEFLG_SWIEPH|on.SEFLG_EQUATORIAL|on.SEFLG_HELCTR),/geocentric/);
});

test('opt-in lunar heliocentric zero sentinel is identified and preserves flag provenance',()=>{
  assert.throws(()=>createExperimentalLegacyBridge({...basic,lunarHeliocentricPolicy:'ZERO_SENTINEL'}),/requires/);
  const api=createExperimentalLegacyBridge({...basic,lunarPointPolicy:'EXPERIMENTAL_GEOCENTRIC',lunarHeliocentricPolicy:'ZERO_SENTINEL'});
  for(const id of [api.SE_MEAN_NODE,api.SE_TRUE_NODE,api.SE_MEAN_APOG,api.SE_OSCU_APOG]){
    let callback;
    const value=api.swe_calc_ut(2451545,id,api.SEFLG_SWIEPH|api.SEFLG_HELCTR|api.SEFLG_SPEED,v=>{callback=v;});
    assert.equal(value,callback);
    for(const field of ['longitude','latitude','distance','longitudeSpeed','latitudeSpeed','distanceSpeed'])assert.equal(value[field],0);
    assert.equal(value.model,'EXPERIMENTAL_LUNAR_HELIOCENTRIC_ZERO_SENTINEL');
    assert.ok(value.warnings.includes('HELIOCENTRIC_ZERO_SENTINEL_NO_PHYSICAL_POSITION'));
    assert.ok(value.warnings.includes('LEGACY_SWIEPH_REQUEST_REPLACED_BY_JPL'));
    assert.ok(value.warnings.includes('RFLAG_OMITS_SWISS_IMPLICIT_FLAGS'));
    assert.equal(value.rflag,api.SEFLG_JPLEPH|api.SEFLG_HELCTR|api.SEFLG_SPEED);
  }
});

test('explicit epochs expose four existing geocentric lunar models with labelled rates',{
  skip:['VIRGOSTARS_EPHEMERIS_CLI','DE441_PATH'].some(key=>!process.env[key]),
},()=>{
  const api=createExperimentalLegacyBridge({...basic,executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,ephemerisDirectory:dirname(process.env.DE441_PATH),lunarPointPolicy:'EXPERIMENTAL_GEOCENTRIC',convertUt1ToTtTdb:()=>({ttJulianDay:2461354.500800741,tdbJulianDay:2461354.500800725})});
  // Two-epoch Swiss comparisons already documented in the existing Rust tests;
  // this single sample tests the bridge, not independent scientific accuracy.
  const fixtures=[[10,'mean Node',325.5968857875848,0,0.0025695552898,-0.05293949325348264,327.8591356215528,-12.987346076475953,-0.05115879177491181,-0.01782004566127463],[11,'true Node',326.0305909519062,0,0.0026645118450,-0.24050703343076724,328.27797742377686,-12.840941923138532,-0.23212880611319137,-0.08137155313763952],[12,'mean Apogee',276.2819455302067,-3.9062359911988915,0.0027106251317,0.11151843478988532,277.04965467270614,-27.19050579022158,0.12440249583877462,0.01505963994096096],[13,'osc. Apogee',278.40526692736915,-3.693729929709786,0.0027057726868,1.477587673426814,279.4105317679838,-26.857444000994672,1.6419170948150232,0.19676381971495702]];
  for(const [id,name,expected,latitude,distance,speed,ra,dec,raSpeed,decSpeed] of fixtures){
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
    const eq=api.swe_calc_ut(2461354.5,id,api.SEFLG_JPLEPH|api.SEFLG_EQUATORIAL|api.SEFLG_SPEED);
    assert.equal(eq.model,'EXPERIMENTAL_EQUATORIAL_LUNAR_POINT');
    assert.ok(Math.abs(eq.rectAscension-ra)<.003);assert.ok(Math.abs(eq.declination-dec)<.002);
    assert.ok(Math.abs(eq.rectAscensionSpeed-raSpeed)<.0003);assert.ok(Math.abs(eq.declinationSpeed-decSpeed)<.0003);
    assert.ok(eq.warnings.includes('OBLIQUITY_RATE_NUMERICAL'));
    const eqNoSpeed=api.swe_calc_ut(2461354.5,id,api.SEFLG_JPLEPH|api.SEFLG_EQUATORIAL);
    assert.equal(eqNoSpeed.rectAscensionSpeed,0);assert.equal(eqNoSpeed.declinationSpeed,0);
  }
});
