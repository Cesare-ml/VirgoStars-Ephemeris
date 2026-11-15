// Original VirgoStars source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// 2026-11-15: read-only cross-repository, operator-invoked fixed-star coverage audit.
// Neither imports application source code nor copies Swiss/Astrodienst catalog measurements.
import {readFileSync,writeFileSync} from 'node:fs';
import {isAbsolute,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const normalize=name=>{
  if(typeof name!=='string' || !name.trim() || name!==name.trim())
    throw new TypeError('Star names must be nonempty trimmed strings');
  return name.toLocaleLowerCase('en-US');
};

export function readLegacyStarNames(helperSource){
  if(typeof helperSource!=='string')throw new TypeError('Expected legacy helper source as text');
  const mapping=helperSource.match(/\b(?:const|var)\s+STARS_ASTRODIENST_TO_COW_ID\s*=\s*(\{[\s\S]*?\})\s*;/);
  const list=helperSource.match(/\b(?:const|var)\s+STARS_ASTRODIENST\s*=\s*(\[[\s\S]*?\])\s*;/);
  if(!mapping || !list)throw new Error('Expected explicit STARS_ASTRODIENST map and array; backend contract changed');
  let ids;
  try{ids=JSON.parse(mapping[1]);}catch{throw new Error('Legacy ID map is not a literal JSON object');}
  if(!ids || typeof ids!=='object' || Array.isArray(ids))throw new Error('Invalid legacy ID map');
  const names=[];
  // Recognize only a simple quoted list. Never eval/require application code.
  for(const raw of list[1].slice(1,-1).split(/\r?\n/)){
    const text=raw.trim();
    if(!text || text.startsWith('//'))continue;
    const m=text.match(/^'([^'\\]+)'\s*,?$/);
    if(!m)throw new Error('Unsupported legacy star list syntax; do not guess names');
    names.push(m[1]);
  }
  const unique=new Set(), mapped=[], unmapped=[];
  for(const name of names){
    const key=normalize(name);
    if(unique.has(key))throw new Error('Duplicate backend star name');
    unique.add(key);
    if(Object.hasOwn(ids,name)){
      if(!Number.isSafeInteger(ids[name]) || ids[name]<=0)throw new Error('Invalid mapped backend star ID');
      mapped.push(name);
    }else unmapped.push(name);
  }
  const orphanIds=Object.keys(ids).filter(name=>!unique.has(normalize(name)));
  return {declared:names.length,mapped,unmapped,orphanIds};
}

