'use strict';
// SPDX-License-Identifier: CPAL-1.0. Sample sensitivity, not an accuracy bound.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine}=require('../index.js');
const {extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs');
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
function separation(a,b){
  if(a.length!==3||b.length!==3||![...a,...b].every(Number.isFinite))throw new Error('invalid vectors');
  const na=Math.hypot(...a),nb=Math.hypot(...b);if(!(na>0&&nb>0))throw new Error('zero vector');
  return Math.acos(Math.max(-1,Math.min(1,a.reduce((v,x,i)=>v+x*b[i],0)/(na*nb))))*180/Math.PI;
}
async function main(){
  const [kernel,kernelHash,swissPackage,source,sourceHash,out,...extra]=process.argv.slice(2);
  if(!kernel||!source||!/^[a-f0-9]{64}$/.test(kernelHash||'')||!/^[a-f0-9]{64}$/.test(sourceHash||'')||!swissPackage||!out||extra.length
      ||new Set([kernel,source,out].map(p=>path.resolve(p))).size!==3||fs.existsSync(out))throw new Error('usage: <de441> <sha256> <swisseph> <comparison> <comparison-sha> <new-report>');
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input hash mismatch');
  const report=JSON.parse(fs.readFileSync(source)),worst=report.summary?.all?.directOwnLongitudeRate;
  if(report.schema!=='virgostars-ancient-obliquity-comparison/1'||report.kernelSha256!==kernelHash||!Array.isArray(report.rows)||!worst)throw new Error('invalid comparison');
  const matches=report.rows.filter(r=>r.year===worst.year&&r.month===worst.month&&r.body===worst.body);
  if(matches.length!==1||!Number.isFinite(matches[0].jdTdb)||!Number.isFinite(matches[0].direct?.ownRateLon)||!Number.isFinite(worst.absolute)||Math.abs(Math.abs(matches[0].direct.ownRateLon)-worst.absolute)>1e-10)throw new Error('invalid selected outlier');
  const {year,month,body,jdTdb:jd}=matches[0],swiss=require(path.resolve(swissPackage)),engine=new Engine(path.resolve(kernel));
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  const center=engine.experimentalLongTermDirectObliquityTdb(jd,body).xx;
  const elongation=separation(engine.experimentalApparentIcrfTdb(jd,body).positionKm,engine.experimentalApparentIcrfTdb(jd,0).positionKm);
  const steps=[];
  for(const h of [0.001,0.002,0.005,0.01,0.02,0.04]){
    const ts=[jd-2*h,jd-h,jd+h,jd+2*h],ours=ts.map(t=>engine.experimentalLongTermDirectObliquityTdb(t,body).xx);
    const ref=ts.map(t=>swiss.swe_calc(tt(t),body,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED));
    if(!ours.every(x=>x.length===6&&x.every(Number.isFinite))||!ref.every(x=>x&&!x.error&&(x.rflag&swiss.SEFLG_JPLEPH)&&Number.isFinite(x.longitude)))throw new Error('invalid reference state');
    const rate=(v,i,circular=false)=>extrapolatedRate(v[0][i],v[1][i],v[2][i],v[3][i],jd,h,circular);
    const swissRate=extrapolatedRate(ref[0].longitude,ref[1].longitude,ref[2].longitude,ref[3].longitude,jd,h,true);
    steps.push({halfStepDays:h,ownLongitudeArcsecPerDay:(center[3]-rate(ours,0,true))*3600,
      ownLatitudeArcsecPerDay:(center[4]-rate(ours,1))*3600,swissLongitudeArcsecPerDay:(center[3]-swissRate)*3600});
  }
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input changed');
  const output={schema:'virgostars-ancient-rate-outlier/1',status:'EXPLORATORY',kernelSha256:kernelHash,sourceSha256:sourceHash,
    notes:'Solar elongation is apparent ICRF center-to-center. Finite-difference sensitivity is diagnostic, not an error bound or a diagnosis of cause.',year,month,body,jdTdb:jd,solarElongationDeg:elongation,steps};
  fs.writeFileSync(out,JSON.stringify(output,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(output));
}
module.exports={separation};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
