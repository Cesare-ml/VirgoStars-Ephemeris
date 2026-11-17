// SPDX-License-Identifier: CPAL-1.0. Caller-supplied local photometry only.
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {prepareStarCatalog} from '../bindings/node/star-catalog.js';
const sha=b=>createHash('sha256').update(b).digest('hex');

export function updateReviewedMagnitudes(manifestBytes,decisionBytes){
  const manifest=JSON.parse(manifestBytes),catalog=prepareStarCatalog(manifest),data=JSON.parse(decisionBytes);
  if(data.schema!=='virgostars-reviewed-magnitudes/1'||data.manifestSha256!==sha(manifestBytes)
    ||typeof data.reviewer!=='string'||!data.reviewer.trim()||!Array.isArray(data.items)||!data.items.length||data.items.length>10000)throw new Error('invalid decisions or manifest hash');
  const entries=new Map(catalog.stars.filter(s=>s.sourceId!==undefined).map(s=>[s.sourceId,s])),updates=new Map();
  for(const item of data.items){
    if(!item||typeof item.sourceId!=='string'||!item.sourceId.trim()||typeof item.name!=='string'
      ||!Number.isFinite(item.magnitude)||!item.magnitudeInfo||typeof item.magnitudeInfo.source!=='string')throw new Error('invalid magnitude decision');
    const star=entries.get(item.sourceId);
    if(!star||star.name!==item.name||star.magnitude!==null||updates.has(item.sourceId))throw new Error('unknown, duplicated or already populated magnitude');
    updates.set(item.sourceId,item);
  }
  const output={...manifest,reviewer:`${manifest.reviewer}; magnitude decisions: ${data.reviewer}`,
    sourceHashes:{...manifest.sourceHashes,reviewedMagnitudeDecisionsSha256:sha(decisionBytes)},
    stars:manifest.stars.map(star=>{const item=updates.get(star.sourceId);if(!item)return star;
      const {magnitudeMissingReason,...rest}=star;
      return {...rest,magnitude:item.magnitude,magnitudeSource:item.magnitudeInfo.source,magnitudeInfo:item.magnitudeInfo};})};
  prepareStarCatalog(output);
  return {manifest:output,report:{schema:'virgostars-reviewed-magnitudes-report/1',status:'CALLER_ASSERTED',manifestSha256:sha(manifestBytes),
    decisionsSha256:sha(decisionBytes),reviewer:data.reviewer,updated:data.items.map(({sourceId,name,magnitude})=>({sourceId,name,magnitude}))}};
}
function main(){
  const [manifestPath,decisionsPath,outPath,reportPath,...extra]=process.argv.slice(2),paths=[manifestPath,decisionsPath,outPath,reportPath];
  if(!paths.every(Boolean)||extra.length||new Set(paths.map(p=>resolve(p))).size!==paths.length||existsSync(outPath)||existsSync(reportPath))throw new Error('usage: <manifest> <decisions> <new-manifest> <new-report>');
  const a=readFileSync(manifestPath),b=readFileSync(decisionsPath);if(a.length>8e6||b.length>8e6)throw new Error('input too large');
  const result=updateReviewedMagnitudes(a,b);if(sha(readFileSync(manifestPath))!==sha(a)||sha(readFileSync(decisionsPath))!==sha(b))throw new Error('input changed');
  writeFileSync(outPath,JSON.stringify(result.manifest,null,2)+'\n',{flag:'wx'});
  writeFileSync(reportPath,JSON.stringify(result.report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({status:result.report.status,updated:result.report.updated.length}));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)try{main();}catch(e){console.error(e.message);process.exitCode=1;}
