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

/** Pure development tools: compare an SPK result with a Horizons vector response. */
import { SpkError } from '../src/spk.mjs';

export function horizonsVectorUrl({ target, center, etTdbSeconds }) {
  if (!Number.isInteger(target) || !Number.isInteger(center) || !Number.isFinite(etTdbSeconds))
    throw new SpkError('INVALID_ARGUMENT','Expected NAIF target/center IDs and TDB seconds since J2000');
  const jdTdb = 2451545.0 + etTdbSeconds / 86400;
  const query = new URLSearchParams({
    format: 'json', COMMAND: `'${target}'`, CENTER: `'500@${center}'`,
    MAKE_EPHEM: 'YES', OBJ_DATA: 'NO', EPHEM_TYPE: 'VECTORS',
    REF_SYSTEM: 'ICRF', REF_PLANE: 'FRAME', VEC_CORR: 'NONE',
    OUT_UNITS: 'KM-S', VEC_TABLE: '2', CSV_FORMAT: 'YES', VEC_LABELS: 'NO',
    TIME_TYPE: 'TDB', TLIST_TYPE: 'JD', TLIST: `'${jdTdb.toFixed(12)}'`,
  });
  return Object.freeze({ url:`https://ssd.jpl.nasa.gov/api/horizons.api?${query}`,
    jdTdbRequested: jdTdb, reference: 'ICRF/J2000 geometric km and km/s',
    timeType:'TDB', center, target });
}

export function parseHorizonsVector(text, { expectedJdTdb } = {}) {
  if (expectedJdTdb !== undefined && !Number.isFinite(expectedJdTdb))
    throw new SpkError('INVALID_ARGUMENT', 'Expected a finite requested TDB Julian date');
  if (typeof text !== 'string') throw new SpkError('INVALID_HORIZONS','Expected raw Horizons result text');
  const start = text.indexOf('$$SOE'), end = text.indexOf('$$EOE');
  if (start < 0 || end < start) throw new SpkError('INVALID_HORIZONS','Horizons response has no valid ephemeris markers');
  const lines=text.slice(start+5,end).split(/\r?\n/).map(s=>s.trim()).filter(Boolean);
  if (lines.length !== 1) throw new SpkError('INVALID_HORIZONS','Expected exactly one Horizons vector row');
  const fields=lines[0].split(',').map(s=>s.trim());
  // Horizons CSV may have one trailing comma, but no missing, extra or
  // silently zero-filled numeric fields are permitted.
  if (fields.length === 9 && fields[8] === '') fields.pop();
  if (fields.length !== 8 || !fields[1])
    throw new SpkError('INVALID_HORIZONS','Expected exactly JD, date, XYZ, VXYZ columns');
  const decimal = /^[+-]?(?:\\d+(?:\\.\\d*)?|\\.\\d+)(?:[eE][+-]?\\d+)?$/;
  if (!decimal.test(fields[0]) || fields.slice(2).some(x => !decimal.test(x)))
    throw new SpkError('INVALID_HORIZONS','Missing or invalid Horizons numeric field');
  const jdTdb=Number(fields[0]);
  const numbers=fields.slice(2).map(Number);
  if(!Number.isFinite(jdTdb) || numbers.some(x=>!Number.isFinite(x)))
    throw new SpkError('INVALID_HORIZONS','Nonfinite Horizons vector value');
  // A non-matching epoch must not be used as a purported reference vector.
  // 1e-8 JD is < 1 ms and allows harmless Horizons output rounding.
  if (expectedJdTdb !== undefined && Math.abs(jdTdb - expectedJdTdb) > 1e-8)
    throw new SpkError('INVALID_HORIZONS','Horizons returned a different TDB Julian date');
  return Object.freeze({jdTdb,calendar:fields[1],positionKm:Object.freeze(numbers.slice(0,3)),
    velocityKmPerSec:Object.freeze(numbers.slice(3,6))});
}

export function compareGeometricVectors(actual,reference) {
  if (!actual || !reference || actual.frame!=='J2000' || actual.corrections!=='GEOMETRIC' ||
      !Array.isArray(actual.positionKm) || !Array.isArray(actual.velocityKmPerSec))
    throw new SpkError('INVALID_ARGUMENT','Actual state must be J2000 geometric position/velocity');
  const diff = (a,b) => {
    if (!Array.isArray(b)||a.length!==3||b.length!==3||
        [...a,...b].some(x=>!Number.isFinite(x)))
      throw new SpkError('INVALID_ARGUMENT','Invalid reference state vectors');
    const signed=a.map((x,i)=>x-b[i]);
    return Object.freeze({signed:Object.freeze(signed),norm:Math.hypot(...signed),maxAbs:Math.max(...signed.map(Math.abs))});
  };
  return Object.freeze({positionErrorKm:diff(actual.positionKm,reference.positionKm),
    velocityErrorKmPerSec:diff(actual.velocityKmPerSec,reference.velocityKmPerSec)});
}
