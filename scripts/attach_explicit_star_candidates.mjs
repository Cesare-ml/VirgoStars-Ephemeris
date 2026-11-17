// SPDX-License-Identifier: CPAL-1.0. Caller-supplied astronomical data only.
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {prepareStarCatalog} from '../bindings/node/star-catalog.js';
const sha=b=>createHash('sha256').update(b).digest('hex');
export function attachExplicitStars(manifestBytes,candidateBytes){
  const manifest=JSON.parse(manifestBytes),data=JSON.parse(candidateBytes);prepareStarCatalog(manifest);
  if(manifest.qualityPolicy!=='experimental'||!['virgostars-explicit-star-candidates/1','virgostars-explicit-star-candidates/2'].includes(data.schema)
    ||data.status!=='REVIEW_REQUIRED'||!Array.isArray(data.stars)||!data.stars.length||data.stars.length>10000)
    throw new Error('invalid policy or candidate file');
  if(data.schema==='virgostars-explicit-star-candidates/2'
    &&(data.manifestSha256!==sha(manifestBytes)||data.swissCatalogSha256!==manifest.sourceHashes?.swissCatalogSha256
      ||!/^[a-f0-9]{64}$/.test(data.swissCatalogSha256||'')))throw new Error('candidate source hash mismatch');
  for(const star of data.stars)if(star.hip!==undefined||star.qualityTier!=='experimental')throw new Error('direct candidates must use experimental non-HIP entries');
  const output={...manifest,schema:'virgostars-star-manifest/2',reviewer:`${manifest.reviewer}; explicit astrometry requires review`,
    sourceHashes:{...manifest.sourceHashes,explicitStarCandidatesSha256:sha(candidateBytes)},stars:[...manifest.stars,...data.stars]};
  prepareStarCatalog(output);
  return {manifest:output,count:data.stars.length};
}
function main(){
  const [manifestPath,candidatesPath,outPath,...extra]=process.argv.slice(2);
  if(!manifestPath||!candidatesPath||!outPath||extra.length||new Set([manifestPath,candidatesPath,outPath].map(p=>resolve(p))).size!==3||existsSync(outPath))throw new Error('usage: <manifest> <explicit-candidates.json> <new-manifest>');
  const a=readFileSync(manifestPath),b=readFileSync(candidatesPath);if(a.length>8e6||b.length>8e6)throw new Error('input too large');
  const r=attachExplicitStars(a,b);if(sha(readFileSync(manifestPath))!==sha(a)||sha(readFileSync(candidatesPath))!==sha(b))throw new Error('input changed');
  writeFileSync(outPath,JSON.stringify(r.manifest,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({added:r.count,total:r.manifest.stars.length}));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)try{main();}catch(e){console.error(e.message);process.exitCode=1;}
