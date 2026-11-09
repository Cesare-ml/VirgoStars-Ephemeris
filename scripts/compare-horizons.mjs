#!/usr/bin/env node
/** Manual, ONLINE scientific comparison. No automatic downloads or CI. */
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { inspectSpk, stateAtTdbSeconds } from '../src/index.mjs';
import { horizonsVectorUrl, parseHorizonsVector, compareGeometricVectors } from '../tools/horizons.mjs';

const [file, etString, targetString, centerString] = process.argv.slice(2);
if (![file,etString,targetString,centerString].every(x=>x!==undefined)) {
  console.error('Usage: node scripts/compare-horizons.mjs /path/to/de440s.bsp ET_TDB_SECONDS TARGET_NAIF_ID CENTER_NAIF_ID');
  process.exitCode=2;
} else {
  try {
    const et=Number(etString),target=Number(targetString),center=Number(centerString);
    const bytes=await readFile(file);
    const sha256=createHash('sha256').update(bytes).digest('hex');
    const spk=inspectSpk(bytes);
    const local=stateAtTdbSeconds(bytes,target,center,et);
    const query=horizonsVectorUrl({target,center,etTdbSeconds:et});
    const http=await fetch(query.url,{signal:AbortSignal.timeout(30000)});
    if(!http.ok) throw new Error(`Horizons HTTP ${http.status}`);
    const api=await http.json();
    if(typeof api.result!=='string') throw new Error('Horizons API did not return its expected text result');
    const remote=parseHorizonsVector(api.result);
    const difference=compareGeometricVectors(local,remote);
    const payload={
      status:'COMPARISON_ONLY_NOT_A_CERTIFICATION',
      sourceKernel:file, sourceSha256:sha256,
      spkInternalName:spk.internalName,
      kernelTypeCoverage:[...new Set(spk.segments.map(s=>s.type))],
      epochTdbSeconds:et, jdTdbRequested:query.jdTdbRequested,
      horizonsJdTdbReturned:remote.jdTdb,
      requestUrl:query.url, reference:query.reference,
      local, horizons:remote, differences:difference,
      warning:'Horizons might use a DIFFERENT DE solution than the local SPK. Results are NOT apparent/zodiacal positions. Validate dataset family, frame, timescale and coverage before assigning tolerances.',
    };
    console.log(JSON.stringify(payload,null,2));
  }catch(error){
    console.error(JSON.stringify({code:error.code??'VALIDATION_FAILED',message:error.message}));
    process.exitCode=1;
  }
}
