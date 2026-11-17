'use strict';
// SPDX-License-Identifier: CPAL-1.0. Caller-supplied Swiss; explicit experimental comparison only.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs');
const AU_KM=149597870.7,C_KM_DAY=299792.458*86400;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
// Approximation: Earth acceleration is minus the acceleration of the geometric Sun relative to Earth.
// Solar barycentric acceleration is ignored. Prediction is exploratory, not a correction.
function projectedDelayAcceleration(positionKm,sunVelocityBefore,sunVelocityAfter,spanDays){
  if(![positionKm,sunVelocityBefore,sunVelocityAfter].every(v=>Array.isArray(v)&&v.length===3&&v.every(Number.isFinite))||!Number.isFinite(spanDays)||spanDays<=0)throw new Error('invalid state');
  const distance=Math.hypot(...positionKm);if(!Number.isFinite(distance)||distance<=0)throw new Error('invalid direction');
  const a=sunVelocityBefore.map((v,i)=>(v-sunVelocityAfter[i])/spanDays);
  return a.reduce((sum,v,i)=>sum+v*positionKm[i]/distance,0)*(distance/C_KM_DAY)/AU_KM;
}
function regression(rows){
  let xx=0,xy=0,yy=0,resid2=0;
  for(const r of rows){const x=r.predictionAuDay,y=r.modes.apparent.residual[2];xx+=x*x;xy+=x*y;yy+=y*y;resid2+=(x-y)**2;}
  if(!(xx>0))throw new Error('degenerate prediction');
  return {count:rows.length,slopeThroughOrigin:xy/xx,observedRmsAuDay:Math.sqrt(yy/rows.length),predictionErrorRmsAuDay:Math.sqrt(resid2/rows.length)};
}
async function main(){
  const [kernel,expected,swissPackage,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!swissPackage||!out||extra.length||fs.existsSync(out)||await sha(kernel)!==expected)throw new Error('usage: <de441> <sha256> <swisseph> <new-report>');
  const swiss=require(path.resolve(swissPackage)),engine=new Engine(path.resolve(kernel)),h=0.02;
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  const flags={apparent:0,noAberration:swiss.SEFLG_NOABERR,noDeflection:swiss.SEFLG_NOGDEFL,
    noNutation:swiss.SEFLG_NONUT,fixedJ2000:swiss.SEFLG_J2000,truePosition:swiss.SEFLG_TRUEPOS};
  if(Object.entries(flags).some(([name,value])=>name!=='apparent'&&(!Number.isInteger(value)||!value)))throw new Error('missing Swiss flags');
  function calc(jd,body,mode){const requested=swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED|flags[mode],r=swiss.swe_calc(jd,body,requested);
    const v=[r?.longitude,r?.latitude,r?.distance,r?.longitudeSpeed,r?.latitudeSpeed,r?.distanceSpeed];
    if(!r||r.error||(r.rflag&requested)!==requested||!v.every(Number.isFinite))throw new Error(`Swiss mode unavailable: ${body}, ${mode}, ${jd}`);
    return v;}
  const rows=[];
  // New dates selected before this grid: years 10,30,...,2990; four seasons.
  // JD is interpreted as TT by Swiss and approximately as TDB by the engine.
  for(let year=10;year<3000;year+=20)for(const month of [1,4,7,10]){
    const jd=julday(year,month,1,12,true),span=(jd+h)-(jd-h),
      before=engine.geometricStateTdb(jd-h,0).velocityKmPerDay,after=engine.geometricStateTdb(jd+h,0).velocityKmPerDay;
    for(let body=1;body<=9;body++){
      const modes={};
      for(const mode of Object.keys(flags)){
        const a=[-2,-1,0,1,2].map(k=>calc(jd+k*h,body,mode));
        const finite=[0,1,2].map(i=>extrapolatedRate(a[0][i],a[1][i],a[3][i],a[4][i],jd,h,i===0));
        modes[mode]={published:a[2].slice(3),finite,residual:a[2].slice(3).map((x,i)=>x-finite[i])};
      }
      const state=engine.experimentalApparentIcrfTdb(jd,body),predictionAuDay=projectedDelayAcceleration(state.positionKm,before,after,span);
      rows.push({year,month,body,jdTt:jd,predictionAuDay,modes});
    }
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const summarize=items=>Object.fromEntries(Object.keys(flags).map(mode=>[mode,[0,1,2].map(i=>{
    const scale=i<2?3600:1;let worst=null,sum=0;
    for(const row of items){const r=row.modes[mode].residual[i]*scale;sum+=r*r;if(!worst||Math.abs(r)>worst.absolute)worst={absolute:Math.abs(r),year:row.year,month:row.month,body:row.body};}
    return {...worst,rms:Math.sqrt(sum/items.length),unit:i<2?'arcsec/day':'AU/day'};
  })]));
  const summary={cases:rows.length,years:150,months:[1,4,7,10],bodies:9,halfStepTtDays:h,comparisons:summarize(rows),
    perBody:Object.fromEntries(Array.from({length:9},(_,i)=>[i+1,summarize(rows.filter(r=>r.body===i+1))])),
    prediction:regression(rows),predictionPerBody:Object.fromEntries(Array.from({length:9},(_,i)=>[i+1,regression(rows.filter(r=>r.body===i+1))]))};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-swiss-apparent-rate-flags/1',status:'EXPLORATORY',kernelSha256:expected,swissVersion:swiss.swe_version(),
    notes:'Swiss geocentric speeds vs derivatives of Swiss positions in each flag mode; geometric approximation is a hypothesis, not a physical correction or proof of internals.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
module.exports={projectedDelayAcceleration,regression};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
