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

export const SE_JUL_CAL=0,SE_GREG_CAL=1,SEFLG_JPLEPH=1,SEFLG_SWIEPH=2,SEFLG_HELCTR=8,SEFLG_SPEED=256,SEFLG_EQUATORIAL=2048;
export const SE_ECL2HOR=0,SE_EQU2HOR=1;
export const SE_CALC_RISE=1,SE_CALC_SET=2,SE_CALC_MTRANSIT=4,SE_CALC_ITRANSIT=8;
export const SE_ECL_NUT=-1,SE_SUN=0,SE_MOON=1,SE_MERCURY=2,SE_VENUS=3,SE_MARS=4,SE_JUPITER=5,SE_SATURN=6,SE_URANUS=7,SE_NEPTUNE=8,SE_PLUTO=9;
const PHYSICAL_TARGETS=Object.freeze(['Sun','Moon','Mercury','Venus','Mars','Jupiter','Saturn','Uranus','Neptune','Pluto']);
const HOUSE_SYSTEMS=Object.freeze({V:'vehlow',P:'placidus',K:'koch',R:'regiomontanus',O:'porphyry'});
const calendar=flag=>{
  if(flag===SE_GREG_CAL)return 'gregorian';
  if(flag===SE_JUL_CAL)return 'julian';
  throw new RangeError('Explicit Gregorian or Julian calendar flag is required');
};
const warnings=Object.freeze(['EXPERIMENTAL_NOT_SWISS_PARITY','SOLAR_DEFLECTION_OMITTED','NUTATION_IAU_2000B_APPROXIMATE','TDB_DAY_RATES_NOT_EXACT_UT_RATES']);
const centerWarnings=Object.freeze([...warnings,'PHYSICAL_CENTER_SPK_EXPLICIT','DE441_AND_SPK_MAY_USE_DIFFERENT_SOLUTIONS']);
const deflectedPlanetWarnings=Object.freeze(['EXPERIMENTAL_NOT_SWISS_PARITY','SOLAR_DEFLECTION_POINT_MASS_APPLIED','PLANETARY_DEFLECTION_OMITTED','PLANET_RATES_NUMERICAL_PRECISION_LIMITED','NUTATION_IAU_2000B_APPROXIMATE','TDB_DAY_RATES_NOT_EXACT_UT_RATES']);
const deflectedCenterWarnings=Object.freeze([...deflectedPlanetWarnings,'PHYSICAL_CENTER_SPK_EXPLICIT','DE441_AND_SPK_MAY_USE_DIFFERENT_SOLUTIONS']);
const deflectedSunWarnings=Object.freeze(['EXPERIMENTAL_NOT_SWISS_PARITY','SUN_SELF_DEFLECTION_NOT_APPLICABLE','PLANETARY_DEFLECTION_OMITTED','NUTATION_IAU_2000B_APPROXIMATE','TDB_DAY_RATES_NOT_EXACT_UT_RATES']);
const helioWarnings=Object.freeze(['EXPERIMENTAL_NOT_SWISS_PARITY','NO_ANNUAL_ABERRATION','SOLAR_DEFLECTION_OMITTED','NUTATION_IAU_2000B_APPROXIMATE','TDB_DAY_RATES_NOT_EXACT_UT_RATES']);
const helioCenterWarnings=Object.freeze([...helioWarnings,'PHYSICAL_CENTER_SPK_EXPLICIT','DE441_AND_SPK_MAY_USE_DIFFERENT_SOLUTIONS']);
const helioSunWarnings=Object.freeze([...helioWarnings,'SUN_CENTER_ZERO_SENTINEL_NO_DIRECTION']);
const starWarnings=Object.freeze(['EXPERIMENTAL_NOT_SWISS_PARITY','SOLAR_DEFLECTION_OMITTED','STELLAR_LIGHT_TIME_OMITTED','RECTILINEAR_STELLAR_MOTION','STAR_RATES_NUMERICAL_PRECISION_LIMITED','NUTATION_IAU_2000B_APPROXIMATE','TDB_DAY_RATES_NOT_EXACT_UT_RATES']);
const deflectedStarWarnings=Object.freeze(['EXPERIMENTAL_NOT_SWISS_PARITY','SOLAR_DEFLECTION_POINT_MASS_APPLIED','PLANETARY_DEFLECTION_OMITTED','STELLAR_LIGHT_TIME_OMITTED','RECTILINEAR_STELLAR_MOTION','STAR_RATES_NUMERICAL_PRECISION_LIMITED','NUTATION_IAU_2000B_APPROXIMATE','TDB_DAY_RATES_NOT_EXACT_UT_RATES']);

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
export function createExperimentalSwissSubset({executablePath,kernelPath,convertUt1ToTtTdb,catalog,houseSupport,physicalCenterKernels,horizontalSupport,solarEventSupport,legacyMutableResults=false,starSolarDeflection='OMIT',planetSolarDeflection='OMIT'}) {
  if(typeof convertUt1ToTtTdb!=='function')throw new TypeError('An explicit UT1 to TT/TDB converter is required');
  if(typeof legacyMutableResults!=='boolean')throw new TypeError('legacyMutableResults must be boolean');
  if(!['OMIT','POINT_MASS'].includes(starSolarDeflection))throw new RangeError('Unsupported stellar solar deflection model');
  if(!['OMIT','POINT_MASS'].includes(planetSolarDeflection))throw new RangeError('Unsupported planetary solar deflection model');
  const provider=createExperimentalRustCli({executablePath,kernelPath});
  const stars=snapshotCatalog(catalog);
  let solarEvents=null;
  if(solarEventSupport!==undefined){
    if(!solarEventSupport || typeof solarEventSupport!=='object')throw new TypeError('Explicit solar event search configuration is required');
    const {windowDays,stepMinutes,timeOffsets}=solarEventSupport;
    if(!Number.isFinite(windowDays) || windowDays<=0 || windowDays>2 || !Number.isFinite(stepMinutes) || stepMinutes<1 || stepMinutes>60 || timeOffsets!=='CONSTANT_IN_SEARCH')throw new RangeError('Solar event window, cadence and constant-time-offset assumption are required');
    solarEvents=Object.freeze({windowDays,stepMinutes});
  }
  let horizontalModel=null;
  if(horizontalSupport!==undefined){
    if(!horizontalSupport || typeof horizontalSupport!=='object' || !['GEOMETRIC','NOAA_STANDARD'].includes(horizontalSupport.refraction))throw new TypeError('Explicit horizontal refraction model is required');
    horizontalModel=horizontalSupport.refraction;
  }
  const centers={};
  if(physicalCenterKernels!==undefined){
    if(!physicalCenterKernels || typeof physicalCenterKernels!=='object' || Array.isArray(physicalCenterKernels))throw new TypeError('Physical-center SPK paths must be an object');
    for(const [name,path] of Object.entries(physicalCenterKernels)){
      if(!PHYSICAL_TARGETS.slice(4).includes(name) || typeof path!=='string' || !path.trim())throw new TypeError('Supported physical-center SPK paths are required');
      centers[name]=path;
    }
  }
  Object.freeze(centers);
  let houseConfig=null;
  if(houseSupport!==undefined){
    if(!houseSupport || typeof houseSupport!=='object')throw new TypeError('Explicit house configuration required');
    const {calculateHouseSet,convertUt1ToUtc,engineVersion,kernelSetId,timeModel,calculatePlacidusHousePosition,positionContext}=houseSupport;
    const text=value=>typeof value==='string' && value.length>0 && value===value.trim();
    if(typeof calculateHouseSet!=='function' || typeof convertUt1ToUtc!=='function' || ![engineVersion,kernelSetId,timeModel].every(text))throw new TypeError('House provider, UT1-to-UTC converter and source identifiers are required');
    if(calculatePlacidusHousePosition!==undefined && typeof calculatePlacidusHousePosition!=='function')throw new TypeError('Placidus position provider must be a function');
    let pointContext=null;
    if(positionContext!==undefined){
      if(!positionContext || typeof positionContext!=='object')throw new TypeError('Explicit house-position context required');
      const {datetimeUtc,corrections,observerLongitudeDegEast}=positionContext;
      if(!text(datetimeUtc) || !text(corrections) || !Number.isFinite(observerLongitudeDegEast) || Math.abs(observerLongitudeDegEast)>180)throw new TypeError('House-position UTC, corrections and observer longitude are required');
      pointContext=Object.freeze({datetimeUtc,corrections,observerLongitudeDegEast});
    }
    houseConfig=Object.freeze({calculateHouseSet,convertUt1ToUtc,engineVersion,kernelSetId,timeModel,calculatePlacidusHousePosition,positionContext:pointContext});
  }
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
  const nutation=(ttJulianDay,flags,callback)=>{
    callbackType(callback);if(flags!==0)throw new RangeError('Ecliptic nutation accepts only flags 0');
    const n=provider.eclipticNutation(ttJulianDay);
    const value=Object.freeze({longitude:n.trueObliquity,latitude:n.meanObliquity,distance:n.nutationLongitude,longitudeSpeed:n.nutationObliquity,latitudeSpeed:0,distanceSpeed:0,rflag:0,model:'EXPERIMENTAL_IAU2006_2000B_NUTATION',warnings:Object.freeze(['NUTATION_IAU_2000B_APPROXIMATE','FIELDS_ARE_ANGLES_NOT_POSITION_OR_SPEED'])});
    if(callback!==undefined)callback(value);return value;
  };
  const star=name=>{if(!stars)throw new Error('An explicit star catalog is required');return stars.lookup(name);};
  const api={
    SE_JUL_CAL,SE_GREG_CAL,SEFLG_JPLEPH,SEFLG_SWIEPH,SEFLG_HELCTR,SEFLG_EQUATORIAL,SEFLG_SPEED,SE_ECL2HOR,SE_EQU2HOR,SE_CALC_RISE,SE_CALC_SET,SE_CALC_MTRANSIT,SE_CALC_ITRANSIT,SE_ECL_NUT,SE_SUN,SE_MOON,SE_MERCURY,SE_VENUS,SE_MARS,SE_JUPITER,SE_SATURN,SE_URANUS,SE_NEPTUNE,SE_PLUTO,
    swe_rise_trans(julianDay,body,starName,flags,event,longitudeDegEast,latitudeDeg,heightMetres,pressureHpa,temperatureC,callback){
      if(!solarEvents)throw new Error('Explicit solar event search configuration is required');
      callbackType(callback);
      if(body!==SE_SUN || starName!=='' || flags!==SEFLG_JPLEPH || ![SE_CALC_RISE,SE_CALC_SET,SE_CALC_MTRANSIT,SE_CALC_ITRANSIT].includes(event))throw new RangeError('Only explicit JPL solar rise, set and meridian transit events are supported');
      if(![julianDay,longitudeDegEast,latitudeDeg,heightMetres,pressureHpa,temperatureC].every(Number.isFinite) || Math.abs(longitudeDegEast)>180 || Math.abs(latitudeDeg)>=90 || heightMetres!==0 || pressureHpa!==0 || temperatureC!==0)throw new RangeError('Valid nonpolar site and zero height, pressure and temperature placeholders are required');
      const {ttJulianDay,tdbJulianDay}=getEpochs(julianDay);
      const kind=event===SE_CALC_RISE?'rise':event===SE_CALC_SET?'set':event===SE_CALC_MTRANSIT?'upper_transit':'lower_transit';
      const input={kind,ut1JulianDay:julianDay,ttJulianDay,tdbJulianDay,latitudeDeg,longitudeDegEast,...solarEvents};
      const result=event===SE_CALC_RISE || event===SE_CALC_SET?provider.findStandardSolarEvent(input):provider.findGeometricEvent({...input,target:'Sun',thresholdAltitudeDeg:0,observerHeightM:0});
      const value=Object.freeze({name:'',transitTime:result.ut1JulianDay,bracketSeconds:result.bracketSeconds,model:result.model,warnings:result.warnings});
      if(callback!==undefined)callback(value);return value;
    },
    swe_azalt(julianDay,mode,longitudeDegEast,latitudeDeg,heightMetres,pressureHpa,temperatureC,longitudeOrRa,latitudeOrDec,distance,callback){
      if(!horizontalModel)throw new Error('Explicit horizontal refraction model is required');
      callbackType(callback);
      if(mode!==SE_ECL2HOR && mode!==SE_EQU2HOR)throw new RangeError('Only ecliptic and equatorial horizontal inputs are supported');
      if(![julianDay,longitudeDegEast,latitudeDeg,heightMetres,pressureHpa,temperatureC,longitudeOrRa,latitudeOrDec,distance].every(Number.isFinite) || heightMetres!==0 || pressureHpa!==0 || temperatureC!==0 || distance<0)throw new RangeError('Finite coordinates and zero height, pressure and temperature placeholders are required');
      const {ttJulianDay}=getEpochs(julianDay);
      const horizontal=provider.calculateHorizontal({ut1JulianDay:julianDay,ttJulianDay,longitudeDegEast,latitudeDeg,rightAscensionDeg:longitudeOrRa,declinationDeg:latitudeOrDec,coordinateFrame:mode===SE_ECL2HOR?'ecliptic':'equatorial'});
      if(horizontal.azimuthSouthWestDeg===null)throw new RangeError('Azimuth is undefined at zenith, nadir or a geographic pole');
      let apparentAltitude=horizontal.trueAltitudeDeg;
      const notes=['NOT_SWISS_PARITY','INPUT_PARALLAX_NOT_APPLIED','NO_POLAR_MOTION','DISTANCE_UNUSED'];
      if(horizontalModel==='GEOMETRIC')notes.push('NO_REFRACTION');
      else if(horizontal.trueAltitudeDeg < -1)notes.push('BELOW_MINUS_ONE_NO_REFRACTION');
      else {apparentAltitude=provider.calculateStandardRefraction({trueAltitudeDeg:horizontal.trueAltitudeDeg}).apparentAltitudeDeg;notes.push('NOAA_FIXED_ATMOSPHERE_APPROXIMATE');}
      const value={azimuth:horizontal.azimuthSouthWestDeg,trueAltitude:horizontal.trueAltitudeDeg,apparentAltitude,model:horizontalModel==='GEOMETRIC'?'EXPERIMENTAL_GEOMETRIC_HORIZON':'EXPERIMENTAL_NOAA_STANDARD_HORIZON',warnings:Object.freeze(notes)};
      if(callback!==undefined)callback(value);return value;
    },
    swe_get_planet_name(body,callback){
      callbackType(callback);if(!Number.isInteger(body) || body<0 || body>=PHYSICAL_TARGETS.length)throw new RangeError('Only Sun through Pluto names are supported');
      const value=Object.freeze({name:PHYSICAL_TARGETS[body]});if(callback!==undefined)callback(value);return value;
    },
    swe_calc(julianDayTt,body,flags,callback){
      if(body!==SE_ECL_NUT)throw new RangeError('Only SE_ECL_NUT is supported in swe_calc');
      return nutation(julianDayTt,flags,callback);
    },
    swe_julday(year,month,day,hour,calendarFlag){
      return provider.julianDay({year,month,day,decimalHour:hour,calendar:calendar(calendarFlag),timeScale:'ut1'});
    },
    swe_revjul(julianDay,calendarFlag){
      const value=provider.reverseJulianDay({julianDay,calendar:calendar(calendarFlag),timeScale:'ut1'});
      return Object.freeze({year:value.year,month:value.month,day:value.day,hour:value.decimalHour});
    },
    swe_houses(julianDay,latitudeDeg,longitudeDegEast,system,callback){
      if(!houseConfig)throw new Error('Explicit house support is required');
      callbackType(callback);
      if(typeof system!=='string' || !Object.hasOwn(HOUSE_SYSTEMS,system))throw new RangeError('Only V, P, K, R and O house systems are supported');
      if(!Number.isFinite(latitudeDeg) || Math.abs(latitudeDeg)>=90 || !Number.isFinite(longitudeDegEast) || Math.abs(longitudeDegEast)>180)throw new RangeError('Valid nonpolar latitude and east longitude are required');
      const {ttJulianDay}=getEpochs(julianDay);
      const datetimeUtc=houseConfig.convertUt1ToUtc(julianDay);
      const match=typeof datetimeUtc==='string'?/^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d{1,3}))?Z$/.exec(datetimeUtc):null;
      const ms=match?Date.parse(datetimeUtc):NaN;
      const canonical=match?`${match[1]}.${(match[2]??'').padEnd(3,'0')}Z`:'';
      if(!Number.isFinite(ms) || new Date(ms).toISOString()!==canonical || ms<Date.UTC(1972,0,1) || Math.abs(julianDay-(ms/86400000+2440587.5))*86400>2)throw new RangeError('Converter must provide coherent modern UTC with seconds');
      const angleInput={ut1JulianDay:julianDay,ttJulianDay,latitudeDeg,longitudeDegEast};
      const angles=provider.calculateHouseAngles(angleInput);
      const result=houseConfig.calculateHouseSet({datetimeUtc:canonical,birthTimeKnown:true,...angleInput,engineVersion:houseConfig.engineVersion,kernelSetId:houseConfig.kernelSetId,timeModel:houseConfig.timeModel},{calculateHouseAngles:()=>angles});
      if(!result || typeof result!=='object')throw new Error('Invalid experimental house-set contract');
      const {contract,houses,validation}=result;
      if(contract!=='ExperimentalHouseSet' || validation!=='CALLER_ASSERTED_INPUTS_NOT_INDEPENDENTLY_VERIFIED' || !houses || typeof houses!=='object')throw new Error('Invalid experimental house-set contract');
      const chosen=houses[HOUSE_SYSTEMS[system]];
      if(!chosen || typeof chosen!=='object')throw new Error('Invalid house cusp contract');
      const {houseSystem,cuspsDeg}=chosen;
      if(houseSystem!==HOUSE_SYSTEMS[system].toUpperCase() || !Array.isArray(cuspsDeg) || cuspsDeg.length!==12)throw new Error('Invalid house cusp contract');
      const cusps=[];for(let i=0;i<12;i++){if(!Object.hasOwn(cuspsDeg,i))throw new Error('Invalid house cusp row');const v=cuspsDeg[i];if(!Number.isFinite(v) || v<0 || v>=360)throw new Error('Invalid house cusp row');cusps.push(v);}
      const houseWarnings=Object.freeze(['NOT_SWISS_PARITY','AUXILIARY_ANGLES_OMITTED','NO_SOUTHERN_INVERSION','ALL_FIVE_SYSTEMS_REQUIRED']);
      let value;
      if(legacyMutableResults){
        value={house:cusps,ascendant:angles.ascendantDeg,mc:angles.midheavenDeg,armc:angles.armcDeg};
        Object.defineProperties(value,{trueObliquity:{value:angles.trueObliquityDeg},model:{value:'EXPERIMENTAL_IAU2006_2000B_HOUSES'},warnings:{value:houseWarnings}});
      } else value=Object.freeze({house:Object.freeze(cusps),ascendant:angles.ascendantDeg,mc:angles.midheavenDeg,armc:angles.armcDeg,trueObliquity:angles.trueObliquityDeg,model:'EXPERIMENTAL_IAU2006_2000B_HOUSES',warnings:houseWarnings});
      if(callback!==undefined)callback(value);return value;
    },
    swe_houses_pos(armcDeg,latitudeDeg,trueObliquityDeg,system,eclipticLongitudeDeg,eclipticLatitudeDeg,callback){
      if(!houseConfig || !houseConfig.positionContext || !houseConfig.calculatePlacidusHousePosition)throw new Error('Explicit Placidus position provider and context are required');
      callbackType(callback);if(system!=='P')throw new RangeError('Only Placidus P positions are supported');
      const {datetimeUtc,corrections,observerLongitudeDegEast}=houseConfig.positionContext;
      const source={engineVersion:houseConfig.engineVersion,kernelSetId:houseConfig.kernelSetId,timeModel:houseConfig.timeModel,corrections,center:'EARTH',observerLongitudeDegEast};
      const result=houseConfig.calculatePlacidusHousePosition({armcDeg,latitudeDeg,trueObliquityDeg,eclipticLongitudeDeg,eclipticLatitudeDeg,frame:'ECLIPTIC_OF_DATE',zodiac:'TROPICAL',birthTimeKnown:true,datetimeUtc,source});
      if(!result || typeof result!=='object')throw new Error('Invalid Placidus position contract');
      const {contract,housePosition}=result;
      if(contract!=='PlacidusHousePosition' || !Number.isFinite(housePosition) || housePosition<1 || housePosition>=13)throw new Error('Invalid Placidus position contract');
      const value=Object.freeze({housePosition,model:'EXPERIMENTAL_PLACIDUS_POSITION',warnings:Object.freeze(['NOT_SWISS_PARITY','NO_CIRCUMPOLAR_FALLBACK','NO_SOUTHERN_INVERSION','CALLER_ASSERTED_EPOCH'])});
      if(callback!==undefined)callback(value);return value;
    },
    swe_calc_ut(julianDay,body,flags,callback){
      if(!Number.isFinite(julianDay))throw new TypeError('A finite UT1 Julian day is required');
      if(body===SE_ECL_NUT){callbackType(callback);if(flags!==0)throw new RangeError('Ecliptic nutation accepts only flags 0');return nutation(getEpochs(julianDay).ttJulianDay,flags,callback);}
      if(!Number.isInteger(body) || body<0 || body>=PHYSICAL_TARGETS.length)throw new RangeError('Only Sun through Pluto physical centers are supported');
      if(!Number.isInteger(flags) || flags<0 || flags>(SEFLG_JPLEPH|SEFLG_SPEED|SEFLG_HELCTR|SEFLG_EQUATORIAL) || (flags&SEFLG_JPLEPH)===0 || (flags&~(SEFLG_JPLEPH|SEFLG_SPEED|SEFLG_HELCTR|SEFLG_EQUATORIAL))!==0 || ((flags&SEFLG_HELCTR)!==0 && (flags&SEFLG_EQUATORIAL)!==0))throw new RangeError('Only explicit JPL, optional speed, and one supported coordinate flag are allowed in flags');
      callbackType(callback);
      const target=PHYSICAL_TARGETS[body];
      if(body>=4 && !Object.hasOwn(centers,target))throw new RangeError('Explicit physical-center SPK is required for '+target);
      const {ttJulianDay,tdbJulianDay}=getEpochs(julianDay);
      const equatorial=(flags&SEFLG_EQUATORIAL)!==0, heliocentric=(flags&SEFLG_HELCTR)!==0;
      let state;
      if(heliocentric && body===SE_SUN)state={longitudeDeg:0,latitudeDeg:0,distanceAu:0,longitudeSpeedDegPerTdbDay:0,latitudeSpeedDegPerTdbDay:0,distanceSpeedAuPerTdbDay:0};
      else if(!heliocentric && planetSolarDeflection==='POINT_MASS')state=provider.calculateCoordinateAtEpoch({mode:equatorial?'equatorial':'ecliptic',target,ttJulianDay,tdbJulianDay,centerKernelPath:centers[target],solarDeflection:planetSolarDeflection});
      else if(equatorial || heliocentric)state=provider.calculateCoordinateAtEpoch({mode:equatorial?'equatorial':'heliocentric',target,ttJulianDay,tdbJulianDay,centerKernelPath:centers[target]});
      else state=body<4?provider.calculateAtEpoch({target,ttJulianDay,tdbJulianDay}):provider.calculatePhysicalAtEpoch({target,ttJulianDay,tdbJulianDay,centerKernelPath:centers[target]});
      const speed=(flags&SEFLG_SPEED)!==0;
      const coordinates=equatorial?{rectAscension:state.longitudeDeg,declination:state.latitudeDeg,rectAscensionSpeed:speed?state.longitudeSpeedDegPerTdbDay:0,declinationSpeed:speed?state.latitudeSpeedDegPerTdbDay:0}:{longitude:state.longitudeDeg,latitude:state.latitudeDeg,longitudeSpeed:speed?state.longitudeSpeedDegPerTdbDay:0,latitudeSpeed:speed?state.latitudeSpeedDegPerTdbDay:0};
      const chosenWarnings=heliocentric?(body===SE_SUN?helioSunWarnings:body<4?helioWarnings:helioCenterWarnings):planetSolarDeflection==='POINT_MASS'?(body===SE_SUN?deflectedSunWarnings:body<4?deflectedPlanetWarnings:deflectedCenterWarnings):body<4?warnings:centerWarnings;
      const value=Object.freeze({...coordinates,distance:state.distanceAu,distanceSpeed:speed?state.distanceSpeedAuPerTdbDay:0,
        rflag:flags,model:'EXPERIMENTAL_DE441_IAU2006_2000B',warnings:chosenWarnings});
      if(callback!==undefined)callback(value);
      return value;
    },
    swe_fixstar_ut(name,julianDay,flags,callback){
      validFlags(flags);callbackType(callback);const entry=star(name);
      const {ttJulianDay,tdbJulianDay}=getEpochs(julianDay);
      const state=provider.calculateStarAtEpoch({...entry,ttJulianDay,tdbJulianDay,solarDeflection:starSolarDeflection});
      const speed=(flags&SEFLG_SPEED)!==0;
      const fields={name:entry.name,longitude:state.longitudeDeg,latitude:state.latitudeDeg,distance:state.distanceAu,
        longitudeSpeed:speed?state.longitudeSpeedDegPerTdbDay:0,latitudeSpeed:speed?state.latitudeSpeedDegPerTdbDay:0,distanceSpeed:speed?state.distanceSpeedAuPerTdbDay:0,
        rflag:flags};
      let value;
      const selectedWarnings=starSolarDeflection==='POINT_MASS'?deflectedStarWarnings:starWarnings;
      if(legacyMutableResults){
        value=fields;Object.defineProperties(value,{model:{value:'EXPERIMENTAL_ICRS_LINEAR_STAR'},catalogId:{value:stars.id},warnings:{value:selectedWarnings}});
      } else value=Object.freeze({...fields,model:'EXPERIMENTAL_ICRS_LINEAR_STAR',catalogId:stars.id,warnings:selectedWarnings});
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
