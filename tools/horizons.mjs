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

export function parseHorizonsVector(text) {
  if (typeof text !== 'string') throw new SpkError('INVALID_HORIZONS','Expected raw Horizons result text');
  const start = text.indexOf('$$SOE'), end = text.indexOf('$$EOE');
  if (start < 0 || end < start) throw new SpkError('INVALID_HORIZONS','Horizons response has no valid ephemeris markers');
  const lines=text.slice(start+5,end).split(/\r?\n/).map(s=>s.trim()).filter(Boolean);
  if (lines.length !== 1) throw new SpkError('INVALID_HORIZONS','Expected exactly one Horizons vector row');
  const fields=lines[0].split(',').map(s=>s.trim());
  if(fields.length < 8) throw new SpkError('INVALID_HORIZONS','Expected JD, date, XYZ, VXYZ columns');
  const jdTdb=Number(fields[0]);
  const numbers=fields.slice(2,8).map(Number);
  if(!Number.isFinite(jdTdb) || numbers.some(x=>!Number.isFinite(x)))
    throw new SpkError('INVALID_HORIZONS','Nonfinite Horizons vector value');
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
