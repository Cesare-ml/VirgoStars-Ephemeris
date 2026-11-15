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

/** Offline one-instant composition example, added 2026-11-15. Not production. */
import {readFile} from 'node:fs/promises';
import {isAbsolute} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createExperimentalSwissSubset} from '../src/experimental-swiss-subset.mjs';

function requirePath(path,label){if(typeof path!=='string' || !isAbsolute(path))throw new TypeError(`${label} must be an explicit absolute path`);return path;}
function requireText(value,label){if(typeof value!=='string' || !value.trim() || value!==value.trim())throw new TypeError(`${label} must be explicit text`);return value;}

/** Compute only one caller-specified instant; no UTC/UT1 or ΔT is inferred. */
export async function calculateExperimentalChart(config){
  if(!config || typeof config!=='object' || Array.isArray(config))throw new TypeError('Explicit chart configuration required');
  const {ut1JulianDay,ttMinusUt1Seconds,tdbMinusTtSeconds,datetimeUtc,latitudeDeg,longitudeDegEast,houseSystem,bodies,catalog,starNames,provenance}=config;
  if(![ut1JulianDay,ttMinusUt1Seconds,tdbMinusTtSeconds,latitudeDeg,longitudeDegEast].every(Number.isFinite))throw new TypeError('Finite instant, time offsets and observer are required');
  if(!Array.isArray(bodies) || bodies.length===0 || !bodies.every(n=>Number.isInteger(n)&&n>=0&&n<=9) || new Set(bodies).size!==bodies.length)throw new TypeError('Unique requested planet IDs 0–9 are required');
  if(!Array.isArray(starNames) || starNames.length===0 || !starNames.every(name=>typeof name==='string'&&name.trim()===name&&name.length>0) || new Set(starNames.map(name=>name.toLowerCase())).size!==starNames.length)throw new TypeError('Unique explicit star names are required');
  if(houseSystem!=='P')throw new RangeError('This example uses Placidus P to demonstrate house positions');
  if(config.physicalCenterKernels!==undefined){
    if(!config.physicalCenterKernels || typeof config.physicalCenterKernels!=='object' || Array.isArray(config.physicalCenterKernels))throw new TypeError('Physical-center kernel paths must be an object');
    for(const [name,path] of Object.entries(config.physicalCenterKernels))requirePath(path,`${name} kernel path`);
  }
  if(!provenance || typeof provenance!=='object')throw new TypeError('Explicit source identifiers required');
  const {engineVersion,kernelSetId,timeModel,corrections}=provenance;
  for(const [key,value] of Object.entries({engineVersion,kernelSetId,timeModel,corrections}))requireText(value,key);
  const setModule=await import(pathToFileURL(requirePath(config.houseSetModulePath,'houseSetModulePath')).href);
  const pointModule=await import(pathToFileURL(requirePath(config.placidusPositionModulePath,'placidusPositionModulePath')).href);
  const convertUt1ToTtTdb=jd=>{
    if(jd!==ut1JulianDay)throw new RangeError('This example accepts only the configured UT1 instant');
    const ttJulianDay=jd+ttMinusUt1Seconds/86400;
    return {ttJulianDay,tdbJulianDay:ttJulianDay+tdbMinusTtSeconds/86400};
  };
  const swe=createExperimentalSwissSubset({
    executablePath:requirePath(config.executablePath,'executablePath'),kernelPath:requirePath(config.kernelPath,'kernelPath'),
    convertUt1ToTtTdb,catalog,physicalCenterKernels:config.physicalCenterKernels,
    planetSolarDeflection:config.planetSolarDeflection,starSolarDeflection:config.starSolarDeflection,
    horizontalSupport:{refraction:'GEOMETRIC'},
    houseSupport:{calculateHouseSet:setModule.calculateHouseSet,calculatePlacidusHousePosition:pointModule.calculatePlacidusHousePosition,
      convertUt1ToUtc:()=>datetimeUtc,engineVersion,kernelSetId,timeModel,
      positionContext:{datetimeUtc,corrections,observerLongitudeDegEast:longitudeDegEast}},
  });
  const date=swe.swe_revjul(ut1JulianDay,swe.SE_GREG_CAL);
  const roundTrip=swe.swe_julday(date.year,date.month,date.day,date.hour,swe.SE_GREG_CAL);
  if(Math.abs(roundTrip-ut1JulianDay)>1e-9)throw new Error('Calendar round trip failed');
  const houses=swe.swe_houses(ut1JulianDay,latitudeDeg,longitudeDegEast,houseSystem);
  const positions=bodies.map(body=>{
    const name=swe.swe_get_planet_name(body).name;
    const position=swe.swe_calc_ut(ut1JulianDay,body,swe.SEFLG_JPLEPH|swe.SEFLG_SPEED);
    const house=swe.swe_houses_pos(houses.armc,latitudeDeg,houses.trueObliquity,'P',position.longitude,position.latitude);
    const horizontal=swe.swe_azalt(ut1JulianDay,swe.SE_ECL2HOR,longitudeDegEast,latitudeDeg,0,0,0,position.longitude,position.latitude,position.distance);
    return {body,name,position,house,horizontal};
  });
  const stars=starNames.map(name=>{
    const position=swe.swe_fixstar_ut(name,ut1JulianDay,swe.SEFLG_JPLEPH|swe.SEFLG_SPEED);
    const magnitude=swe.swe_fixstar_mag(name);
    return {position,magnitude};
  });
  return {contract:'ExperimentalChartDemo',validation:'NOT_PRODUCTION_OR_SWISS_PARITY',instant:{ut1JulianDay,datetimeUtc,ttMinusUt1Seconds,tdbMinusTtSeconds},provenance:{engineVersion,kernelSetId,timeModel,corrections},calendar:date,houses,positions,stars};
}

async function main(){
  if(process.argv.length!==3)throw new Error('Usage: node examples/experimental-chart.mjs <absolute-config.json>');
  const configPath=requirePath(process.argv[2],'config path');
  const config=JSON.parse(await readFile(configPath,'utf8'));
  const result=await calculateExperimentalChart(config);
  process.stdout.write(`${JSON.stringify(result,null,2)}\n`);
}
if(process.argv[1] && import.meta.url===pathToFileURL(process.argv[1]).href)main().catch(error=>{process.stderr.write(`${error.message}\n`);process.exitCode=1;});
