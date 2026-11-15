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

/**
 * Thin synchronous Node adapter for the experimental Rust DE441 example.
 * No astronomical model is implemented here. Added 2026-11-14.
 * The executable and separately licensed kernel are supplied by the caller.
 */
import { spawnSync } from 'node:child_process';

const HEADER = 'target,tt_jd,tdb_jd,longitude_deg,latitude_deg,distance_au,longitude_deg_per_tdb_day,latitude_deg_per_tdb_day,distance_au_per_tdb_day';
const TARGETS = Object.freeze(['Sun', 'Moon', 'Mercury', 'Venus', 'MarsBarycenter', 'JupiterBarycenter', 'SaturnBarycenter', 'UranusBarycenter', 'NeptuneBarycenter', 'PlutoBarycenter']);
const ANGLE_HEADER = 'armc_deg,ascendant_deg,midheaven_deg,true_obliquity_deg';
const LUNAR_HEADER = 'point,tt_jd,tdb_jd,longitude_deg,latitude_deg,distance_au,longitude_deg_per_tt_day,latitude_deg_per_tt_day,distance_au_per_tt_day';
const LUNAR_POINTS = Object.freeze(['MeanNode','TrueNode','MeanApogee','OsculatingApogee']);
const EQUATORIAL_HEADER = 'target,tt_jd,tdb_jd,ra_deg,declination_deg,distance_au,ra_deg_per_tdb_day,declination_deg_per_tdb_day,distance_au_per_tdb_day';
const HORIZONTAL_HEADER = 'true_altitude_deg,azimuth_north_east_deg,azimuth_south_west_deg';
const EVENT_HEADER = 'event,target,ut1_jd,tt_jd,tdb_jd,bracket_seconds';

/**
 * Construct an explicit offline provider. It returns experimental positions,
 * not Swiss-compatible output; unsupported bodies and options are errors.
 * The child is started once per batch, so this is not a high-throughput API.
 */
