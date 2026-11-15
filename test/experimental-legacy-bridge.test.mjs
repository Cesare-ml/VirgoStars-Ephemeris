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
import {tmpdir} from 'node:os';
import {dirname,join,parse} from 'node:path';
import {realpathSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
import {createExperimentalLegacyBridge as create} from '../src/experimental-legacy-bridge.mjs';

test('legacy bridge requires explicit policies and refuses source or path ambiguity',()=>{
  const root=tmpdir();
  const base={ephemerisDirectory:root,executablePath:join(root,'unused-executable'),kernelPath:join(root,'unused-kernel'),
    flagPolicy:'MAP_SWISS_AND_DEFAULT_TO_JPL',defaultCalendar:'GREGORIAN',mutableResults:'LEGACY',convertUt1ToTtTdb:()=>({ttJulianDay:0,tdbJulianDay:0})};
  assert.throws(()=>create({...base,flagPolicy:undefined}),/policy/);
  assert.throws(()=>create({...base,defaultCalendar:undefined}),/calendar/);
  assert.throws(()=>create({...base,mutableResults:undefined}),/mutability/);
  const api=create(base);
  assert.equal(api.swe_set_ephe_path(root),realpathSync(root));
  assert.throws(()=>api.swe_set_ephe_path(parse(root).root),/differs/);
  assert.throws(()=>api.swe_calc_ut(2451545,api.SE_SUN,3),/source flags/);
  assert.throws(()=>api.swe_calc_ut(2451545,api.SE_SUN,4),/source flags/);
  assert.throws(()=>api.swe_calc_ut(2451545,api.SE_SUN,4294967297),/flags/);
});

const required=['VIRGOSTARS_EPHEMERIS_CLI','DE441_PATH','MARS_SPK_PATH','VIRGOSTARS_HOUSE_SET_MODULE'];
test('legacy bridge uses effective JPL flags and supports real app-shaped calls',{
  skip:required.some(key=>!process.env[key]),
},async()=>{
  const directory=dirname(process.env.DE441_PATH);
  const {calculateHouseSet}=await import(pathToFileURL(process.env.VIRGOSTARS_HOUSE_SET_MODULE));
  const api=create({ephemerisDirectory:directory,executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,
    flagPolicy:'MAP_SWISS_AND_DEFAULT_TO_JPL',defaultCalendar:'GREGORIAN',mutableResults:'LEGACY',
    convertUt1ToTtTdb:ut=>({ttJulianDay:ut+.000800741,tdbJulianDay:ut+.000800725}),
    physicalCenterKernels:{Mars:process.env.MARS_SPK_PATH},
    houseSupport:{calculateHouseSet,convertUt1ToUtc:()=> '2026-11-10T00:00:00Z',engineVersion:'test',kernelSetId:'test',timeModel:'test'},
    solarEventSupport:{windowDays:1.5,stepMinutes:10,timeOffsets:'CONSTANT_IN_SEARCH'},
    catalog:{id:'synthetic-fixture',records:[{name:'A',aliases:[],epochTdbJulianDay:2451545,rightAscensionDeg:20,declinationDeg:-30,properMotionRaCosDecMasPerYear:120,properMotionDecMasPerYear:-40,parallaxMas:50,radialVelocityKmPerSecond:20,visualMagnitude:1.23}]}});
  assert.equal(api.swe_set_ephe_path(directory),realpathSync(directory));
  assert.equal(api.swe_julday(2000,1,1,12),2451545);
  const jd=2461354.5;
  let callback;const sun=api.swe_calc_ut(jd,api.SE_SUN,api.SEFLG_SWIEPH|api.SEFLG_SPEED,x=>{callback=x;});
  assert.equal(callback,sun);assert.equal(sun.rflag,api.SEFLG_JPLEPH|api.SEFLG_SPEED);
  assert.equal(sun.requestedFlags,api.SEFLG_SWIEPH|api.SEFLG_SPEED);assert.ok(sun.warnings.includes('LEGACY_SWIEPH_REQUEST_REPLACED_BY_JPL'));
  const defaultSource=api.swe_calc_ut(jd,api.SE_MOON,api.SEFLG_SPEED);
  assert.equal(defaultSource.rflag,api.SEFLG_JPLEPH|api.SEFLG_SPEED);assert.ok(defaultSource.warnings.includes('LEGACY_DEFAULT_SOURCE_REPLACED_BY_JPL'));
  const explicit=api.swe_calc_ut(jd,api.SE_SUN,api.SEFLG_JPLEPH);
  assert.equal(Object.hasOwn(explicit,'requestedFlags'),false);assert.ok(!explicit.warnings.includes('LEGACY_DEFAULT_SOURCE_REPLACED_BY_JPL'));
  const eq=api.swe_calc_ut(jd,api.SE_MARS,api.SEFLG_SWIEPH|api.SEFLG_EQUATORIAL);
  assert.ok(Number.isFinite(eq.rectAscension));assert.equal(eq.rflag,api.SEFLG_JPLEPH|api.SEFLG_EQUATORIAL);
  const star=api.swe_fixstar_ut('A',jd,api.SEFLG_SPEED);
  delete star.name;star.longitude=(star.longitude+180)%360;
  assert.equal(star.name,undefined);assert.ok(star.warnings.includes('LEGACY_DEFAULT_SOURCE_REPLACED_BY_JPL'));assert.equal(star.catalogId,'synthetic-fixture');
  const houses=api.swe_houses(jd,41.9028,12.4964,'P');houses.house[0]=1;houses.ds=2;assert.equal(houses.ds,2);
  const event=api.swe_rise_trans(jd,api.SE_SUN,'',api.SEFLG_SWIEPH,api.SE_CALC_RISE,12.4964,41.9028,0,0,0);
  assert.ok(Number.isFinite(event.transitTime));assert.equal(event.effectiveFlags,api.SEFLG_JPLEPH);assert.ok(event.warnings.includes('LEGACY_SWIEPH_REQUEST_REPLACED_BY_JPL'));
  assert.equal(Object.keys(event).includes('effectiveFlags'),false);
  const special=api.swe_calc_ut(jd,api.SE_ECL_NUT,0);assert.equal(special.rflag,0);
  assert.throws(()=>api.swe_calc_ut(jd,api.SE_SUN,api.SEFLG_SWIEPH,'bad callback'),/Callback/);
});
