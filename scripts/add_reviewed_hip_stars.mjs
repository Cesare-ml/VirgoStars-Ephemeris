// SPDX-License-Identifier: CPAL-1.0. Caller-asserted research candidates only.
import {createHash} from 'node:crypto';
import {existsSync,readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {inventorySwissStars} from './inventory_swiss_stars.mjs';
import {prepareStarCatalog} from '../bindings/node/star-catalog.js';
import {inspectHipparcosQuality} from '../bindings/node/index.js';
const sha=b=>createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');

export function addReviewedHipStars(manifestBytes,swissBytes,hipBytes,decisionBytes,hipPath,inspect=inspectHipparcosQuality){
  const manifest=JSON.parse(manifestBytes),catalog=prepareStarCatalog(manifest),decision=JSON.parse(decisionBytes);
  const hashes={manifestSha256:sha(manifestBytes),swissCatalogSha256:sha(swissBytes),hip2Sha256:sha(hipBytes)};
  if(catalog.qualityPolicy!=='experimental'||decision.schema!=='virgostars-reviewed-hip-stars/1'
    ||Object.entries(hashes).some(([k,v])=>decision[k]!==v)
    ||manifest.sourceHashes?.swissCatalogSha256!==hashes.swissCatalogSha256||manifest.sourceHashes?.hip2Sha256!==hashes.hip2Sha256
    ||typeof decision.reviewer!=='string'||!decision.reviewer.trim()||!Array.isArray(decision.stars)||!decision.stars.length)throw new Error('invalid decisions, policy or hashes');
  const inventory=inventorySwissStars(swissBytes.toString('utf8'));
  const universe=new Set([...inventory.names.map(x=>x.value),...inventory.designations.map(x=>','+x.value)].map(fold));
  const hips=decision.stars.map(s=>s.hip);
  if(hips.some(h=>!Number.isSafeInteger(h)||h<=0||catalog.hips.includes(h))||new Set(hips).size!==hips.length)throw new Error('invalid or existing HIP');
  for(const star of decision.stars){
    if(typeof star.name!=='string'||!Array.isArray(star.aliases)||[star.name,...star.aliases].some(n=>typeof n!=='string'||!universe.has(fold(n))))throw new Error('lookup outside inventory');
  }
  const quality=inspect(hipPath,hips),byHip=new Map(quality.map(q=>[q.hip,q]));
  if(quality.length!==hips.length||byHip.size!==hips.length||hips.some(h=>!byHip.has(h)))throw new Error('incomplete HIP quality');
  for(const star of decision.stars){
    const q=byHip.get(star.hip);
    if(['strictEligible','traditionalEligible','experimentalEligible'].some(k=>typeof q[k]!=='boolean')
      ||!q.experimentalEligible||(q.strictEligible&&!q.traditionalEligible)||(q.traditionalEligible&&!q.experimentalEligible))throw new Error('ineligible HIP quality');
    const tier=q.strictEligible?'strict':q.traditionalEligible?'traditional':'experimental';
    if(star.qualityTier!==tier)throw new Error('incorrect HIP tier');
  }
  const output={...manifest,reviewer:`${manifest.reviewer}; added HIP candidates: ${decision.reviewer}`,
    sourceHashes:{...manifest.sourceHashes,reviewedHipDecisionsSha256:sha(decisionBytes)},stars:[...manifest.stars,...decision.stars]};
  prepareStarCatalog(output);
  return {manifest:output,report:{schema:'virgostars-reviewed-hip-report/1',status:'CALLER_ASSERTED',...hashes,
    decisionsSha256:sha(decisionBytes),reviewer:decision.reviewer,added:decision.stars.map(s=>({hip:s.hip,name:s.name,aliases:s.aliases,qualityTier:s.qualityTier})),quality}};
}
function main(){
  const [manifestPath,swissPath,hipPath,decisionPath,outPath,reportPath,...extra]=process.argv.slice(2),paths=[manifestPath,swissPath,hipPath,decisionPath,outPath,reportPath];
  if(!paths.every(Boolean)||extra.length||new Set(paths.map(p=>resolve(p))).size!==paths.length||existsSync(outPath)||existsSync(reportPath))throw new Error('usage: <manifest> <sefstars> <hip2> <decisions> <new-manifest> <new-report>');
  const inputs=paths.slice(0,4).map(p=>readFileSync(p));if(inputs.some((b,i)=>b.length>(i===2?1e8:8e6)))throw new Error('input too large');
  const result=addReviewedHipStars(...inputs,hipPath);
  if(paths.slice(0,4).some((p,i)=>sha(readFileSync(p))!==sha(inputs[i])))throw new Error('input changed');
  writeFileSync(outPath,JSON.stringify(result.manifest,null,2)+'\n',{flag:'wx'});
  writeFileSync(reportPath,JSON.stringify(result.report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({status:result.report.status,added:result.report.added.length}));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)try{main();}catch(e){console.error(e.message);process.exitCode=1;}
