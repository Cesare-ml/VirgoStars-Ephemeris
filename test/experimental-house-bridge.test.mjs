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
import {createExperimentalSwissSubset as create} from '../src/experimental-swiss-subset.mjs';

const options={executablePath:'x',kernelPath:'y',convertUt1ToTtTdb:()=>({ttJulianDay:0,tdbJulianDay:0})};
test('house bridge requires explicit providers and context',()=>{
  const api=create(options);
  assert.throws(()=>api.swe_houses(2451545,40,10,'P'),/house support/);
  assert.throws(()=>api.swe_houses_pos(0,40,23,'P',20,0),/position provider/);
  assert.throws(()=>create({...options,houseSupport:{}}),/House provider/);
});

const modules=['VIRGOSTARS_HOUSE_SET_MODULE','VIRGOSTARS_PLACIDUS_POSITION_MODULE'];
test('real Rust angles compose with five house systems and Placidus position',{
  skip:!modules.some(name=>process.env[name]),
},async()=>{
  for(const name of [...modules,'VIRGOSTARS_EPHEMERIS_CLI','DE441_PATH'])assert.ok(process.env[name],`${name} required`);
  const {calculateHouseSet}=await import(pathToFileURL(process.env.VIRGOSTARS_HOUSE_SET_MODULE));
  const {calculatePlacidusHousePosition}=await import(pathToFileURL(process.env.VIRGOSTARS_PLACIDUS_POSITION_MODULE));
  const api=create({executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,
    convertUt1ToTtTdb:()=>({ttJulianDay:2461354.500800741,tdbJulianDay:2461354.500800725}),
    houseSupport:{calculateHouseSet,calculatePlacidusHousePosition,convertUt1ToUtc:()=> '2026-11-10T00:00:00Z',engineVersion:'fixture',kernelSetId:'external-DE441',timeModel:'UT1_EQUALS_UTC_TEST_ASSUMPTION',positionContext:{datetimeUtc:'2026-11-10T00:00:00Z',corrections:'LIGHT_TIME_ANNUAL_ABERRATION',observerLongitudeDegEast:12.4964}}});
  // Recorded Swiss 2.09.03 first and second cusps, Rome, 2026-11-10.
  const refs={V:[142.8264147901,172.8264147901],P:[157.8264147901,181.4426149268],K:[157.8264147901,186.5437676119],R:[157.8264147901,181.3589363779],O:[157.8264147901,186.4436990138]};
  for(const [system,reference] of Object.entries(refs)){
    let cb;const result=api.swe_houses(2461354.5,41.9028,12.4964,system,value=>{cb=value;});
    assert.equal(result,cb);assert.equal(result.house.length,12);assert.ok(Object.isFrozen(result.house));
    for(let i=0;i<2;i++)assert.ok(Math.abs(result.house[i]-reference[i])<0.00001,`${system} ${i+1}`);
    assert.ok(result.warnings.includes('AUXILIARY_ANGLES_OMITTED'));
  }
  const houses=api.swe_houses(2461354.5,41.9028,12.4964,'P');
  let cb;const placement=api.swe_houses_pos(houses.armc,41.9028,houses.trueObliquity,'P',227.5,-2,value=>{cb=value;});
  assert.equal(placement,cb);assert.ok(Math.abs(placement.housePosition-3.5213852131119285)<0.00001);
  assert.throws(()=>api.swe_houses(2461354.5,41.9028,12.4964,'X'),/Only V/);
  assert.throws(()=>api.swe_houses_pos(houses.armc,41.9028,houses.trueObliquity,'V',227.5,-2),/Only Placidus/);
});
