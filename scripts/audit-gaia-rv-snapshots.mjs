// Original VirgoStars source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// Offline operator audit: source bytes and results are not authenticated or certified.
import {createHash} from 'node:crypto';
import {closeSync,existsSync,fstatSync,openSync,readSync,realpathSync,unlinkSync,writeFileSync} from 'node:fs';
import {basename,dirname,isAbsolute,join,parse,relative,resolve,sep} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';

const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const hashValid=s=>typeof s==='string' && /^[0-9a-f]{64}$/.test(s);
const MAX_BYTES=5*1024*1024;
function readBounded(path){
  const fd=openSync(path,'r');
  try{
    const stat=fstatSync(fd);
    if(!stat.isFile() || stat.size<1 || stat.size>MAX_BYTES)throw new Error('Input must be a nonempty regular file at most 5 MiB');
    const data=Buffer.allocUnsafe(stat.size+1);let n=0;
    while(n<data.length){const read=readSync(fd,data,n,data.length-n,null);if(read===0)break;n+=read;}
    if(n!==stat.size)throw new Error('Input size changed while reading');
    return data.subarray(0,n);
  }finally{closeSync(fd);}
}
const decode=bytes=>new TextDecoder('utf-8',{fatal:true}).decode(bytes);

/** SIMBAD TAP snapshot: preserve Gaia source IDs as decimal strings. */
export function parseSimbadHipGaiaTsv(text){
  if(typeof text!=='string')throw new TypeError('SIMBAD TSV text required');
  const lines=text.replace(/\r\n/g,'\n').split('\n');
  if(lines.pop()!=='' || lines.shift()!=='hip_id\tgaia_id' || !lines.length || lines.length>500)throw new Error('Invalid SIMBAD crossmatch TSV');
  const hips=new Set(),sources=new Set();
  return lines.map(line=>{
    const match=/^"HIP\s+([1-9]\d{0,6})"\t"Gaia DR3 ([1-9]\d{0,18})"$/.exec(line);
    if(!match || hips.has(match[1]) || sources.has(match[2]))throw new Error('Invalid, duplicate or ambiguous SIMBAD crossmatch');
    hips.add(match[1]);sources.add(match[2]);return {hip:match[1],gaiaSourceId:match[2]};
  });
}

/** Parse only one exact Gaia I/355 source row; a blank RV remains missing. */
export function parseGaiaRvTsv(text,expectedSourceId){
  if(typeof text!=='string' || !/^[1-9]\d{0,18}$/.test(expectedSourceId))throw new TypeError('Gaia TSV and exact decimal source ID required');
  const lines=text.replace(/\r\n/g,'\n').split('\n');
  if(!lines.includes('#Name: I/355/gaiadr3') || !lines.includes('Source\tRV\te_RV\tRUWE'))throw new Error('Unexpected Gaia response schema');
  const candidates=lines.filter(line=>line.split('\t')[0].trim()===expectedSourceId);
  if(candidates.length!==1)throw new Error('Gaia response must contain exactly one matching source');
  const parts=candidates[0].split('\t').map(x=>x.trim());
  if(parts.length!==4)throw new Error('Invalid Gaia row');
  const number=(v,label)=>{if(!v)return null;if(!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$/.test(v))throw new Error(`Invalid ${label}`);const n=Number(v);if(!Number.isFinite(n))throw new Error(`Invalid ${label}`);return n;};
  const rvKmPerSecond=number(parts[1],'RV'),rvErrorKmPerSecond=number(parts[2],'RV uncertainty'),ruwe=number(parts[3],'RUWE');
  if((rvErrorKmPerSecond!==null && rvErrorKmPerSecond<0) || (ruwe!==null && ruwe<=0))throw new Error('Invalid Gaia uncertainty or RUWE');
  return {rvKmPerSecond,rvErrorKmPerSecond,ruwe,status:rvKmPerSecond===null?'RV_MISSING':'RV_PRESENT_REVIEW_REQUIRED'};
}