export function checkFixedStarCoverage(helperSource, candidateManifest, extendedManifest, identityReviewManifest){
  const legacy=readLegacyStarNames(helperSource);
  if(!candidateManifest || candidateManifest.schemaVersion!==1 || !Array.isArray(candidateManifest.stars))
    throw new TypeError('Expected versioned candidate star manifest');
  const candidateNames=new Set();
  for(const star of candidateManifest.stars){
    if(!star || typeof star!=='object')throw new TypeError('Invalid star candidate');
    const name=normalize(star.name);
    if(candidateNames.has(name))throw new Error('Duplicate candidate star name');
    candidateNames.add(name);
    if(star.catalogReady===true)throw new Error('Target manifest must not declare catalogReady; require separate scientific acceptance evidence');
  }
  const extendedNames=new Set();
  if(extendedManifest!==undefined){
    if(!extendedManifest || extendedManifest.schemaVersion!==1 || !Array.isArray(extendedManifest.objects))
      throw new TypeError('Expected versioned extended-object manifest');
    for(const object of extendedManifest.objects){
      if(!object || typeof object!=='object' || !object.name || !['OPEN_CLUSTER','GLOBULAR_CLUSTER','AMBIGUOUS_NEBULAR_REGION'].includes(object.astronomicalType))
        throw new TypeError('Invalid extended-object target or type');
      const name=normalize(object.name);
      if(extendedNames.has(name) || candidateNames.has(name))throw new Error('Duplicate or overlapping stellar and nonstellar target name');
      extendedNames.add(name);
      if(object.status==='READY' || object.catalogReady===true)
        throw new Error('Extended-object target must remain explicitly unvalidated');
    }
  }
  const reviewNames=new Set();
  if(identityReviewManifest!==undefined){
    if(!identityReviewManifest || identityReviewManifest.schemaVersion!==1 ||
       identityReviewManifest.reviewStatus!=='NO_AUTOMATIC_RESOLUTION' ||
       !Array.isArray(identityReviewManifest.names))
      throw new TypeError('Expected explicit manual-review identity manifest');
    for(const entry of identityReviewManifest.names){
      if(!entry || typeof entry!=='object' || entry.status!=='REQUIRES_LEGACY_SEMANTICS_REVIEW' ||
         entry.catalogReady!==false || typeof entry.reason!=='string' || entry.reason.length<20 ||
         !Array.isArray(entry.options) || !entry.options.length)
        throw new TypeError('Identity review entries must be non-ready and explain unresolved aliases');
      const name=normalize(entry.name);
      if(reviewNames.has(name) || candidateNames.has(name) || extendedNames.has(name))
        throw new Error('Duplicate or overlapping manual-review target');
      reviewNames.add(name);
      for(const option of entry.options){
        if(!option || typeof option.designation!=='string' || !option.designation ||
           (option.hip!==null && (!Number.isSafeInteger(option.hip) || option.hip<=0)) ||
           typeof option.sourceUrl!=='string' || !option.sourceUrl.startsWith('https://'))
          throw new TypeError('Review option needs an optional HIP and explicit source URL');
      }
    }
  }
  const identified=legacy.mapped.filter(name=>candidateNames.has(normalize(name)));
  const routedExtended=legacy.mapped.filter(name=>extendedNames.has(normalize(name)));
  const missing=legacy.mapped.filter(name=>!candidateNames.has(normalize(name))&&!extendedNames.has(normalize(name)));
  const reviewed=missing.filter(name=>reviewNames.has(normalize(name)));
  const unclassified=missing.filter(name=>!reviewNames.has(normalize(name)));
  const candidateNotUsed=candidateManifest.stars.map(s=>s.name).filter(name=>!legacy.mapped.some(n=>normalize(n)===normalize(name)));
  return {
    schemaVersion:1,
    status:'NAMING_COVERAGE_ONLY_NOT_ASTROMETRIC_OR_SCIENTIFIC_PARITY',
    legacyDeclared:legacy.declared,
    legacyUsed:legacy.mapped.length,
    legacyUnmapped:legacy.unmapped,
    legacyOrphanMappingNames:legacy.orphanIds,
    candidatesIdentified:identified.length,
    candidateNames:identified,
    nonstellarIdentified:routedExtended.length,
    nonstellarNames:routedExtended,
    extendedNamesNotUsed:[...extendedNames].filter(name=>!legacy.mapped.some(n=>normalize(n)===name)),
    totalRoutedTargets:identified.length+routedExtended.length,
    pendingIdentityReview:reviewed.length,
    pendingIdentityReviewNames:reviewed,
    reviewNamesNotUsed:[...reviewNames].filter(name=>!legacy.mapped.some(n=>normalize(n)===name)),
    unclassifiedCount:unclassified.length,
    unclassifiedNames:unclassified,
    missingCandidates:missing.length,
    missingNames:missing,
    candidateNamesNotUsed: candidateNotUsed,
    scientificallyValidated:0,
    note:'Stellar targets carry HIP IDs, extended objects use separate science, and pending-review labels are NOT accepted as resolved HIP stars. No target has production scientific validation.',
  };
}

export function runFixedStarCoverageAudit(argv){
  const options=Object.fromEntries(argv.filter(x=>x.startsWith('--')&&x.includes('=')).map(x=>{
    const i=x.indexOf('=');return [x.slice(0,i),x.slice(i+1)];
  }));
  for(const key of Object.keys(options))
    if(!['--backend-helper','--candidates','--extended','--review','--out'].includes(key))throw new Error(`Unknown argument: ${key}`);
  for(const key of ['--backend-helper','--candidates'])
    if(typeof options[key]!=='string' || !isAbsolute(options[key]))throw new Error(`${key} requires an absolute local path`);
  const manifest=JSON.parse(readFileSync(resolve(options['--candidates']),'utf8'));
  if(options['--extended']!==undefined && !isAbsolute(options['--extended']))
    throw new Error('--extended requires an absolute local path');
  const extended=options['--extended']===undefined?undefined:
    JSON.parse(readFileSync(resolve(options['--extended']),'utf8'));
  if(options['--review']!==undefined && !isAbsolute(options['--review']))
    throw new Error('--review requires an absolute local path');
  const review=options['--review']===undefined?undefined:
    JSON.parse(readFileSync(resolve(options['--review']),'utf8'));
  const report=checkFixedStarCoverage(readFileSync(resolve(options['--backend-helper']),'utf8'),manifest,extended,review);
  if(options['--out']!==undefined){
    if(!isAbsolute(options['--out']))throw new Error('--out must be an absolute local path');
    writeFileSync(resolve(options['--out']),JSON.stringify(report,null,2)+'\n',{flag:'wx'});
  }
  return report;
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try{console.log(JSON.stringify(runFixedStarCoverageAudit(process.argv.slice(2)),null,2));}
  catch(error){console.error(error.message);process.exitCode=1;}
}
