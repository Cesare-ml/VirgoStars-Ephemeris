// Original VirgoStars source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// Offline single-epoch exploratory comparison; no scientific certification.
import {closeSync,openSync,unlinkSync,writeFileSync} from 'node:fs';
import {isAbsolute,resolve} from 'node:path';
import {spawnSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';
import {sha,readBounded,outsideGit} from './audit-gaia-rv-snapshots.mjs';
import {angularSeparationArcsec} from './compare-star-epoch-icrs.mjs';
import {loadExperimentalStarCatalogFile} from '../src/experimental-star-catalog-file.mjs';

const GAIA_EPOCH_TCB_JD=2457389.0;
const header='Source\tRA_ICRS\tDE_ICRS\te_RA_ICRS\te_DE_ICRS\tPlx\te_Plx\tpmRA\tpmDE\tRV\te_RV\tRUWE';
function readPinned(path,digest){
  if(!isAbsolute(path) || !/^[0-9a-f]{64}$/.test(digest))throw new TypeError('Absolute input and lowercase SHA-256 required');
  const bytes=readBounded(path);if(sha(bytes)!==digest)throw new Error('Pinned input SHA-256 mismatch');return bytes;
}
function json(bytes){return JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes));}

/** Read one J2016.0 ICRS source row. It does not authenticate data. */
export function parseGaiaPositionTsv(text,expectedId){
  if(typeof text!=='string' || typeof expectedId!=='string' || !/^[1-9]\d{0,18}$/.test(expectedId))throw new TypeError('Gaia snapshot and exact source ID required');
  const lines=text.replace(/\r\n/g,'\n').split('\n');
  if(!lines.includes('#Name: I/355/gaiadr3') || !lines.includes('#Coosys\tH_2016.000:\tICRS') || !lines.includes(header))throw new Error('Unexpected Gaia ICRS J2016 snapshot schema');
  const matching=lines.filter(x=>x.split('\t')[0].trim()===expectedId);
  if(matching.length!==1)throw new Error('Gaia source must match exactly one row');
  const fields=matching[0].split('\t').map(x=>x.trim());
  if(fields.length!==12)throw new Error('Invalid Gaia source row');
  const required=(index,label)=>{const s=fields[index];if(!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$/.test(s))throw new Error(`Missing or invalid ${label}`);const n=Number(s);if(!Number.isFinite(n))throw new Error(`Invalid ${label}`);return n;};
  const rightAscensionDeg=required(1,'RA'),declinationDeg=required(2,'declination'),raErrorMas=required(3,'RA uncertainty'),decErrorMas=required(4,'declination uncertainty');
  const ruwe=fields[11]?required(11,'RUWE'):null;
  if(rightAscensionDeg<0 || rightAscensionDeg>=360 || Math.abs(declinationDeg)>=90 || raErrorMas<0 || decErrorMas<0 || (ruwe!==null && ruwe<=0))throw new RangeError('Invalid Gaia coordinates or quality fields');
  return {rightAscensionDeg,declinationDeg,raErrorMas,decErrorMas,ruwe};
}

/** Input sources and the epoch approximation remain caller assertions. */
export function compareGaiaPosition({star,hip,sourceId,gaia,executablePath,epochPolicy}){
  if(epochPolicy!=='TCB_JD_AS_TDB_JD_APPROXIMATION')throw new Error('Explicit TCB-to-TDB approximation policy is required');
  if(!star || typeof star.name!=='string' || !Number.isSafeInteger(hip) || hip<1 || typeof sourceId!=='string' || !/^[1-9]\d{0,18}$/.test(sourceId) || !gaia || typeof executablePath!=='string' || !isAbsolute(executablePath))throw new TypeError('Catalog star, HIP, exact Gaia source, position and executable required');
  const values=[star.epochTdbJulianDay,star.rightAscensionDeg,star.declinationDeg,star.properMotionRaCosDecMasPerYear,star.properMotionDecMasPerYear,star.parallaxMas,star.radialVelocityKmPerSecond,GAIA_EPOCH_TCB_JD];
  if(!values.every(Number.isFinite))throw new TypeError('Finite catalog astrometry required');
  const child=spawnSync(executablePath,['star_icrs',...values.map(String)],{encoding:'utf8',windowsHide:true,timeout:30000,maxBuffer:65536});
  if(child.error)throw new Error(`ICRS process failed: ${child.error.message}`,{cause:child.error});
  if(child.status!==0)throw new Error(`ICRS process exited with status ${child.status}: ${(child.stderr||'').trim()}`);
  const lines=child.stdout.trim().split(/\r?\n/);
  if(lines.length!==2 || lines[0]!=='tdb_jd,ra_deg,declination_deg,distance_au')throw new Error('Invalid ICRS candidate output');
  const parts=lines[1].split(',');if(parts.length!==4 || parts.some(x=>!x.trim() || !Number.isFinite(Number(x))))throw new Error('Invalid ICRS candidate row');
  const [epoch,ra,dec,distance]=parts.map(Number);
  if(epoch!==GAIA_EPOCH_TCB_JD || distance<=0)throw new Error('Mismatched ICRS epoch or distance');
  const separationArcsec=angularSeparationArcsec(ra,dec,gaia.rightAscensionDeg,gaia.declinationDeg);
  return {schemaVersion:1,status:'OBSERVED_POSITION_DIAGNOSTIC_NOT_CERTIFIED',hip,name:star.name,gaiaSourceId:sourceId,coordinateFrame:'ICRS',gaiaEpoch:'J2016.0 TCB',candidateEpochTdbJulianDay:epoch,epochPolicy,separationArcsec,gaiaFormalErrorsMas:{ra:gaia.raErrorMas,dec:gaia.decErrorMas},gaiaRuwe:gaia.ruwe,scientificallyValidated:false,limitations:['Source bytes and identity associations are caller assertions, not independently authenticated','Formal errors are not an uncertainty budget; catalog correlations and systematics are not assessed','Gaia J2016 TCB is approximated as the same numeric TDB JD','Hipparcos input epoch is interpreted as TDB; stellar light-time and relativistic motion are omitted','Single source and epoch do not certify stellar astrometry, apparent ecliptic positions or Swiss parity']};
}

