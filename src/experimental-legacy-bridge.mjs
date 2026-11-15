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

/** Explicit legacy-shaped bridge, not a drop-in Swiss replacement. */
import {realpathSync,statSync} from 'node:fs';
import {isAbsolute} from 'node:path';
import {createExperimentalSwissSubset,SEFLG_JPLEPH,SEFLG_SWIEPH,SE_ECL_NUT,SE_GREG_CAL,SE_JUL_CAL} from './experimental-swiss-subset.mjs';

const SOURCE_MASK=7;
function canonicalDirectory(path){
  if(typeof path!=='string' || !isAbsolute(path))throw new TypeError('Explicit absolute ephemeris directory required');
  const resolved=realpathSync(path);
  if(!statSync(resolved).isDirectory())throw new TypeError('Ephemeris directory must be a directory');
  return resolved;
}
const comparable=path=>process.platform==='win32'?path.toLowerCase():path;
const checkCallback=callback=>{if(callback!==undefined && typeof callback!=='function')throw new TypeError('Callback must be a function');};

/** The policy changes the requested source explicitly; rflag remains JPL. */
function translateFlags(flags){
  if(!Number.isInteger(flags) || flags<0 || flags>0x7fffffff)throw new RangeError('Invalid legacy flags');
  const source=flags&SOURCE_MASK;
  if(source===SEFLG_JPLEPH)return {effective:flags,warning:null};
  if(source===SEFLG_SWIEPH)return {effective:(flags&~SOURCE_MASK)|SEFLG_JPLEPH,warning:'LEGACY_SWIEPH_REQUEST_REPLACED_BY_JPL'};
  if(source===0)return {effective:flags|SEFLG_JPLEPH,warning:'LEGACY_DEFAULT_SOURCE_REPLACED_BY_JPL'};
  throw new RangeError('Mixed or unsupported legacy ephemeris source flags');
}

function annotate(value,requestedFlags,warning,effectiveFlags){
  if(!warning)return value;
  if(!value || typeof value!=='object' || !Array.isArray(value.warnings))throw new Error('Missing experimental result warnings');
  const descriptors=Object.getOwnPropertyDescriptors(value);
  descriptors.warnings.value=Object.freeze([...value.warnings,warning]);
  descriptors.requestedFlags={value:requestedFlags,enumerable:false,writable:false,configurable:false};
  if(!Object.hasOwn(value,'rflag'))descriptors.effectiveFlags={value:effectiveFlags,enumerable:false,writable:false,configurable:false};
  const output=Object.defineProperties({},descriptors);
  return Object.isFrozen(value)?Object.freeze(output):output;
}

/** Explicit configuration enables one instance; no global paths are set. */
export function createExperimentalLegacyBridge(options){
  if(!options || typeof options!=='object' || Array.isArray(options))throw new TypeError('Explicit legacy bridge configuration required');
  const {ephemerisDirectory,defaultCalendar,flagPolicy,mutableResults,...subsetOptions}=options;
  if(flagPolicy!=='MAP_SWISS_AND_DEFAULT_TO_JPL')throw new TypeError('Explicit legacy source-flag policy is required');
  if(!['GREGORIAN','JULIAN'].includes(defaultCalendar))throw new TypeError('Explicit default calendar is required');
  if(!['LEGACY','FROZEN'].includes(mutableResults))throw new TypeError('Explicit result mutability policy is required');
  if(Object.hasOwn(subsetOptions,'legacyMutableResults'))throw new TypeError('Use the explicit mutability policy');
  if(typeof subsetOptions.executablePath!=='string' || !isAbsolute(subsetOptions.executablePath) || typeof subsetOptions.kernelPath!=='string' || !isAbsolute(subsetOptions.kernelPath))throw new TypeError('Absolute executable and classic JPL kernel paths are required');
  const directory=canonicalDirectory(ephemerisDirectory);
  const engine=createExperimentalSwissSubset({...subsetOptions,legacyMutableResults:mutableResults==='LEGACY'});
  const invoke=(method,args,flags,callback)=>{
    checkCallback(callback);
    const mapping=translateFlags(flags);
    const result=method(...args,mapping.effective);
    const value=annotate(result,flags,mapping.warning,mapping.effective);
    if(callback!==undefined)callback(value);
    return value;
  };
  return Object.freeze({...engine,
    // Legacy callers use this Swiss ID as an inclusive iteration bound. Its
    // presence does not make this or any other unsupported target calculable.
    SE_PLUTO_PICKERING:54,
    swe_set_ephe_path(path){
      const requested=canonicalDirectory(path);
      if(comparable(requested)!==comparable(directory))throw new RangeError('Path differs from the explicitly configured legacy directory');
      return directory;
    },
    swe_julday(year,month,day,hour,calendarFlag){
      return engine.swe_julday(year,month,day,hour,calendarFlag===undefined?(defaultCalendar==='GREGORIAN'?SE_GREG_CAL:SE_JUL_CAL):calendarFlag);
    },
    swe_calc_ut(jd,body,flags,callback){
      if(body===SE_ECL_NUT && flags===0)return engine.swe_calc_ut(jd,body,flags,callback);
      return invoke(engine.swe_calc_ut,[jd,body],flags,callback);
    },
    swe_fixstar_ut(name,jd,flags,callback){return invoke(engine.swe_fixstar_ut,[name,jd],flags,callback);},
    swe_rise_trans(jd,body,starName,flags,event,longitude,latitude,height,pressure,temperature,callback){
      checkCallback(callback);
      const mapping=translateFlags(flags);
      const result=engine.swe_rise_trans(jd,body,starName,mapping.effective,event,longitude,latitude,height,pressure,temperature);
      const value=annotate(result,flags,mapping.warning,mapping.effective);
      if(callback!==undefined)callback(value);
      return value;
    },
  });
}