function outsideGit(path){
  const out=resolve(path),root=realpathSync(resolve(dirname(fileURLToPath(import.meta.url)),'..'));
  const parent=realpathSync(dirname(out));
  for(const dest of [out,resolve(parent,basename(out))]){
    const rel=relative(root,dest);
    if(rel==='' || (rel!=='..' && !rel.startsWith('..'+sep) && !isAbsolute(rel)))throw new Error('Output must remain outside the repository');
  }
  for(let dir=parent;;dir=dirname(dir)){
    if(existsSync(join(dir,'.git')))throw new Error('Output must remain outside any Git worktree');
    if(dir===parse(dir).root)break;
  }
  return out;
}

export function auditGaiaSnapshots({crossmatchBytes,crossmatchSha256,readSnapshot}){
  if(!Buffer.isBuffer(crossmatchBytes) || !hashValid(crossmatchSha256) || sha(crossmatchBytes)!==crossmatchSha256 || typeof readSnapshot!=='function')throw new Error('Pinned crossmatch bytes and snapshot reader required');
  const identities=parseSimbadHipGaiaTsv(decode(crossmatchBytes));
  const stars=identities.map(identity=>{
    const bytes=readSnapshot(identity.hip);
    if(bytes===null)return {...identity,status:'SNAPSHOT_NOT_SUPPLIED',snapshotSha256:null};
    if(!Buffer.isBuffer(bytes) || bytes.length<1 || bytes.length>MAX_BYTES)throw new Error('Invalid Gaia snapshot bytes');
    return {...identity,...parseGaiaRvTsv(decode(bytes),identity.gaiaSourceId),snapshotSha256:sha(bytes)};
  });
  const counts={};for(const star of stars)counts[star.status]=(counts[star.status]??0)+1;
  return {schemaVersion:1,status:'OPERATOR_SNAPSHOTS_NOT_AUTHENTICATED_OR_CERTIFIED',crossmatchSha256,counts,stars,scientificallyValidated:false,limitations:['Snapshot hashes identify bytes but do not prove provenance, accuracy or rights','A Gaia identifier association is not independently validated','RUWE and RV availability do not certify single-star motion or epoch stability','Missing RV is never replaced with zero; all measured RVs require review']};
}

export function runGaiaSnapshotAudit(argv){
  const allowed=['--crossmatch','--crossmatch-sha256','--gaia-dir','--out'],opts={};
  for(const arg of argv){const i=arg.indexOf('=');const key=arg.slice(0,i);if(i<3 || !allowed.includes(key) || Object.hasOwn(opts,key) || !arg.slice(i+1))throw new Error('Expected unique supported --key=value arguments');opts[key]=arg.slice(i+1);}
  if(allowed.some(k=>!opts[k]) || ![opts['--crossmatch'],opts['--gaia-dir'],opts['--out']].every(isAbsolute))throw new Error('Absolute inputs/output and pinned crossmatch are required');
  const out=outsideGit(opts['--out']);
  const input=readBounded(opts['--crossmatch']);
  const result=auditGaiaSnapshots({crossmatchBytes:input,crossmatchSha256:opts['--crossmatch-sha256'],readSnapshot:hip=>{
    try{return readBounded(join(opts['--gaia-dir'],`${hip}.tsv`));}catch(e){if(e?.code==='ENOENT')return null;throw e;}
  }});
  let fd;
  try{fd=openSync(out,'wx',0o600);writeFileSync(fd,JSON.stringify(result,null,2)+'\n');closeSync(fd);fd=undefined;}
  catch(e){if(fd!==undefined){try{closeSync(fd);}catch{}try{unlinkSync(out);}catch{}}throw e;}
  return {counts:result.counts,stars:result.stars.length,scientificallyValidated:false};
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try{console.log(JSON.stringify(runGaiaSnapshotAudit(process.argv.slice(2))));}
  catch(e){console.error(e.message);process.exitCode=1;}
}