export function runGaiaPositionComparison(argv){
  const required=['--catalog','--catalog-sha256','--catalog-audit','--catalog-audit-sha256','--identity-audit','--identity-audit-sha256','--gaia-observed','--gaia-observed-sha256','--hip','--executable','--epoch-policy','--out'],opts={};
  for(const arg of argv){const i=arg.indexOf('=');const key=arg.slice(0,i);if(i<3 || !required.includes(key) || Object.hasOwn(opts,key) || !arg.slice(i+1))throw new Error('Expected unique supported --key=value arguments');opts[key]=arg.slice(i+1);}
  if(required.some(k=>!opts[k]) || !['--catalog','--catalog-audit','--identity-audit','--gaia-observed','--executable','--out'].every(k=>isAbsolute(opts[k])) || !/^[1-9]\d{0,6}$/.test(opts['--hip']))throw new Error('Absolute inputs/output, exact HIP and all pinned arguments required');
  const output=outsideGit(opts['--out']);
  const {catalog}=loadExperimentalStarCatalogFile({filePath:opts['--catalog'],sha256:opts['--catalog-sha256'],source:'operator asserted',license:'operator asserted'});
  const catalogAudit=json(readPinned(opts['--catalog-audit'],opts['--catalog-audit-sha256']));
  const identities=json(readPinned(opts['--identity-audit'],opts['--identity-audit-sha256']));
  const hip=Number(opts['--hip']);
  if(!catalogAudit || !Array.isArray(catalogAudit.stars) || !identities || identities.status!=='OPERATOR_SNAPSHOTS_NOT_AUTHENTICATED_OR_CERTIFIED' || identities.scientificallyValidated!==false || !Array.isArray(identities.stars))throw new Error('Matching uncertified catalog and identity audits required');
  const sourceHash=catalogAudit.hipparcos?.sha256,suppHash=catalogAudit.supplements?.sha256;
  if(!/^[0-9a-f]{64}$/.test(sourceHash??'') || !/^[0-9a-f]{64}$/.test(suppHash??'') || typeof catalogAudit.complexSolutionsAllowed!=='boolean' || catalog.id!==`hipparcos-2007-curated-${sourceHash.slice(0,12)}-${suppHash.slice(0,12)}-${catalogAudit.complexSolutionsAllowed?'complex':'standard'}`)throw new Error('Catalog and audit provenance identifiers differ');
  const ca=catalogAudit.stars.filter(x=>x.hip===hip),id=identities.stars.filter(x=>x.hip===opts['--hip']);
  if(ca.length!==1 || id.length!==1)throw new Error('HIP must match one catalog and identity audit record');
  const records=catalog.records.filter(x=>x.name===ca[0].name);if(records.length!==1)throw new Error('Catalog identity does not match audit');
  const observed=readPinned(opts['--gaia-observed'],opts['--gaia-observed-sha256']);
  const gaia=parseGaiaPositionTsv(new TextDecoder('utf-8',{fatal:true}).decode(observed),id[0].gaiaSourceId);
  const result=compareGaiaPosition({star:records[0],hip,sourceId:id[0].gaiaSourceId,gaia,executablePath:opts['--executable'],epochPolicy:opts['--epoch-policy']});
  result.inputHashes={catalog:opts['--catalog-sha256'],catalogAudit:opts['--catalog-audit-sha256'],identityAudit:opts['--identity-audit-sha256'],gaiaObserved:opts['--gaia-observed-sha256']};
  let fd;try{fd=openSync(output,'wx',0o600);writeFileSync(fd,JSON.stringify(result,null,2)+'\n');closeSync(fd);fd=undefined;}
  catch(e){if(fd!==undefined){try{closeSync(fd);}catch{}try{unlinkSync(output);}catch{}}throw e;}
  return {status:result.status,hip,gaiaSourceId:result.gaiaSourceId,separationArcsec:result.separationArcsec,scientificallyValidated:false};
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try{console.log(JSON.stringify(runGaiaPositionComparison(process.argv.slice(2))));}
  catch(e){console.error(e.message);process.exitCode=1;}
}
