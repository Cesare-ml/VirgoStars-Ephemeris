// SPDX-License-Identifier: CPAL-1.0. Local coverage gate, not scientific approval.
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {inventorySwissStars} from './inventory_swiss_stars.mjs';
import {prepareStarCatalog} from '../bindings/node/star-catalog.js';
const sha=b=>createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
export function verifySwissCoverage(manifestBytes,swissBytes){
  const manifest=JSON.parse(manifestBytes),catalog=prepareStarCatalog(manifest);
  if(manifest.sourceHashes?.swissCatalogSha256!==sha(swissBytes))throw new Error('Swiss source hash mismatch');
  const inventory=inventorySwissStars(swissBytes.toString('utf8'));
  const available=new Map(catalog.stars.flatMap(s=>[s.name,...s.aliases].map(name=>[fold(name),s])));
  const names=inventory.names.map(x=>x.value),designations=inventory.designations.map(x=>','+x.value);
  const missingNames=names.filter(x=>!available.has(fold(x)));
  const missingDesignations=designations.filter(x=>!available.has(fold(x)));
  // Lookup coverage is not magnitude coverage: swe_fixstar_mag fails for null.
  // Count lookup strings separately from catalog objects, including aliases.
  const matching=(items,predicate)=>items.filter(x=>{const star=available.get(fold(x));return star&&predicate(star);});
  const fieldCoverage={
    starsWithoutMagnitude:catalog.stars.filter(s=>s.magnitude===null).length,
    starsAssumingZeroRadialVelocity:catalog.stars.filter(s=>s.radialVelocityKmS===null).length,
    namesWithoutMagnitude:matching(names,s=>s.magnitude===null),
    designationsWithoutMagnitude:matching(designations,s=>s.magnitude===null),
    namesAssumingZeroRadialVelocity:matching(names,s=>s.radialVelocityKmS===null),
    designationsAssumingZeroRadialVelocity:matching(designations,s=>s.radialVelocityKmS===null),
    notes:'Only matched catalog metadata is inspected; runtime data loading, positional accuracy and source decisions are not checked.'
  };
  const warnings=catalog.stars.flatMap(s=>(s.lookupWarnings??[]).map(w=>({name:w.name,code:w.code,status:'CALLER_ASSERTED'})));
  const counts={stars:catalog.stars.length,aliases:catalog.stars.reduce((n,s)=>n+s.aliases.length,0),
    swissNames:inventory.names.length,coveredNames:inventory.names.length-missingNames.length,missingNames:missingNames.length,
    swissDesignations:inventory.designations.length,coveredDesignations:inventory.designations.length-missingDesignations.length,
    missingDesignations:missingDesignations.length,declaredWarnings:warnings.length};
  return {schema:'virgostars-swiss-coverage-gate/1',status:missingNames.length||missingDesignations.length?'INCOMPLETE':'COMPLETE_LOOKUP_COVERAGE_ONLY',
    notes:'Exact lookup strings only; not a physical-identity, accuracy, deployment or data-use approval.',
    manifestSha256:sha(manifestBytes),swissCatalogSha256:sha(swissBytes),counts,missingNames,missingDesignations,warnings,fieldCoverage};
}
function main(){
  const [manifestPath,swissPath,outPath,...extra]=process.argv.slice(2);
  if(!manifestPath||!swissPath||!outPath||extra.length||new Set([manifestPath,swissPath,outPath].map(p=>resolve(p))).size!==3||existsSync(outPath))throw new Error('usage: <manifest> <sefstars> <new-report>');
  const a=readFileSync(manifestPath),b=readFileSync(swissPath);if(a.length>8e6||b.length>8e6)throw new Error('input too large');
  const r=verifySwissCoverage(a,b);if(sha(readFileSync(manifestPath))!==sha(a)||sha(readFileSync(swissPath))!==sha(b))throw new Error('source changed');
  writeFileSync(outPath,JSON.stringify(r,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({status:r.status,...r.counts,
    starsWithoutMagnitude:r.fieldCoverage.starsWithoutMagnitude,starsAssumingZeroRadialVelocity:r.fieldCoverage.starsAssumingZeroRadialVelocity}));
  if(r.status!=='COMPLETE_LOOKUP_COVERAGE_ONLY')process.exitCode=2;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)try{main();}catch(e){console.error(e.message);process.exitCode=1;}
