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

/** Opt-in API-shape experiment. Not a replacement for the Swiss binding. */
import {createExperimentalRustCli} from './experimental-rust-cli.mjs';

export const SE_JUL_CAL=0,SE_GREG_CAL=1,SEFLG_JPLEPH=1,SEFLG_SWIEPH=2,SEFLG_SPEED=256;
export const SE_SUN=0,SE_MOON=1,SE_MERCURY=2,SE_VENUS=3,SE_MARS=4,SE_JUPITER=5,SE_SATURN=6,SE_URANUS=7,SE_NEPTUNE=8,SE_PLUTO=9;
const PHYSICAL_TARGETS=Object.freeze(['Sun','Moon','Mercury','Venus']);
const calendar=flag=>{
  if(flag===SE_GREG_CAL)return 'gregorian';
  if(flag===SE_JUL_CAL)return 'julian';
  throw new RangeError('Explicit Gregorian or Julian calendar flag is required');
};
const warnings=Object.freeze(['EXPERIMENTAL_NOT_SWISS_PARITY','SOLAR_DEFLECTION_OMITTED','NUTATION_IAU_2000B_APPROXIMATE','TDB_DAY_RATES_NOT_EXACT_UT_RATES']);
const starWarnings=Object.freeze(['EXPERIMENTAL_NOT_SWISS_PARITY','SOLAR_DEFLECTION_OMITTED','STELLAR_LIGHT_TIME_OMITTED','RECTILINEAR_STELLAR_MOTION','STAR_RATES_NUMERICAL_PRECISION_LIMITED','NUTATION_IAU_2000B_APPROXIMATE','TDB_DAY_RATES_NOT_EXACT_UT_RATES']);

function snapshotCatalog(catalog){
  if(catalog===undefined)return null;
  if(!catalog || typeof catalog!=='object')throw new TypeError('Explicit catalog ID and nonempty record list are required');
  const {id,records}=catalog;
  if(typeof id!=='string' || !id.trim() || id!==id.trim() || !Array.isArray(records))throw new TypeError('Explicit catalog ID and nonempty record list are required');
  const count=records.length,byName=new Map();
  if(!Number.isInteger(count) || count===0 || count>1000000)throw new TypeError('Explicit catalog ID and nonempty record list are required');
  for(let i=0;i<count;i++){
    if(!Object.hasOwn(records,i))throw new TypeError('Catalog records must be dense objects');
    const record=records[i];if(!record || typeof record!=='object')throw new TypeError('Catalog records must be dense objects');
    const {name,aliases,epochTdbJulianDay,rightAscensionDeg,declinationDeg,properMotionRaCosDecMasPerYear,properMotionDecMasPerYear,parallaxMas,radialVelocityKmPerSecond,visualMagnitude}=record;
    const values=[epochTdbJulianDay,rightAscensionDeg,declinationDeg,properMotionRaCosDecMasPerYear,properMotionDecMasPerYear,parallaxMas,radialVelocityKmPerSecond,visualMagnitude];
    if(typeof name!=='string' || !name.trim() || name!==name.trim() || !Array.isArray(aliases) || !values.every(Number.isFinite) || rightAscensionDeg<0 || rightAscensionDeg>=360 || Math.abs(declinationDeg)>=90 || parallaxMas<0.001 || parallaxMas>10000 || Math.abs(radialVelocityKmPerSecond)>=299792.458)throw new RangeError('Invalid or incomplete star catalog record');
    const aliasCount=aliases.length;if(!Number.isInteger(aliasCount) || aliasCount>20)throw new RangeError('Invalid star aliases');
    const entry=Object.freeze({name,epochTdbJulianDay,rightAscensionDeg,declinationDeg,properMotionRaCosDecMasPerYear,properMotionDecMasPerYear,parallaxMas,radialVelocityKmPerSecond,visualMagnitude});
    for(let n=-1;n<aliasCount;n++){
      if(n>=0 && !Object.hasOwn(aliases,n))throw new TypeError('Star aliases must be dense');
      const alias=n<0?name:aliases[n];
      if(typeof alias!=='string' || !alias.trim() || alias!==alias.trim())throw new TypeError('Star names and aliases must be nonempty trimmed strings');
      const key=alias.toLowerCase();if(byName.has(key))throw new RangeError('Duplicate star name or alias');byName.set(key,entry);
    }
  }
  return Object.freeze({id,lookup(name){if(typeof name!=='string' || !name.trim() || name!==name.trim())throw new TypeError('Explicit trimmed star name required');const entry=byName.get(name.toLowerCase());if(!entry)throw new RangeError('Star is not in the explicitly supplied catalog');return entry;}});
}

/** Caller supplies the classic JPL kernel, compiled Rust executable and a
 * synchronous UT1→{TT,TDB} provider for the same instant. No time conversion,
 * file-path inference, missing-body fallback or unsupported flag is invented.
 */
