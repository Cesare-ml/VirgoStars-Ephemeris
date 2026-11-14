#!/usr/bin/env node
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
    const remote=parseHorizonsVector(api.result,{expectedJdTdb:query.jdTdbRequested});
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