export function createExperimentalRustCli({ executablePath, kernelPath }) {
  if (typeof executablePath !== 'string' || !executablePath.trim() ||
      typeof kernelPath !== 'string' || !kernelPath.trim()) {
    throw new TypeError('Explicit executablePath and kernelPath are required');
  }
  const execute = args => {
    const child = spawnSync(executablePath, args, {
      encoding: 'utf8', windowsHide: true, timeout: 30000, maxBuffer: 1024 * 1024,
    });
    if (child.error) throw new Error(`Ephemeris process failed: ${child.error.message}`, { cause: child.error });
    if (child.status !== 0) throw new Error(`Ephemeris process exited with status ${child.status}: ${(child.stderr || '').trim()}`);
    return child.stdout.trim().split(/\r?\n/);
  };
  return Object.freeze({
    eclipticNutation(ttJulianDay) {
      if(!Number.isFinite(ttJulianDay))throw new TypeError('Explicit finite TT Julian day required');
      const lines=execute(['ecl_nut',String(ttJulianDay)]);
      if(lines.length!==2 || lines[0]!=='true_obliquity_deg,mean_obliquity_deg,nutation_longitude_deg,nutation_obliquity_deg,reserved_4,reserved_5')throw new Error('Invalid ecliptic nutation CSV contract');
      const parts=lines[1].split(',');
      if(parts.length!==6 || parts.some(value=>!value.trim() || !Number.isFinite(Number(value))))throw new Error('Invalid ecliptic nutation CSV row');
      const [trueObliquity,meanObliquity,nutationLongitude,nutationObliquity,reserved4,reserved5]=parts.map(Number);
      if(trueObliquity<=0 || trueObliquity>=90 || meanObliquity<=0 || meanObliquity>=90 || reserved4!==0 || reserved5!==0 || Math.abs(trueObliquity-meanObliquity-nutationObliquity)>1e-12)throw new Error('Out-of-range ecliptic nutation CSV row');
      return Object.freeze({trueObliquity,meanObliquity,nutationLongitude,nutationObliquity});
    },
    julianDay({year, month, day, decimalHour, calendar, timeScale}) {
      if (![year, month, day].every(Number.isInteger) || !Number.isFinite(decimalHour) || typeof calendar !== 'string' || typeof timeScale !== 'string') throw new TypeError('Explicit numeric date, decimal hour, calendar and time scale are required');
      const lines = execute(['julday', String(year), String(month), String(day), String(decimalHour), calendar, timeScale]);
      if (lines.length !== 2 || lines[0] !== 'julian_day' || !lines[1].trim() || !Number.isFinite(Number(lines[1]))) throw new Error('Invalid Julian-day CSV contract');
      return Number(lines[1]);
    },
    reverseJulianDay({julianDay, calendar, timeScale}) {
      if (!Number.isFinite(julianDay) || typeof calendar !== 'string' || typeof timeScale !== 'string') throw new TypeError('Finite Julian day, calendar and time scale are required');
      const lines = execute(['revjul', String(julianDay), calendar, timeScale]);
      if (lines.length !== 2 || lines[0] !== 'year,month,day,decimal_hour') throw new Error('Invalid reverse Julian-day CSV contract');
      const parts = lines[1].split(',');
      if (parts.length !== 4 || parts.some(part => !part.trim() || !Number.isFinite(Number(part)))) throw new Error('Invalid reverse Julian-day CSV row');
      const [year, month, day, decimalHour] = parts.map(Number);
      if (![year, month, day].every(Number.isInteger) || month < 1 || month > 12 || day < 1 || day > 31 || decimalHour < 0 || decimalHour >= 24) throw new Error('Out-of-range reverse Julian-day CSV row');
      return Object.freeze({year, month, day, decimalHour, calendar, timeScale});
    },
    calculateStarAtEpoch({ttJulianDay,tdbJulianDay,epochTdbJulianDay,rightAscensionDeg,declinationDeg,properMotionRaCosDecMasPerYear,properMotionDecMasPerYear,parallaxMas,radialVelocityKmPerSecond,solarDeflection='OMIT'}) {
      const values=[ttJulianDay,tdbJulianDay,epochTdbJulianDay,rightAscensionDeg,declinationDeg,properMotionRaCosDecMasPerYear,properMotionDecMasPerYear,parallaxMas,radialVelocityKmPerSecond];
      if(!values.every(Number.isFinite) || !['OMIT','POINT_MASS'].includes(solarDeflection))throw new TypeError('Explicit finite TT, TDB, stellar astrometry and supported solar deflection are required');
      const lines=execute([solarDeflection==='POINT_MASS'?'star_epoch_solar':'star_epoch',kernelPath,...values.map(String)]);
      if(lines.length!==2 || lines[0]!=='tt_jd,tdb_jd,longitude_deg,latitude_deg,distance_au,longitude_deg_per_tdb_day,latitude_deg_per_tdb_day,distance_au_per_tdb_day')throw new Error('Invalid star CSV contract');
      const parts=lines[1].split(',');if(parts.length!==8 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part))))throw new Error('Invalid star CSV row');
      const [tt,tdb,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay]=parts.map(Number);
      if(tt!==ttJulianDay || tdb!==tdbJulianDay || longitudeDeg<0 || longitudeDeg>=360 || Math.abs(latitudeDeg)>90 || distanceAu<=0)throw new Error('Out-of-range star CSV row');
      return Object.freeze({ttJulianDay:tt,tdbJulianDay:tdb,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay});
    },
    calculateAtEpoch({target,ttJulianDay,tdbJulianDay}) {
      if(!TARGETS.includes(target) || !Number.isFinite(ttJulianDay) || !Number.isFinite(tdbJulianDay)) throw new TypeError('Supported target and explicit finite TT and TDB are required');
      const lines=execute(['calc_epoch',kernelPath,target,String(ttJulianDay),String(tdbJulianDay)]);
      if(lines.length!==2 || lines[0]!==HEADER) throw new Error('Invalid epoch ephemeris CSV contract');
      const [returnedTarget,...parts]=lines[1].split(',');
      if(returnedTarget!==target || parts.length!==8 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part)))) throw new Error('Invalid epoch ephemeris CSV row');
      const [tt,tdb,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay]=parts.map(Number);
      if(tt!==ttJulianDay || tdb!==tdbJulianDay || longitudeDeg<0 || longitudeDeg>=360 || Math.abs(latitudeDeg)>90 || distanceAu<=0) throw new Error('Out-of-range epoch ephemeris CSV row');
      return Object.freeze({target,ttJulianDay:tt,tdbJulianDay:tdb,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay});
    },
    calculatePhysicalAtEpoch({target,ttJulianDay,tdbJulianDay,centerKernelPath}) {
      if(!['Mars','Jupiter','Saturn','Uranus','Neptune','Pluto'].includes(target) || !Number.isFinite(ttJulianDay) || !Number.isFinite(tdbJulianDay) || typeof centerKernelPath!=='string' || !centerKernelPath.trim())throw new TypeError('Physical target, explicit center SPK and finite TT and TDB are required');
      const lines=execute(['physical_epoch',kernelPath,centerKernelPath,target,String(ttJulianDay),String(tdbJulianDay)]);
      if(lines.length!==2 || lines[0]!==HEADER)throw new Error('Invalid physical epoch CSV contract');
      const [returnedTarget,...parts]=lines[1].split(',');
      if(returnedTarget!==target || parts.length!==8 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part))))throw new Error('Invalid physical epoch CSV row');
      const [tt,tdb,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay]=parts.map(Number);
      if(tt!==ttJulianDay || tdb!==tdbJulianDay || longitudeDeg<0 || longitudeDeg>=360 || Math.abs(latitudeDeg)>90 || distanceAu<=0)throw new Error('Out-of-range physical epoch CSV row');
      return Object.freeze({target,ttJulianDay:tt,tdbJulianDay:tdb,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay});
    },
    calculateCoordinateAtEpoch({mode,target,ttJulianDay,tdbJulianDay,centerKernelPath,solarDeflection='OMIT'}) {
      const targets=['Sun','Moon','Mercury','Venus','Mars','Jupiter','Saturn','Uranus','Neptune','Pluto'];
      const index=targets.indexOf(target);
      if(!['equatorial','heliocentric','ecliptic'].includes(mode) || !['OMIT','POINT_MASS'].includes(solarDeflection) || (solarDeflection==='OMIT' && mode==='ecliptic') || (solarDeflection==='POINT_MASS' && mode==='heliocentric') || index<0 || !Number.isFinite(ttJulianDay) || !Number.isFinite(tdbJulianDay) || (index>=4 && (typeof centerKernelPath!=='string' || !centerKernelPath.trim())))throw new TypeError('Supported coordinate mode, target, epochs and required center SPK must be explicit');
      const lines=execute([solarDeflection==='POINT_MASS'?'planet_deflected_epoch':'coordinate_epoch',mode,kernelPath,index>=4?centerKernelPath:'-',target,String(ttJulianDay),String(tdbJulianDay)]);
      if(lines.length!==2 || lines[0]!==HEADER)throw new Error('Invalid coordinate epoch CSV contract');
      const [returnedTarget,...parts]=lines[1].split(',');
      if(returnedTarget!==target || parts.length!==8 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part))))throw new Error('Invalid coordinate epoch CSV row');
      const [tt,tdb,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay]=parts.map(Number);
      if(tt!==ttJulianDay || tdb!==tdbJulianDay || longitudeDeg<0 || longitudeDeg>=360 || Math.abs(latitudeDeg)>90 || distanceAu<=0)throw new Error('Out-of-range coordinate epoch CSV row');
      return Object.freeze({target,ttJulianDay:tt,tdbJulianDay:tdb,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay});
    },
    calculate(timestamp) {
      if (typeof timestamp !== 'string' || !timestamp.trim()) {
        throw new TypeError('An ISO civil timestamp with explicit UTC offset is required');
      }
      const lines = execute([kernelPath, timestamp]);
      if (lines.length !== TARGETS.length + 1 || lines[0] !== HEADER) throw new Error('Invalid ephemeris CSV contract');
      const results = lines.slice(1).map((line, index) => {
        const [target, ...parts] = line.split(',');
        if (target !== TARGETS[index] || parts.length !== 8 || parts.some(part => !part.trim() || !Number.isFinite(Number(part)))) {
          throw new Error('Invalid ephemeris CSV row');
        }
        const [ttJulianDay, tdbJulianDay, longitudeDeg, latitudeDeg, distanceAu,
          longitudeSpeedDegPerTdbDay, latitudeSpeedDegPerTdbDay, distanceSpeedAuPerTdbDay] = parts.map(Number);
        if (longitudeDeg < 0 || longitudeDeg >= 360 || Math.abs(latitudeDeg) > 90 || distanceAu <= 0) {
          throw new Error('Out-of-range ephemeris CSV row');
        }
        return Object.freeze({ target, ttJulianDay, tdbJulianDay, longitudeDeg, latitudeDeg, distanceAu,
          longitudeSpeedDegPerTdbDay, latitudeSpeedDegPerTdbDay, distanceSpeedAuPerTdbDay });
      });
      return Object.freeze({
        contract: 'VIRGOSTARS_EXPERIMENTAL_DE441_TRUE_ECLIPTIC_1',
        frame: 'IAU2006_2000B_TRUE_ECLIPTIC_OF_DATE', center: 'EARTH',
        corrections: 'LIGHT_TIME_ANNUAL_ABERRATION', timeModel: 'BOUNDED_UTC_TRUNCATED_TDB',
        warnings: Object.freeze(['NOT_SWISS_PARITY', 'SOLAR_DEFLECTION_OMITTED', 'PLANET_SYSTEM_BARYCENTERS']),
        results: Object.freeze(results),
      });
    },
    calculateLunarPoints(timestamp) {
      if (typeof timestamp !== 'string' || !timestamp.trim()) throw new TypeError('An ISO civil timestamp with explicit UTC offset is required');
      const lines = execute(['lunar',kernelPath,timestamp]);
      if (lines.length !== LUNAR_POINTS.length + 1 || lines[0] !== LUNAR_HEADER) throw new Error('Invalid lunar-point CSV contract');
      const results = lines.slice(1).map((line,index) => {
        const [point,...parts]=line.split(',');
        if(point!==LUNAR_POINTS[index] || parts.length!==8 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part)))) throw new Error('Invalid lunar-point CSV row');
        const [ttJulianDay,tdbJulianDay,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTtDay,latitudeSpeedDegPerTtDay,distanceSpeedAuPerTtDay]=parts.map(Number);
        if(longitudeDeg<0 || longitudeDeg>=360 || Math.abs(latitudeDeg)>90 || distanceAu<=0) throw new Error('Out-of-range lunar-point CSV row');
        return Object.freeze({point,ttJulianDay,tdbJulianDay,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTtDay,latitudeSpeedDegPerTtDay,distanceSpeedAuPerTtDay});
      });
      return Object.freeze({contract:'VIRGOSTARS_EXPERIMENTAL_LUNAR_POINTS_1', frame:'IAU2006_2000B_TRUE_ECLIPTIC_OF_DATE', center:'EARTH', timeModel:'BOUNDED_UTC_TRUNCATED_TDB', warnings:Object.freeze(['NOT_SWISS_PARITY','MEAN_MODEL_APPROXIMATE','MEAN_DISTANCES_NOMINAL']),results:Object.freeze(results)});
    },
    calculateEquatorial(timestamp) {
      if (typeof timestamp !== 'string' || !timestamp.trim()) throw new TypeError('An ISO civil timestamp with explicit UTC offset is required');
      const lines=execute(['equatorial',kernelPath,timestamp]);
      if(lines.length!==TARGETS.length+1 || lines[0]!==EQUATORIAL_HEADER) throw new Error('Invalid equatorial CSV contract');
      const results=lines.slice(1).map((line,index)=>{
        const [target,...parts]=line.split(',');
        if(target!==TARGETS[index] || parts.length!==8 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part)))) throw new Error('Invalid equatorial CSV row');
        const [ttJulianDay,tdbJulianDay,rightAscensionDeg,declinationDeg,distanceAu,rightAscensionSpeedDegPerTdbDay,declinationSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay]=parts.map(Number);
        if(rightAscensionDeg<0 || rightAscensionDeg>=360 || Math.abs(declinationDeg)>90 || distanceAu<=0) throw new Error('Out-of-range equatorial CSV row');
        return Object.freeze({target,ttJulianDay,tdbJulianDay,rightAscensionDeg,declinationDeg,distanceAu,rightAscensionSpeedDegPerTdbDay,declinationSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay});
      });
      return Object.freeze({contract:'VIRGOSTARS_EXPERIMENTAL_DE441_TRUE_EQUATORIAL_1',frame:'IAU2006_2000B_TRUE_EQUATOR_OF_DATE',center:'EARTH',corrections:'LIGHT_TIME_ANNUAL_ABERRATION',timeModel:'BOUNDED_UTC_TRUNCATED_TDB',warnings:Object.freeze(['NOT_SWISS_PARITY','SOLAR_DEFLECTION_OMITTED','PLANET_SYSTEM_BARYCENTERS']),results:Object.freeze(results)});
    },
    calculateHeliocentric(timestamp) {
      if(typeof timestamp!=='string' || !timestamp.trim()) throw new TypeError('An ISO civil timestamp with explicit UTC offset is required');
      const lines=execute(['heliocentric',kernelPath,timestamp]);
      if(lines.length!==TARGETS.length+1 || lines[0]!==HEADER) throw new Error('Invalid heliocentric CSV contract');
      const results=lines.slice(1).map((line,index)=>{
        const [target,...parts]=line.split(',');
        if(target!==TARGETS[index] || parts.length!==8 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part)))) throw new Error('Invalid heliocentric CSV row');
        const [ttJulianDay,tdbJulianDay,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay]=parts.map(Number);
        const directionDefined=target!=='Sun';
        if(directionDefined ? (longitudeDeg<0 || longitudeDeg>=360 || Math.abs(latitudeDeg)>90 || distanceAu<=0) : [longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay].some(value=>value!==0)) throw new Error('Out-of-range heliocentric CSV row');
        return Object.freeze({target,ttJulianDay,tdbJulianDay,directionDefined,longitudeDeg,latitudeDeg,distanceAu,longitudeSpeedDegPerTdbDay,latitudeSpeedDegPerTdbDay,distanceSpeedAuPerTdbDay});
      });
      return Object.freeze({contract:'VIRGOSTARS_EXPERIMENTAL_DE441_HELIOCENTRIC_1',frame:'IAU2006_2000B_TRUE_ECLIPTIC_OF_DATE',center:'SUN',corrections:'LIGHT_TIME_ONLY',timeModel:'BOUNDED_UTC_TRUNCATED_TDB',warnings:Object.freeze(['NOT_SWISS_PARITY','SUN_ZERO_SENTINEL','PLANET_SYSTEM_BARYCENTERS']),results:Object.freeze(results)});
    },
    calculateHouseAngles({ ut1JulianDay, ttJulianDay, latitudeDeg, longitudeDegEast }) {
      const values = [ut1JulianDay, ttJulianDay, latitudeDeg, longitudeDegEast];
      if (!values.every(Number.isFinite)) throw new TypeError('Finite UT1, TT, latitude and east longitude are required');
      const lines = execute(['angles', ...values.map(String)]);
      if (lines.length !== 2 || lines[0] !== ANGLE_HEADER) throw new Error('Invalid angle CSV contract');
      const parts = lines[1].split(',');
      if (parts.length !== 4 || parts.some(part => !part.trim() || !Number.isFinite(Number(part)))) throw new Error('Invalid angle CSV row');
      const [armcDeg, ascendantDeg, midheavenDeg, trueObliquityDeg] = parts.map(Number);
      if ([armcDeg, ascendantDeg, midheavenDeg].some(value => value < 0 || value >= 360) || trueObliquityDeg <= 0 || trueObliquityDeg >= 90) throw new Error('Out-of-range angle CSV row');
      return Object.freeze({ armcDeg, ascendantDeg, midheavenDeg, trueObliquityDeg,
        model: 'IAU2006_2000B_APPROXIMATE_SIDEREAL_TIME', ut1JulianDay, ttJulianDay, latitudeDeg, longitudeDegEast });
    },
    findGeometricEvent({target,kind,ut1JulianDay,ttJulianDay,tdbJulianDay,latitudeDeg,longitudeDegEast,thresholdAltitudeDeg,windowDays,stepMinutes,observerHeightM}) {
      const values=[ut1JulianDay,ttJulianDay,tdbJulianDay,latitudeDeg,longitudeDegEast,thresholdAltitudeDeg,windowDays,stepMinutes];
      if(!TARGETS.includes(target) || !['rise','set','upper_transit','lower_transit'].includes(kind) || !values.every(Number.isFinite)) throw new TypeError('A supported target, event kind and all finite search parameters are required');
      const topocentric=observerHeightM!==undefined;
      if(topocentric && !Number.isFinite(observerHeightM)) throw new TypeError('Observer height must be finite');
      const lines=execute([topocentric?'topocentric_event':'event',kernelPath,target,kind,...values.map(String),...(topocentric?[String(observerHeightM)]:[])]);
      if(lines.length!==2 || lines[0]!==EVENT_HEADER) throw new Error('Invalid event CSV contract');
      const [eventKind,eventTarget,...parts]=lines[1].split(',');
      if(eventKind!==kind || eventTarget!==target || parts.length!==4 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part)))) throw new Error('Invalid event CSV row');
      const [eventUt1JulianDay,eventTtJulianDay,eventTdbJulianDay,bracketSeconds]=parts.map(Number);
      if(eventUt1JulianDay<ut1JulianDay || eventUt1JulianDay>ut1JulianDay+windowDays || bracketSeconds<0 || bracketSeconds>0.25) throw new Error('Out-of-range event CSV row');
      return Object.freeze({target,kind,ut1JulianDay:eventUt1JulianDay,ttJulianDay:eventTtJulianDay,tdbJulianDay:eventTdbJulianDay,bracketSeconds,model:topocentric?'EXPERIMENTAL_TOPOCENTRIC_GEOMETRIC':'EXPERIMENTAL_GEOCENTRIC_GEOMETRIC',warnings:Object.freeze(['NOT_SWISS_PARITY',...(topocentric?['NO_DIURNAL_ABERRATION','NO_OBSERVER_LIGHT_TIME']:['NO_PARALLAX']),'NO_REFRACTION','NO_LIMB','NO_UT1_INTERPOLATION','SAMPLING_MAY_MISS_CROSSINGS'])});
    },
    calculateTopocentricDirection({ut1JulianDay,ttJulianDay,latitudeDeg,longitudeDegEast,observerHeightM,rightAscensionDeg,declinationDeg,distanceAu}) {
      const values=[ut1JulianDay,ttJulianDay,latitudeDeg,longitudeDegEast,observerHeightM,rightAscensionDeg,declinationDeg,distanceAu];
      if(!values.every(Number.isFinite)) throw new TypeError('Finite epochs, observer and geocentric direction are required');
      const lines=execute(['topocentric',...values.map(String)]);
      if(lines.length!==2 || lines[0]!=='ra_deg,declination_deg') throw new Error('Invalid topocentric CSV contract');
      const parts=lines[1].split(',');
      if(parts.length!==2 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part)))) throw new Error('Invalid topocentric CSV row');
      const [rightAscensionDegResult,declinationDegResult]=parts.map(Number);
      if(rightAscensionDegResult<0 || rightAscensionDegResult>=360 || Math.abs(declinationDegResult)>90) throw new Error('Out-of-range topocentric direction');
      return Object.freeze({rightAscensionDeg:rightAscensionDegResult,declinationDeg:declinationDegResult,model:'EXPERIMENTAL_WGS84_PARALLAX',warnings:Object.freeze(['NO_DIURNAL_ABERRATION','NO_OBSERVER_LIGHT_TIME','NO_POLAR_MOTION'])});
    },
    findStandardSolarEvent({kind,ut1JulianDay,ttJulianDay,tdbJulianDay,latitudeDeg,longitudeDegEast,windowDays,stepMinutes}) {
      const values=[ut1JulianDay,ttJulianDay,tdbJulianDay,latitudeDeg,longitudeDegEast,windowDays,stepMinutes];
      if(!['rise','set'].includes(kind) || !values.every(Number.isFinite)) throw new TypeError('Rise/set and all finite search parameters are required');
      const lines=execute(['standard_solar_event',kernelPath,kind,...values.map(String)]);
      if(lines.length!==2 || lines[0]!==EVENT_HEADER) throw new Error('Invalid event CSV contract');
      const [eventKind,target,...parts]=lines[1].split(',');
      if(eventKind!==kind || target!=='Sun' || parts.length!==4 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part)))) throw new Error('Invalid event CSV row');
      const [eventUt1JulianDay,eventTtJulianDay,eventTdbJulianDay,bracketSeconds]=parts.map(Number);
      if(eventUt1JulianDay<ut1JulianDay || eventUt1JulianDay>ut1JulianDay+windowDays || bracketSeconds<0 || bracketSeconds>0.25) throw new Error('Out-of-range event CSV row');
      return Object.freeze({kind,target,ut1JulianDay:eventUt1JulianDay,ttJulianDay:eventTtJulianDay,tdbJulianDay:eventTdbJulianDay,bracketSeconds,model:'USNO_CONVENTIONAL_SOLAR_CENTER_MINUS_50_ARCMIN',warnings:Object.freeze(['NOT_SWISS_PARITY','AVERAGE_ATMOSPHERE_AND_SEMIDIAMETER','NO_TERRAIN','NO_UT1_INTERPOLATION','SAMPLING_MAY_MISS_CROSSINGS'])});
    },
    calculateStandardRefraction({trueAltitudeDeg}) {
      if(!Number.isFinite(trueAltitudeDeg)) throw new TypeError('A finite geometric altitude is required');
      const lines=execute(['noaa_refraction',String(trueAltitudeDeg)]);
      if(lines.length!==2 || lines[0]!=='true_altitude_deg,correction_deg,apparent_altitude_deg') throw new Error('Invalid refraction CSV contract');
      const parts=lines[1].split(',');
      if(parts.length!==3 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part)))) throw new Error('Invalid refraction CSV row');
      const [altitude,correctionDeg,apparentAltitudeDeg]=parts.map(Number);
      if(Math.abs(altitude-trueAltitudeDeg)>1e-9 || correctionDeg<0 || Math.abs(apparentAltitudeDeg-altitude-correctionDeg)>1e-9) throw new Error('Out-of-range refraction CSV row');
      return Object.freeze({trueAltitudeDeg:altitude,correctionDeg,apparentAltitudeDeg,model:'NOAA_STANDARD_APPROXIMATE',warnings:Object.freeze(['FIXED_ATMOSPHERE','NOT_SWISS_PARITY','NO_LOCAL_WEATHER'])});
    },
    calculateHorizontal({ut1JulianDay,ttJulianDay,latitudeDeg,longitudeDegEast,rightAscensionDeg,declinationDeg,coordinateFrame='equatorial'}) {
      const values=[ut1JulianDay,ttJulianDay,latitudeDeg,longitudeDegEast,rightAscensionDeg,declinationDeg];
      if(!values.every(Number.isFinite) || !['equatorial','ecliptic'].includes(coordinateFrame))throw new TypeError('Finite UT1, TT, location and supported direction are required');
      const lines=execute([coordinateFrame==='equatorial'?'horizontal':'ecliptic_horizontal',...values.map(String)]);
      if(lines.length!==2 || lines[0]!==HORIZONTAL_HEADER)throw new Error('Invalid horizontal CSV contract');
      const parts=lines[1].split(',');
      if(parts.length!==3 || !parts[0].trim() || !Number.isFinite(Number(parts[0])))throw new Error('Invalid horizontal CSV row');
      const trueAltitudeDeg=Number(parts[0]);
      let azimuthNorthEastDeg=null,azimuthSouthWestDeg=null;
      if(parts[1]!=='undefined' || parts[2]!=='undefined') {
        if(!parts[1].trim() || !parts[2].trim() || !Number.isFinite(Number(parts[1])) || !Number.isFinite(Number(parts[2])))throw new Error('Invalid horizontal azimuth');
        azimuthNorthEastDeg=Number(parts[1]); azimuthSouthWestDeg=Number(parts[2]);
        if(azimuthNorthEastDeg<0 || azimuthNorthEastDeg>=360 || azimuthSouthWestDeg<0 || azimuthSouthWestDeg>=360)throw new Error('Out-of-range horizontal azimuth');
      }
      if(Math.abs(trueAltitudeDeg)>90)throw new Error('Out-of-range horizontal altitude');
      return Object.freeze({trueAltitudeDeg,azimuthNorthEastDeg,azimuthSouthWestDeg,model:'IAU2006_2000B_APPROXIMATE_SIDEREAL_TIME',warnings:Object.freeze(['NO_REFRACTION','INPUT_PARALLAX_NOT_APPLIED','NO_POLAR_MOTION'])});
    },
  });
}
