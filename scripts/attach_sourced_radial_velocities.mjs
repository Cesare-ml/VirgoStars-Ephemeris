// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Caller-supplied measurements, local candidates, not identity approval.
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {prepareStarCatalog} from '../bindings/node/star-catalog.js';
const sha=b=>createHash('sha256').update(b).digest('hex');
export function attachRadialVelocities(manifestBytes,measurementsBytes){
  const manifest=JSON.parse(manifestBytes),input=JSON.parse(measurementsBytes);prepareStarCatalog(manifest);
  if(input.schema!=='virgostars-radial-velocities/1'||input.status!=='REVIEW_REQUIRED'
      ||!Array.isArray(input.measurements)||!input.measurements.length)throw new Error('invalid measurement file');
  const known=new Set(manifest.stars.map(s=>s.hip)),byHip=new Map();
  for(const row of input.measurements){
    if(!Number.isSafeInteger(row.hip)||!known.has(row.hip)||byHip.has(row.hip)
       ||typeof row.kmPerSecond!=='number'||!Number.isFinite(row.kmPerSecond)||Math.abs(row.kmPerSecond)>=299792.458
       ||typeof row.source!=='string'||!/^https:\/\//.test(row.source)||typeof row.evidence!=='string'||!row.evidence.trim())throw new Error('invalid or conflicting measurement');
    byHip.set(row.hip,row);
  }
  const stars=manifest.stars.map(star=>{
    const row=byHip.get(star.hip);if(!row)return {...star};
    if(star.radialVelocityKmS!==null)throw new Error('would overwrite an existing measurement');
    const {radialVelocityMissingReason,...copy}=star;
    return {...copy,radialVelocityKmS:row.kmPerSecond,radialVelocitySource:`${row.source}; ${row.evidence}; local measurement file SHA256 ${sha(measurementsBytes)}`};
  });
  const prior=manifest.sourceHashes?.radialVelocityMeasurementHashes
    ?? (manifest.sourceHashes?.radialVelocityMeasurementsSha256?[manifest.sourceHashes.radialVelocityMeasurementsSha256]:[]);
  if(!Array.isArray(prior)||prior.some(x=>!/^[a-f0-9]{64}$/.test(x)))throw new Error('invalid earlier radial-velocity provenance');
  const output={...manifest,reviewer:`${manifest.reviewer}; independently sourced RV values require review`,
    sourceHashes:{...manifest.sourceHashes,radialVelocityMeasurementHashes:[...new Set([...prior,sha(measurementsBytes)])]},stars};
  prepareStarCatalog(output);
  return {manifest:output,count:byHip.size};
}
function main(){
  const [manifestPath,measurementsPath,outPath,...extra]=process.argv.slice(2);
  if(!manifestPath||!measurementsPath||!outPath||extra.length||new Set([manifestPath,measurementsPath,outPath].map(p=>resolve(p))).size!==3||existsSync(outPath))throw new Error('usage: <manifest> <measurements.json> <new-manifest>');
  const a=readFileSync(manifestPath),b=readFileSync(measurementsPath);if(a.length>8e6||b.length>1e6)throw new Error('input too large');
  const result=attachRadialVelocities(a,b);
  if(sha(readFileSync(manifestPath))!==sha(a)||sha(readFileSync(measurementsPath))!==sha(b))throw new Error('input changed');
  writeFileSync(outPath,JSON.stringify(result.manifest,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({updated:result.count}));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)try{main();}catch(e){console.error(e.message);process.exitCode=1;}
