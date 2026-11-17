'use strict';
// SPDX-License-Identifier: CPAL-1.0. Sampled retrograde-station diagnostic, not an event API.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs');
const seeds=[[2,1160,10],[3,620,1],[4,1380,1],[5,1340,1],[6,420,1],[7,1800,1],[8,960,1],[9,2820,1]];
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
function bisect(f,lo,hi,tolerance){
  if(typeof f!=='function'||![lo,hi,tolerance].every(Number.isFinite)||lo>=hi||tolerance<=0)throw new Error('invalid bracket');
  let fl=f(lo),fh=f(hi);if(!Number.isFinite(fl)||!Number.isFinite(fh)||fl*fh>0)throw new Error('not bracketed');
  if(fl===0)return {lo,hi:lo};if(fh===0)return {lo:hi,hi};
  for(let i=0;i<100&&hi-lo>tolerance;i++){
    const mid=lo+(hi-lo)/2,fm=f(mid);if(!Number.isFinite(fm)||mid===lo||mid===hi)throw new Error('invalid bisection');
    if(fm===0)return {lo:mid,hi:mid};if(Math.sign(fm)===Math.sign(fl)){lo=mid;fl=fm;}else{hi=mid;fh=fm;}
  }
  if(hi-lo>tolerance)throw new Error('bisection did not converge');return {lo,hi};
}
function separation(a,b){
  const cross=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
  return Math.atan2(Math.hypot(...cross),a.reduce((x,v,i)=>x+v*b[i],0))*180/Math.PI;
}
async function main(){
  const [kernel,expected,swissPackage,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!swissPackage||!out||extra.length||fs.existsSync(out)||await sha(kernel)!==expected)throw new Error('usage: <de441> <sha256> <swisseph> <new-report>');
  const swiss=require(path.resolve(swissPackage)),engine=new Engine(path.resolve(kernel)),tol=0.00001;
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  const reference=(jd,body)=>{const flags=swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED,r=swiss.swe_calc(tt(jd),body,flags);
    if(!r||r.error||(r.rflag&flags)!==flags||![r.longitude,r.longitudeSpeed].every(Number.isFinite))throw new Error('Swiss JPL unavailable');return r;};
  const longitude=(jd,body)=>reference(jd,body).longitude;
  const swissSpeed=(jd,body,h)=>extrapolatedRate(longitude(jd-2*h,body),longitude(jd-h,body),longitude(jd+h,body),longitude(jd+2*h,body),jd,h,true);
  const engineSpeed=(jd,body)=>{const a=engine.experimentalLongTermApparentTdb(jd,body).xx;if(a.length!==6||!a.every(Number.isFinite))throw new Error('invalid engine state');return a[3];};
  const rows=[];
  for(const [body,year,month] of seeds){
    const seed=julday(year,month,1,12,true),f=jd=>swissSpeed(jd,body,0.02),brackets=[];
    let lo=seed-120,prev=f(lo);
    for(let k=1;k<=240;k++){const hi=seed-120+k,next=f(hi);
      if(prev===0||next===0||Math.sign(prev)!==Math.sign(next))brackets.push([lo,hi]);lo=hi;prev=next;}
    if(!brackets.length)throw new Error(`station not bracketed for ${body}`);
    brackets.sort((a,b)=>Math.abs((a[0]+a[1])/2-seed)-Math.abs((b[0]+b[1])/2-seed));
    const [left,right]=brackets[0],engineBracket=bisect(jd=>engineSpeed(jd,body),left,right,tol),
      engineRoot=(engineBracket.lo+engineBracket.hi)/2;
    const steps=[0.01,0.02,0.04].map(h=>{const interval=bisect(jd=>swissSpeed(jd,body,h),left,right,tol);
      const root=(interval.lo+interval.hi)/2;return {halfStepDays:h,bracket:interval,differenceFromEngineMinutes:(root-engineRoot)*1440};});
    const published=bisect(jd=>reference(jd,body).longitudeSpeed,left,right,tol);
    const pos=engine.experimentalApparentIcrfTdb(engineRoot,body).positionKm,sun=engine.experimentalApparentIcrfTdb(engineRoot,0).positionKm;
    rows.push({body,seedYear:year,seedMonth:month,seedJdTdb:seed,searchBracket:[left,right],engineBracket,engineRootJdTdb:engineRoot,
      sunSeparationDegrees:separation(pos,sun),swissPositionDerivativeRoots:steps,swissPublishedSpeedRoot:published,
      publishedDifferenceFromEngineMinutes:((published.lo+published.hi)/2-engineRoot)*1440});
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-swiss-station-roots/1',status:'EXPLORATORY',kernelSha256:expected,swissVersion:swiss.swe_version(),
    notes:'Engine apparent TDB longitude rate vs Swiss fourth-order position derivatives at approximately corresponding TT; same DE441, frame/model differences. Bracket resolution is numerical, not accuracy.',rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(rows));
}
module.exports={bisect,separation};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