export function createExperimentalSwissSubset({executablePath,kernelPath,convertUt1ToTtTdb,catalog}) {
  if(typeof convertUt1ToTtTdb!=='function')throw new TypeError('An explicit UT1 to TT/TDB converter is required');
  const provider=createExperimentalRustCli({executablePath,kernelPath});
  const stars=snapshotCatalog(catalog);
  const getEpochs=ut1=>{
    if(!Number.isFinite(ut1))throw new TypeError('A finite UT1 Julian day is required');
    const epochs=convertUt1ToTtTdb(ut1);
    if(!epochs || typeof epochs!=='object')throw new TypeError('Converter must supply finite TT and TDB Julian days');
    const {ttJulianDay,tdbJulianDay}=epochs;
    if(!Number.isFinite(ttJulianDay) || !Number.isFinite(tdbJulianDay) || Math.abs(ttJulianDay-ut1)>0.1 || Math.abs(ttJulianDay-tdbJulianDay)>1/86400)throw new RangeError('UT1, TT and TDB must describe the same instant');
    return {ttJulianDay,tdbJulianDay};
  };
  const validFlags=flags=>{if(flags!==SEFLG_JPLEPH && flags!==(SEFLG_JPLEPH|SEFLG_SPEED))throw new RangeError('Only explicit JPL and optional speed flags are supported');};
  const callbackType=callback=>{if(callback!==undefined && typeof callback!=='function')throw new TypeError('Callback must be a function');};
  const star=name=>{if(!stars)throw new Error('An explicit star catalog is required');return stars.lookup(name);};
  const api={
    SE_JUL_CAL,SE_GREG_CAL,SEFLG_JPLEPH,SEFLG_SWIEPH,SEFLG_SPEED,SE_SUN,SE_MOON,SE_MERCURY,SE_VENUS,SE_MARS,SE_JUPITER,SE_SATURN,SE_URANUS,SE_NEPTUNE,SE_PLUTO,
    swe_julday(year,month,day,hour,calendarFlag){
      return provider.julianDay({year,month,day,decimalHour:hour,calendar:calendar(calendarFlag),timeScale:'ut1'});
    },
    swe_revjul(julianDay,calendarFlag){
      const value=provider.reverseJulianDay({julianDay,calendar:calendar(calendarFlag),timeScale:'ut1'});
      return Object.freeze({year:value.year,month:value.month,day:value.day,hour:value.decimalHour});
    },
    swe_calc_ut(julianDay,body,flags,callback){
      if(!Number.isFinite(julianDay))throw new TypeError('A finite UT1 Julian day is required');
      if(!Number.isInteger(body) || body<0 || body>=PHYSICAL_TARGETS.length)throw new RangeError('Only Sun, Moon, Mercury and Venus physical centers are supported');
      validFlags(flags);callbackType(callback);
      const {ttJulianDay,tdbJulianDay}=getEpochs(julianDay);
      const state=provider.calculateAtEpoch({target:PHYSICAL_TARGETS[body],ttJulianDay,tdbJulianDay});
      const speed=(flags&SEFLG_SPEED)!==0;
      const value=Object.freeze({longitude:state.longitudeDeg,latitude:state.latitudeDeg,distance:state.distanceAu,
        longitudeSpeed:speed?state.longitudeSpeedDegPerTdbDay:0,latitudeSpeed:speed?state.latitudeSpeedDegPerTdbDay:0,distanceSpeed:speed?state.distanceSpeedAuPerTdbDay:0,
        rflag:flags,model:'EXPERIMENTAL_DE441_IAU2006_2000B',warnings});
      if(callback!==undefined)callback(value);
      return value;
    },
    swe_fixstar_ut(name,julianDay,flags,callback){
      validFlags(flags);callbackType(callback);const entry=star(name);
      const {ttJulianDay,tdbJulianDay}=getEpochs(julianDay);
      const state=provider.calculateStarAtEpoch({...entry,ttJulianDay,tdbJulianDay});
      const speed=(flags&SEFLG_SPEED)!==0;
      const value=Object.freeze({name:entry.name,longitude:state.longitudeDeg,latitude:state.latitudeDeg,distance:state.distanceAu,
        longitudeSpeed:speed?state.longitudeSpeedDegPerTdbDay:0,latitudeSpeed:speed?state.latitudeSpeedDegPerTdbDay:0,distanceSpeed:speed?state.distanceSpeedAuPerTdbDay:0,
        rflag:flags,model:'EXPERIMENTAL_ICRS_LINEAR_STAR',catalogId:stars.id,warnings:starWarnings});
      if(callback!==undefined)callback(value);return value;
    },
    swe_fixstar_mag(name,callback){
      callbackType(callback);const entry=star(name);
      const value=Object.freeze({name:entry.name,magnitude:entry.visualMagnitude,catalogId:stars.id,model:'CALLER_SUPPLIED_CATALOG_MAGNITUDE'});
      if(callback!==undefined)callback(value);return value;
    },
  };
  return Object.freeze(api);
}
