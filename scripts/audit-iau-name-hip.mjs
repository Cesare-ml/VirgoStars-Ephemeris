// Original VirgoStars source, CPAL-1.0; see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// Offline audit of NAME->HIP agreement with a caller-provided IAU-CSN CSV snapshot.
// Not a photometric or astrometric data source; do not redistribute upstream data.
import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

export const KNOWN_IAU_SOURCE_BLOB_SHA='c3d9289db7dbf585bf5fcb51980c008cb4f3d38d';
export const KNOWN_IAU_SOURCE_URL='https://github.com/cyschneck/iau-star-names/blob/main/data/1_iau_stars.csv';
const gitBlobSha=bytes=>createHash('sha1').update('blob '+bytes.length+'\0').update(bytes).digest('hex');

// The upstream name, NEC+, designation, HIP and Bayer columns precede arbitrary
// quoted descriptions. We parse only those four identifying fields, never use eval
// or read 6D science measurements from this secondary naming reference.
export function parseIauNameHipCsv(bytes){
  if(!Buffer.isBuffer(bytes) || bytes.length<60 || bytes.length>8*1024*1024)
    throw new Error('Expected locally supplied IAU name CSV bytes (max 8 MiB)');
  const lines=bytes.toString('utf8').replace(/^\uFEFF/,'').split(/\r?\n/);
  if(!lines.shift().startsWith('Proper Names,NEC+,Designation,HIP,'))
    throw new Error('Unexpected IAU name/identifier CSV schema');
  const map=new Map();
  for(const line of lines){
    const m=line.match(/^([^,\r\n]+),(?:"(?:[^"]|"")*"|[^,]*),[^,]*,([^,]*),/);
    if(!m)continue; // Descriptive text may include multiline quoted content.
    const name=m[1].trim(),hipText=m[2].trim();
    if(!name || !hipText)continue; // Some named exoplanet hosts have no HIP.
    if(!/^\d+$/.test(hipText))throw new Error(`Invalid HIP value for IAU name ${name}`);
    const hip=Number(hipText);
    if(!Number.isSafeInteger(hip)||hip<=0)throw new Error('Invalid IAU HIP ID');
    const key=name.toLocaleLowerCase('en-US');
    if(map.has(key) && map.get(key)!==hip)throw new Error('Conflicting IAU name HIP associations');
    map.set(key,hip);
  }
  return map;
}

export function compareIauNamedStarIdentities(stars,bytes,{expectedGitBlobSha}={}){
  if(!Array.isArray(stars) || !stars.length)throw new TypeError('Stellar manifest records are required');
  if(expectedGitBlobSha!==undefined){
    if(!/^[0-9a-f]{40}$/.test(expectedGitBlobSha) || gitBlobSha(bytes)!==expectedGitBlobSha)
      throw new Error('IAU source Git blob SHA does not match pinned snapshot');
  }
  const source=parseIauNameHipCsv(bytes);
  const matching=[],nonMatching=[],missing=[];
  for(const star of stars){
    if(!star || typeof star.name!=='string' || !Number.isSafeInteger(star.hip) || star.hip<=0)
      throw new TypeError('Invalid candidate star name or HIP');
    const name=star.legacyNameReviewStatus==='SPELLING_OR_HISTORICAL_ALIAS_NEEDS_CONFIRMATION'
      ? star.iauApprovedName:star.name;
    if(typeof name!=='string' || !name)throw new Error('Historical alias missing its IAU canonical name');
    const sourceHip=source.get(name.toLocaleLowerCase('en-US'));
    if(sourceHip===undefined)missing.push({backendName:star.name,iauName:name,hip:star.hip});
    else if(sourceHip!==star.hip)nonMatching.push({backendName:star.name,iauName:name,hip:star.hip,iauHip:sourceHip});
    else matching.push({backendName:star.name,iauName:name,hip:star.hip});
  }
  return {
    status:missing.length||nonMatching.length?'NEEDS_REVIEW':'IAU_NAME_IDENTITY_AGREEMENT_ONLY',
    snapshotGitBlobSha:gitBlobSha(bytes),
    count:stars.length,matching:matching.length,legacyAliases:matching.filter(x=>x.backendName!==x.iauName).length,
    missing,nonMatching,scientificallyValidated:0,
    note:'This cross-check verifies name-to-HIP identification only; it does not validate original Hipparcos astrometry, stellar component motion or source redistribution.',
  };
}

export function runIauNameCrosscheck(argv){
  const args=Object.fromEntries(argv.filter(x=>x.startsWith('--')&&x.includes('=')).map(v=>{
    const i=v.indexOf('=');return [v.slice(0,i),v.slice(i+1)];
  }));
  for(const key of ['--iau-csv','--stars'])if(!args[key])throw new Error(`Missing ${key}=path`);
  if(Object.keys(args).some(k=>!['--iau-csv','--stars','--sha'].includes(k)))throw new Error('Unrecognized argument');
  const rows=JSON.parse(readFileSync(resolve(args['--stars']),'utf8')).stars;
  return compareIauNamedStarIdentities(rows,readFileSync(resolve(args['--iau-csv'])),{
    expectedGitBlobSha:args['--sha']??KNOWN_IAU_SOURCE_BLOB_SHA,
  });
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try{const report=runIauNameCrosscheck(process.argv.slice(2));console.log(JSON.stringify(report,null,2));if(report.status!=='IAU_NAME_IDENTITY_AGREEMENT_ONLY')process.exitCode=2;}
  catch(error){console.error(error.message);process.exitCode=1;}
}
