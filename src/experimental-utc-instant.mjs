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

/** Single-instant UTC adapter for the Rust experimental CLI, added 2026-11-15. */
import {spawnSync} from 'node:child_process';
import {isAbsolute} from 'node:path';

/** All offsets must already be known for this exact UTC label. */
export function createExplicitUtcInstant({executablePath,datetimeUtc,ut1MinusUtcSeconds,taiMinusUtcSeconds,tdbMinusTtSeconds,sourceId}){
  if(typeof executablePath!=='string' || !isAbsolute(executablePath))throw new TypeError('Absolute Rust executable path required');
  if(typeof sourceId!=='string' || !sourceId.trim() || sourceId!==sourceId.trim())throw new TypeError('Explicit time-offset source ID required');
  if(typeof datetimeUtc!=='string' || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(datetimeUtc))throw new TypeError('UTC must be a canonical ISO timestamp with milliseconds');
  const unixMilliseconds=Date.parse(datetimeUtc);
  if(!Number.isSafeInteger(unixMilliseconds) || new Date(unixMilliseconds).toISOString()!==datetimeUtc || unixMilliseconds<Date.UTC(1972,0,1) || unixMilliseconds>=Date.UTC(2100,0,1))throw new RangeError('Unsupported UTC instant or leap-second label');
  if(!Number.isFinite(ut1MinusUtcSeconds) || Math.abs(ut1MinusUtcSeconds)>0.9 || !Number.isInteger(taiMinusUtcSeconds) || taiMinusUtcSeconds<10 || taiMinusUtcSeconds>100 || !Number.isFinite(tdbMinusTtSeconds) || Math.abs(tdbMinusTtSeconds)>0.01)throw new RangeError('Explicit supported UTC, TAI and TDB offsets required');
  const child=spawnSync(executablePath,['utc_instant',String(unixMilliseconds),String(ut1MinusUtcSeconds),String(taiMinusUtcSeconds),String(tdbMinusTtSeconds)],{encoding:'utf8',windowsHide:true,timeout:30000,maxBuffer:65536});
  if(child.error)throw new Error(`Time conversion process failed: ${child.error.message}`,{cause:child.error});
  if(child.status!==0)throw new Error(`Time conversion process exited with status ${child.status}: ${(child.stderr||'').trim()}`);
  const lines=child.stdout.trim().split(/\r?\n/);
  if(lines.length!==2 || lines[0]!=='unix_milliseconds,ut1_jd,tt_jd,tdb_jd')throw new Error('Invalid explicit instant CSV contract');
  const parts=lines[1].split(',');
  if(parts.length!==4 || parts.some(part=>!part.trim() || !Number.isFinite(Number(part))))throw new Error('Invalid explicit instant CSV row');
  const [reportedMilliseconds,ut1JulianDay,ttJulianDay,tdbJulianDay]=parts.map(Number);
  if(reportedMilliseconds!==unixMilliseconds || Math.abs((ttJulianDay-ut1JulianDay)*86400-(taiMinusUtcSeconds+32.184-ut1MinusUtcSeconds))>.00012 || Math.abs((tdbJulianDay-ttJulianDay)*86400-tdbMinusTtSeconds)>.00012)throw new Error('Incoherent explicit instant CSV row');
  const requireInstant=jd=>{if(jd!==ut1JulianDay)throw new RangeError('Converter is valid only for its configured UT1 instant');};
  const epochs=Object.freeze({ttJulianDay,tdbJulianDay});
  return Object.freeze({datetimeUtc,unixMilliseconds,ut1JulianDay,ttJulianDay,tdbJulianDay,sourceId,
    warnings:Object.freeze(['OFFSETS_CALLER_SUPPLIED_NOT_VERIFIED','UTC_LEAP_SECOND_LABELS_UNSUPPORTED','BINARY64_JULIAN_DAY_PRECISION']),
    convertUt1ToTtTdb(jd){requireInstant(jd);return epochs;},
    convertUt1ToUtc(jd){requireInstant(jd);return datetimeUtc;},
  });
}
