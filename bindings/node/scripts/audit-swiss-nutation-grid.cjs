'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Swiss SE_ECL_NUT is a model comparison, not an observation of ancient Earth.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {epochGrid,summarize:summarizeErfa,checkLinearConformance}=require('./audit-erfa-nutation-grid.cjs');
function digest(file){const h=crypto.createHash('sha256'),fd=fs.openSync(file,'r'),buf=Buffer.allocUnsafe(1024*1024);try{for(;;){const n=fs.readSync(fd,buf,0,buf.length,null);if(!n)break;h.update(buf.subarray(0,n));}}finally{fs.closeSync(fd);}return h.digest('hex');}
function checkSwiss(r,jd){
  if(!r||r.error||r.rflag!==0||![r.longitude,r.latitude,r.distance,r.longitudeSpeed].every(Number.isFinite)||r.latitudeSpeed!==0||r.distanceSpeed!==0||Math.abs(r.longitude)>90||Math.abs(r.latitude)>90||Math.abs((r.longitude-r.latitude-r.longitudeSpeed)*3600)>1e-7)throw new Error(`invalid Swiss ECL_NUT at ${jd}`);
  return {psi:r.distance*3600,eps:r.longitudeSpeed*3600,trueObliquityDeg:r.longitude,meanObliquityDeg:r.latitude,flags:r.rflag};
}
function summarize(rows){
  if(!Array.isArray(rows)||!rows.length||!rows.every(r=>Number.isFinite(r.jdTt)&&Number.isInteger(r.year)&&[r.localPsi,r.localEps,r.aPsi,r.aEps,r.bPsi,r.bEps,r.swissPsi,r.swissEps].every(Number.isFinite)))throw new Error('invalid rows');
  const measures={localPsi:r=>r.localPsi-r.swissPsi,localEps:r=>r.localEps-r.swissEps,aPsi:r=>r.aPsi-r.swissPsi,aEps:r=>r.aEps-r.swissEps,bPsi:r=>r.bPsi-r.swissPsi,bEps:r=>r.bEps-r.swissEps,
    localPsiBiasResidual:r=>r.localPsi-r.swissPsi-(-0.000135),localEpsBiasResidual:r=>r.localEps-r.swissEps-0.000388};
  const group=list=>{if(!list.length)throw new Error('empty subgroup');return {cases:list.length,measures:Object.fromEntries(Object.entries(measures).map(([key,fn])=>{
    const values=list.map(fn),row=list.reduce((a,b)=>Math.abs(fn(b))>Math.abs(fn(a))?b:a);
    return [key,{minArcsec:Math.min(...values),maxArcsec:Math.max(...values),maxAbsoluteArcsec:Math.abs(fn(row)),year:row.year,jdTt:row.jdTt}];}))};};
  return {all:group(rows),preHorizons:group(rows.filter(r=>r.year< -9998)),modern:group(rows.filter(r=>r.year>=1900&&r.year<=2100))};
}
function main(){
  const [swissDir,source,sourceHash,out,...extra]=process.argv.slice(2);
  if(!swissDir||!source||!/^[a-f0-9]{64}$/.test(sourceHash||'')||!out||extra.length||fs.existsSync(out))throw new Error('usage: SWISSEPH_PACKAGE ERFA_REPORT ERFA_REPORT_SHA NEW_REPORT');
  const pkg=path.resolve(swissDir),inputs=[path.resolve(source),path.join(pkg,'package.json'),path.join(pkg,'lib/swisseph.js'),path.join(pkg,'build/Release/swisseph.node')];
  if(new Set([...inputs,path.resolve(out)]).size!==inputs.length+1)throw new Error('inputs and output must differ');
  const hashes=inputs.map(digest);if(hashes[0]!==sourceHash)throw new Error('source hash mismatch');
  const reference=JSON.parse(fs.readFileSync(source,'utf8')),grid=epochGrid();
  if(reference.schema!=='erfa-nutation-grid/1'||reference.erfaVersion!=='2.0.1.5'||!Array.isArray(reference.rows)||reference.rows.length!==grid.length)throw new Error('invalid ERFA report');
  checkLinearConformance(summarizeErfa(reference.rows));
  const swiss=require(pkg),version=swiss.swe_version();
  if(swiss.SE_ECL_NUT!==-1)throw new Error('Swiss ECL_NUT unavailable');
  const rows=reference.rows.map((r,i)=>{
    if(r.jdTt!==grid[i].jdTt||r.year!==grid[i].year)throw new Error(`epoch mismatch ${i}`);
    const s=checkSwiss(swiss.swe_calc(r.jdTt,swiss.SE_ECL_NUT,0),r.jdTt);
    return {...r,swissPsi:s.psi,swissEps:s.eps,swissTrueObliquityDeg:s.trueObliquityDeg,swissMeanObliquityDeg:s.meanObliquityDeg,swissFlags:s.flags};
  });
  if(inputs.some((p,i)=>digest(p)!==hashes[i]))throw new Error('input changed');
  const report={schema:'swiss-nutation-grid/1',status:'EXPLORATORY',sourceSha256:hashes[0],swissPackageSha256:hashes[1],swissWrapperSha256:hashes[2],swissNativeSha256:hashes[3],swissVersion:version,
    notes:'Swiss ECL_NUT at explicit TT; no ephemeris source implied. Fixed offsets are those in the local 2000B evaluator; numerical agreement does not reveal Swiss internals or establish physical truth.',summary:summarize(rows),rows};
  fs.writeFileSync(out,JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report.summary));
}
module.exports={checkSwiss,summarize};
if(require.main===module){try{main();}catch(e){console.error(e.message);process.exitCode=1;}}
