// SPDX-License-Identifier: CPAL-1.0. Apply explicit, caller-reviewed aliases to a local catalog.
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {inventorySwissStars} from './inventory_swiss_stars.mjs';
import {prepareStarCatalog} from '../bindings/node/star-catalog.js';
const sha=b=>createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
const nonempty=v=>typeof v==='string'&&v.trim().length>0;

// Source URLs and decisions are assertions; this function does not authenticate them.
export function attachReviewedAliases(manifestBytes,swissBytes,decisionBytes){
  const manifest=JSON.parse(manifestBytes),catalog=prepareStarCatalog(manifest),decisions=JSON.parse(decisionBytes);
  const swissHash=sha(swissBytes),manifestHash=sha(manifestBytes);
  if(decisions.schema!=='virgostars-reviewed-alias-decisions/1'||decisions.manifestSha256!==manifestHash
    ||decisions.swissCatalogSha256!==swissHash||manifest.sourceHashes?.swissCatalogSha256!==swissHash
    ||!nonempty(decisions.reviewer)||!Array.isArray(decisions.aliases)||!decisions.aliases.length)throw new Error('invalid decisions or source hash mismatch');
  const inventory=inventorySwissStars(swissBytes.toString('utf8'));
  const lookupUniverse=new Set([...inventory.names.map(x=>x.value),...inventory.designations.map(x=>','+x.value)].map(fold));
  const occupied=new Set(catalog.stars.flatMap(s=>[s.name,...s.aliases]).map(fold));
  const byHip=new Map(catalog.stars.filter(s=>s.hip!==undefined).map(s=>[s.hip,s]));
  const additions=new Map(),records=[];
  for(const item of decisions.aliases){
    if(!item||!Number.isSafeInteger(item.hip)||!nonempty(item.canonicalName)||!nonempty(item.lookup)
      ||!nonempty(item.sourceUrl)||!nonempty(item.rationale)||item.rationale.length>1000)throw new Error('invalid alias decision');
    let url;try{url=new URL(item.sourceUrl);}catch{throw new Error('invalid source URL');}
    if(url.protocol!=='https:'||url.username||url.password)throw new Error('invalid source URL');
    const key=fold(item.lookup),star=byHip.get(item.hip);
    if(!star||star.name!==item.canonicalName||!lookupUniverse.has(key)||occupied.has(key))throw new Error('unknown, conflicting or duplicate alias');
    if(item.warning!==null&&(!item.warning||Object.keys(item.warning).some(k=>!['code','detail'].includes(k))||!['SWISS_POSITION_MISMATCH','SWISS_LOOKUP_UNAVAILABLE','SWISS_RETURNED_DIFFERENT_NAME'].includes(item.warning.code)
      ||!nonempty(item.warning.detail)||item.warning.detail.length>500))throw new Error('invalid warning');
    occupied.add(key);const record={...item,lookup:item.lookup.trim(),sourceUrl:url.href};
    const group=additions.get(item.hip)??[];group.push(record);additions.set(item.hip,group);records.push(record);
  }
  const output={...manifest,reviewer:`${manifest.reviewer}; alias decisions: ${decisions.reviewer}`,
    sourceHashes:{...manifest.sourceHashes,reviewedAliasDecisionsSha256:sha(decisionBytes)},
    stars:manifest.stars.map(star=>{const items=additions.get(star.hip);if(!items)return star;
      return {...star,aliases:[...star.aliases,...items.map(i=>i.lookup)],
        lookupWarnings:[...(star.lookupWarnings??[]),...items.filter(i=>i.warning!==null).map(i=>({name:i.lookup,code:i.warning.code,detail:i.warning.detail}))],
        reviewedAliasSources:[...(star.reviewedAliasSources??[]),...items.map(i=>({lookup:i.lookup,sourceUrl:i.sourceUrl,rationale:i.rationale}))]};})};
  prepareStarCatalog(output);
  return {manifest:output,report:{schema:'virgostars-reviewed-alias-report/1',status:'CALLER_ASSERTED',manifestSha256:manifestHash,
    swissCatalogSha256:swissHash,decisionsSha256:sha(decisionBytes),reviewer:decisions.reviewer,aliases:records}};
}
function main(){
  const [manifestPath,swissPath,decisionsPath,outPath,reportPath,...extra]=process.argv.slice(2),paths=[manifestPath,swissPath,decisionsPath,outPath,reportPath];
  if(!paths.every(Boolean)||extra.length||new Set(paths.map(p=>resolve(p))).size!==paths.length||existsSync(outPath)||existsSync(reportPath))throw new Error('usage: <manifest> <sefstars> <decisions> <new-manifest> <new-report>');
  const bytes=paths.slice(0,3).map(p=>readFileSync(p));if(bytes.some(b=>b.length>8e6))throw new Error('input too large');
  const result=attachReviewedAliases(...bytes);
  if(paths.slice(0,3).some((p,i)=>sha(readFileSync(p))!==sha(bytes[i])))throw new Error('input changed');
  writeFileSync(outPath,JSON.stringify(result.manifest,null,2)+'\n',{flag:'wx'});
  writeFileSync(reportPath,JSON.stringify(result.report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({status:result.report.status,added:result.report.aliases.length}));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)try{main();}catch(e){console.error(e.message);process.exitCode=1;}
