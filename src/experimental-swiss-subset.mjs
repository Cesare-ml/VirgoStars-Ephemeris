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

/** Caller supplies the classic JPL kernel, compiled Rust executable and a
 * synchronous UT1→{TT,TDB} provider for the same instant. No time conversion,
 * file-path inference, missing-body fallback or unsupported flag is invented.
 */
export function createExperimentalSwissSubset({executablePath,kernelPath,convertUt1ToTtTdb}) {
  if(typeof convertUt1ToTtTdb!=='function')throw new TypeError('An explicit UT1 to TT/TDB converter is required');
  const provider=createExperimentalRustCli({executablePath,kernelPath});
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
      if(flags!==SEFLG_JPLEPH && flags!==(SEFLG_JPLEPH|SEFLG_SPEED))throw new RangeError('Only explicit JPL and optional speed flags are supported');
      if(callback!==undefined && typeof callback!=='function')throw new TypeError('Callback must be a function');
      const epochs=convertUt1ToTtTdb(julianDay);
      if(!epochs || typeof epochs!=='object')throw new TypeError('Converter must supply finite TT and TDB Julian days');
      const {ttJulianDay,tdbJulianDay}=epochs;
      if(!Number.isFinite(ttJulianDay) || !Number.isFinite(tdbJulianDay) || Math.abs(ttJulianDay-julianDay)>0.1 || Math.abs(ttJulianDay-tdbJulianDay)>1/86400)throw new RangeError('UT1, TT and TDB must describe the same instant');
      const state=provider.calculateAtEpoch({target:PHYSICAL_TARGETS[body],ttJulianDay,tdbJulianDay});
      const speed=(flags&SEFLG_SPEED)!==0;
      const value=Object.freeze({longitude:state.longitudeDeg,latitude:state.latitudeDeg,distance:state.distanceAu,
        longitudeSpeed:speed?state.longitudeSpeedDegPerTdbDay:0,latitudeSpeed:speed?state.latitudeSpeedDegPerTdbDay:0,distanceSpeed:speed?state.distanceSpeedAuPerTdbDay:0,
        rflag:flags,model:'EXPERIMENTAL_DE441_IAU2006_2000B',warnings});
      if(callback!==undefined)callback(value);
      return value;
    },
  };
  return Object.freeze(api);
}
