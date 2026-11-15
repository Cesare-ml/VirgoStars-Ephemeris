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
import {mkdtempSync,writeFileSync,unlinkSync,rmdirSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {calculateExperimentalChart} from '../examples/experimental-chart.mjs';

test('chart demo rejects incomplete and ambiguous requests',async()=>{
  await assert.rejects(calculateExperimentalChart({}),/Finite/);
  await assert.rejects(calculateExperimentalChart({ut1JulianDay:2451545,ttMinusUt1Seconds:0,tdbMinusTtSeconds:0,latitudeDeg:0,longitudeDegEast:0,bodies:[0,0],starNames:['A']}),/Unique/);
});

const keys=['VIRGOSTARS_EPHEMERIS_CLI','DE441_PATH','MARS_SPK_PATH','VIRGOSTARS_HOUSE_SET_MODULE','VIRGOSTARS_PLACIDUS_POSITION_MODULE'];
test('real CLI composes calendar, physical planets, houses, stars and warnings',{
  skip:keys.some(key=>!process.env[key]),
},async()=>{
  const config={executablePath:process.env.VIRGOSTARS_EPHEMERIS_CLI,kernelPath:process.env.DE441_PATH,
    physicalCenterKernels:{Mars:process.env.MARS_SPK_PATH},houseSetModulePath:process.env.VIRGOSTARS_HOUSE_SET_MODULE,
    placidusPositionModulePath:process.env.VIRGOSTARS_PLACIDUS_POSITION_MODULE,
    ut1JulianDay:2461354.5,ttMinusUt1Seconds:69.184,tdbMinusTtSeconds:-.0014,datetimeUtc:'2026-11-10T00:00:00Z',latitudeDeg:41.9028,longitudeDegEast:12.4964,houseSystem:'P',bodies:[0,1,4],
    planetSolarDeflection:'POINT_MASS',starSolarDeflection:'POINT_MASS',starNames:['Example'],
    provenance:{engineVersion:'test-engine',kernelSetId:'test-kernels',timeModel:'TEST_UT1_EQUALS_UTC',corrections:'CALLER_ASSERTED'},
    catalog:{id:'synthetic-fixture',records:[{name:'Example',aliases:[],epochTdbJulianDay:2451545,rightAscensionDeg:20,declinationDeg:-30,properMotionRaCosDecMasPerYear:120,properMotionDecMasPerYear:-40,parallaxMas:50,radialVelocityKmPerSecond:20,visualMagnitude:1.23}]}};
  const direct=await calculateExperimentalChart(config);
  assert.equal(direct.contract,'ExperimentalChartDemo');assert.equal(direct.positions.length,3);assert.equal(direct.houses.house.length,12);
  assert.equal(direct.calendar.year,2026);assert.equal(direct.stars[0].magnitude.magnitude,1.23);
  assert.ok(direct.positions[2].position.warnings.includes('PHYSICAL_CENTER_SPK_EXPLICIT'));
  assert.ok(direct.stars[0].position.warnings.includes('SOLAR_DEFLECTION_POINT_MASS_APPLIED'));
  assert.ok(direct.positions.every(item=>item.house.housePosition>=1 && item.house.housePosition<13));
  const dir=mkdtempSync(join(tmpdir(),'virgostars-demo-')),path=join(dir,'config.json');
  try{
    writeFileSync(path,JSON.stringify(config));
    const script=fileURLToPath(new URL('../examples/experimental-chart.mjs',import.meta.url));
    const child=spawnSync(process.execPath,[script,path],{encoding:'utf8',timeout:30000});
    assert.equal(child.status,0,child.stderr);
    const output=JSON.parse(child.stdout);
    assert.equal(output.contract,direct.contract);assert.equal(output.positions.length,3);
    assert.ok(output.houses.warnings.includes('NOT_SWISS_PARITY'));
    assert.equal(JSON.stringify(output).includes(process.env.DE441_PATH),false);
  }finally{try{unlinkSync(path);}finally{rmdirSync(dir);}}
});
