// SPDX-License-Identifier: CPAL-1.0. Local caller-asserted comparison warnings.
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {prepareStarCatalog} from '../bindings/node/star-catalog.js';
const hash=b=>createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
export function attachCenturyWarnings(manifestBytes,reportBytes){
  const manifest=JSON.parse(manifestBytes),report=JSON.parse(reportBytes),catalog=prepareStarCatalog(manifest);
  if(report.schema!=='virgostars-star-century-diagnostic/1'||report.status!=='REVIEW_REQUIRED'||report.manifestSha256!==hash(manifestBytes)
    ||report.hip2Sha256!==manifest.sourceHashes.hip2Sha256||report.swissCatalogSha256!==manifest.sourceHashes.swissCatalogSha256
    ||!Array.isArray(report.epochs)||report.epochs.map(e=>e.year).join(',')!=='1900,1925,1950,1975,2000,2025,2050,2075,2099'
    ||report.summary?.stars!==catalog.stars.length||report.summary.thresholdArcsec!==1||!Array.isArray(report.failures)
    ||report.summary.failedCases!==report.failures.length)throw new Error('invalid or stale diagnostic');
  const keyOf=s=>s.hip===undefined?`source:${s.sourceId}`:`hip:${s.hip}`;
  const byId=new Map(catalog.stars.map(s=>[keyOf(s),s])),cases=new Set(),groups=new Map();
  for(const f of report.failures){
    const id=keyOf(f),star=byId.get(id),caseKey=`${id}:\0${fold(f.name)}:\0${f.year}`;
    if(!star||![star.name,...star.aliases].some(name=>name===f.name)||!report.epochs.some(e=>e.year===f.year)||cases.has(caseKey))throw new Error('invalid diagnostic case');
    cases.add(caseKey);
    if(f.declaredWarning){
      if(!star.lookupWarnings?.some(w=>fold(w.name)===fold(f.name)&&w.code===f.declaredWarning)
        ||(f.reason==='POSITION_MISMATCH'&&f.declaredWarning!=='SWISS_POSITION_MISMATCH'))throw new Error('invalid declared warning');
      continue;
    }
    if(f.reason!=='POSITION_MISMATCH'||![f.longitudeArcsec,f.latitudeArcsec].every(v=>Number.isFinite(v)&&v>=0)
       ||Math.max(f.longitudeArcsec,f.latitudeArcsec)<=1)throw new Error('unexpected error cannot be converted to a position warning');
    const key=`${id}\0${fold(f.name)}`,group=groups.get(key)??[];group.push(f);groups.set(key,group);
  }
  const stars=manifest.stars.map(star=>{
    const relevant=[...groups.entries()].filter(([key])=>key.startsWith(`${keyOf(star)}\0`));
    if(!relevant.length)return {...star};
    const warnings=[...(star.lookupWarnings??[])];
    for(const [,rows] of relevant){
      const name=rows[0].name,max=Math.max(...rows.flatMap(f=>[f.longitudeArcsec,f.latitudeArcsec]));
      const warning={name,code:'SWISS_POSITION_MISMATCH',
        detail:`Position differs from Swiss by up to ${max.toFixed(3)} arcsec at sampled epochs ${rows.map(x=>x.year).join(', ')}; both use DE441.`};
      const index=warnings.findIndex(w=>fold(w.name)===fold(name));
      if(index>=0)warnings[index]=warning;else warnings.push(warning);
    }
    return {...star,lookupWarnings:warnings};
  });
  const output={...manifest,reviewer:`${manifest.reviewer}; century comparison warnings require review`,
    sourceHashes:{...manifest.sourceHashes,centuryDiagnosticSha256:hash(reportBytes)},stars};prepareStarCatalog(output);
  return {manifest:output,added:groups.size};
}
function main(){
  const [manifestPath,reportPath,outPath,...extra]=process.argv.slice(2);
  if(!manifestPath||!reportPath||!outPath||extra.length||new Set([manifestPath,reportPath,outPath].map(p=>resolve(p))).size!==3||existsSync(outPath))throw new Error('usage: <manifest> <century-report> <new-manifest>');
  const a=readFileSync(manifestPath),b=readFileSync(reportPath);if(a.length>8e6||b.length>8e6)throw new Error('input too large');
  const r=attachCenturyWarnings(a,b);if(hash(readFileSync(manifestPath))!==hash(a)||hash(readFileSync(reportPath))!==hash(b))throw new Error('input changed');
  writeFileSync(outPath,JSON.stringify(r.manifest,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({warningsAdded:r.added}));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)try{main();}catch(e){console.error(e.message);process.exitCode=1;}
