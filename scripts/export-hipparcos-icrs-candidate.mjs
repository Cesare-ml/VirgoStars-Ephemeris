// Original VirgoStars source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// Operator-only experimental geometric ICRS motion candidate; no network.
import {createHash} from 'node:crypto';
import {closeSync,fstatSync,openSync,readSync,realpathSync,writeFileSync} from 'node:fs';
import {dirname,basename,isAbsolute,relative,resolve,sep} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {loadExperimentalStarCatalogFile} from '../src/experimental-star-catalog-file.mjs';

const HIP_EPOCH=2448349.0625;
const EPOCHS=Object.freeze([HIP_EPOCH,2451545,2469807.5]);
const digest=value=>createHash('sha256').update(value).digest('hex');
const validHash=value=>typeof value==='string' && /^[0-9a-f]{64}$/.test(value);

function readAudit(path,expected){
  if(typeof path!=='string' || !isAbsolute(path) || !validHash(expected))throw new TypeError('Absolute audit and lowercase SHA-256 are required');
  const fd=openSync(path,'r');let raw;
  try{
    const stat=fstatSync(fd);
    if(!stat.isFile() || stat.size<1 || stat.size>64*1024*1024)throw new RangeError('Audit must be a nonempty regular file at most 64 MiB');
    const buffer=Buffer.allocUnsafe(stat.size+1);let count=0;
    while(count<buffer.length){const n=readSync(fd,buffer,count,buffer.length-count,null);if(n===0)break;count+=n;}
    if(count!==stat.size)throw new Error('Audit size changed during read');
    raw=buffer.subarray(0,count);
  }finally{closeSync(fd);}
  if(digest(raw)!==expected)throw new Error('Audit SHA-256 mismatch');
  return JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(raw));
}

/** Produce a reproducible candidate from supplied records and a pinned audit.
 * The audit's identities are labels, not proof of source authenticity or rights.
 */
export function exportHipparcosIcrsCandidate({catalog,catalogSha256,audit,auditSha256,executablePath}){
  if(!catalog || typeof catalog!=='object' || typeof catalog.id!=='string' || !Array.isArray(catalog.records) || !catalog.records.length || catalog.records.length>1000 || !validHash(catalogSha256) || !validHash(auditSha256))throw new TypeError('Pinned nonempty catalog of at most 1000 records is required');
  if(typeof executablePath!=='string' || !isAbsolute(executablePath))throw new TypeError('Absolute Rust executable path required');
  if(!audit || !Array.isArray(audit.stars) || audit.stars.length!==catalog.records.length)throw new TypeError('Matching per-star identity audit is required');
  const byName=new Map(),hips=new Set();
  for(const item of audit.stars){
    if(!item || typeof item.name!=='string' || !item.name.trim() || item.name!==item.name.trim() || !Number.isSafeInteger(item.hip) || item.hip<1 || byName.has(item.name) || hips.has(item.hip))throw new Error('Invalid or duplicate audit identity');
    byName.set(item.name,item.hip);hips.add(item.hip);
  }
  const seen=new Set(),stars=[];
  for(const star of catalog.records){
    if(!star || !byName.has(star.name) || seen.has(star.name) || star.epochTdbJulianDay!==HIP_EPOCH)throw new Error('Catalog and audit identities or epoch do not match');
    seen.add(star.name);
    const values=[star.epochTdbJulianDay,star.rightAscensionDeg,star.declinationDeg,star.properMotionRaCosDecMasPerYear,star.properMotionDecMasPerYear,star.parallaxMas,star.radialVelocityKmPerSecond];
    if(!values.every(Number.isFinite))throw new TypeError('Finite stellar astrometry is required');
    const samples=EPOCHS.map(epoch=>{
      const child=spawnSync(executablePath,['star_icrs',...values.map(String),String(epoch)],{encoding:'utf8',windowsHide:true,timeout:30000,maxBuffer:65536});
      if(child.error)throw new Error(`ICRS process failed: ${child.error.message}`,{cause:child.error});
      if(child.status!==0)throw new Error(`ICRS process exited with status ${child.status}: ${(child.stderr||'').trim()}`);
      const lines=child.stdout.trim().split(/\r?\n/);
      if(lines.length!==2 || lines[0]!=='tdb_jd,ra_deg,declination_deg,distance_au')throw new Error('Invalid ICRS output contract');
      const parts=lines[1].split(',');if(parts.length!==4 || parts.some(x=>!x.trim() || !Number.isFinite(Number(x))))throw new Error('Invalid ICRS output row');
      const [tdbJulianDay,rightAscensionDeg,declinationDeg,distanceAu]=parts.map(Number);
      if(tdbJulianDay!==epoch || rightAscensionDeg<0 || rightAscensionDeg>=360 || Math.abs(declinationDeg)>=90 || distanceAu<=0)throw new Error('Out-of-range ICRS output row');
      return {tdbJulianDay,rightAscensionDeg,declinationDeg};
    });
    stars.push({name:star.name,hip:byName.get(star.name),samples});
  }
  return {schemaVersion:1,coordinateFrame:'ICRS_ASTROMETRIC',epochConvention:'TDB_JULIAN_DAY',candidateModel:'VIRGOSTARS_CLASSICAL_BARYCENTRIC_ICRS_NO_LIGHT_TIME',catalogId:catalog.id,inputCatalogSha256:catalogSha256,inputAuditSha256:auditSha256,epochsTdbJulianDay:[...EPOCHS],stars,scientificallyValidated:false,limitations:['Classical geometric approximation to ICRS astrometric catalog motion; stellar Roemer/light-time and relativity omitted','No geocentric parallax, aberration, deflection or apparent/ecliptic position','Input identity and license assertions are not verified']};
}

export function runIcrsCandidate(argv){
  const allowed=['--catalog','--catalog-sha256','--audit','--audit-sha256','--executable','--out'];
  const opts={};
  for(const arg of argv){const i=arg.indexOf('=');const key=arg.slice(0,i);if(i<3 || !allowed.includes(key) || Object.hasOwn(opts,key) || !arg.slice(i+1))throw new Error('Expected unique supported --key=value arguments');opts[key]=arg.slice(i+1);}
  if(allowed.some(key=>!opts[key]))throw new Error('All catalog, audit, executable and output arguments are required');
  if(!isAbsolute(opts['--out']))throw new Error('Absolute output path required');
  const output=resolve(opts['--out']),root=realpathSync(resolve(dirname(fileURLToPath(import.meta.url)),'..'));
  const targets=[output,resolve(realpathSync(dirname(output)),basename(output))];
  for(const target of targets){const rel=relative(root,target);if(rel==='' || (rel!=='..' && !rel.startsWith('..'+sep) && !isAbsolute(rel)))throw new Error('Output must remain outside the Git repository');}
  const {catalog}=loadExperimentalStarCatalogFile({filePath:opts['--catalog'],sha256:opts['--catalog-sha256'],source:'operator asserted',license:'operator asserted'});
  const audit=readAudit(opts['--audit'],opts['--audit-sha256']);
  const result=exportHipparcosIcrsCandidate({catalog,catalogSha256:opts['--catalog-sha256'],audit,auditSha256:opts['--audit-sha256'],executablePath:opts['--executable']});
  writeFileSync(output,JSON.stringify(result,null,2)+'\n',{flag:'wx',mode:0o600});
  return {stars:result.stars.length,epochs:result.epochsTdbJulianDay,scientificallyValidated:false};
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try{console.log(JSON.stringify(runIcrsCandidate(process.argv.slice(2))));}
  catch(error){console.error(error.message);process.exitCode=1;}
}
