'use strict';
// SPDX-License-Identifier: CPAL-1.0. Optional cross-model rate diagnostic; no external data bundled.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {validationYears}=require('./planet-validation-years.cjs');
const wrap=x=>((x%360)+540)%360-180;
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
// A fourth-order centered derivative with actual floating-point epoch spans.
function extrapolatedRate(m2,m1,p1,p2,jd,step,circular=false){
  if(![m2,m1,p1,p2,jd,step].every(Number.isFinite)||step<=0)throw new Error('invalid derivative input');
  const h1=((jd+step)-(jd-step))/2,h2=((jd+2*step)-(jd-2*step))/2;
  if(!(h1>0&&h2>h1))throw new Error('unrepresentable derivative interval');
  const diff=(a,b)=>circular?wrap(a-b):a-b;
  const d1=diff(p1,m1)/(2*h1),d2=diff(p2,m2)/(2*h2);
  return (h2*h2*d1-h1*h1*d2)/(h2*h2-h1*h1);
}
async function main(){
  const [kernel,expected,swissPackage,out,step,...extra]=process.argv.slice(2),h=step===undefined?0.02:Number(step);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!swissPackage||!out||extra.length||!Number.isFinite(h)||h<=0||h>0.05||fs.existsSync(out)||await sha(kernel)!==expected)throw new Error('usage: <de441> <sha256> <swisseph> <new-report> [half-step-days]');
  const swiss=require(path.resolve(swissPackage)),engine=new Engine(path.resolve(kernel));
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  const reference=(jd,id)=>{
    const r=swiss.swe_calc(tt(jd),id,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);
    const values=[r?.longitude,r?.latitude,r?.distance,r?.longitudeSpeed,r?.latitudeSpeed,r?.distanceSpeed];
    if(!r||r.error||!(r.rflag&swiss.SEFLG_JPLEPH)||!values.every(Number.isFinite))throw new Error(`Swiss JPL unavailable: ${id}, ${jd}`);
    return values;
  };
  const ours=(jd,id)=>{const r=engine.experimentalLongTermApparentTdb(jd,id).xx;
    if(!Array.isArray(r)||r.length!==6||!r.every(Number.isFinite))throw new Error(`invalid engine result: ${id}, ${jd}`);return r;};
  const deriv=(samples,jd,i)=>extrapolatedRate(samples[0][i],samples[1][i],samples[3][i],samples[4][i],jd,h,i===0);
  const years=validationYears('0-3000-step-10').filter(y=>y%20===0),rows=[];
  for(const year of years)for(const month of [1,4,10])for(let body=0;body<10;body++){
    const jd=julday(year,month,1,12,true),epochs=[jd-2*h,jd-h,jd,jd+h,jd+2*h];
    const a=epochs.map(x=>ours(x,body)),b=epochs.map(x=>reference(x,body));
    const ownFd=[0,1,2].map(i=>deriv(a,jd,i)),swissFd=[0,1,2].map(i=>deriv(b,jd,i));
    rows.push({year,month,body,jdTdb:jd,engineRate:a[2].slice(3),swissRate:b[2].slice(3),engineFiniteRate:ownFd,swissFiniteRate:swissFd});
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  // Angular coordinates: degrees/day; distance: AU/day. Component maxima, not a release budget.
  const comparisons={engineVsSwissFinite:r=>r.engineRate.map((v,i)=>v-r.swissFiniteRate[i]),
    engineVsSwissPublished:r=>r.engineRate.map((v,i)=>v-r.swissRate[i]),
    engineVsOwnFinite:r=>r.engineRate.map((v,i)=>v-r.engineFiniteRate[i]),
    swissPublishedVsOwnFinite:r=>r.swissRate.map((v,i)=>v-r.swissFiniteRate[i])};
  const measure=items=>{
    const result={};
    for(const [name,fn] of Object.entries(comparisons)){
      result[name]=[0,1,2].map(component=>{
      const scale=component<2?3600:1;let worst=null,sum=0;
      for(const row of items){const residual=fn(row)[component]*scale;sum+=residual*residual;
        if(!worst||Math.abs(residual)>worst.absolute)worst={absolute:Math.abs(residual),year:row.year,month:row.month,body:row.body};}
      return {...worst,rms:Math.sqrt(sum/items.length),unit:component<2?'arcsec/day':'AU/day'};
      });
    }return result;
  };
  const summary={cases:rows.length,years:years.length,months:[1,4,10],halfStepTdbDays:h,comparisons:measure(rows),
    byBody:Object.fromEntries(Array.from({length:10},(_,body)=>[body,measure(rows.filter(r=>r.body===body))])),
    longitudeSignMismatches:rows.filter(r=>Math.sign(r.engineRate[0])!==Math.sign(r.swissFiniteRate[0])).length};
  summary.nearStations=[2,3,4,5,6,7,8,9].map(body=>{
    const row=rows.filter(r=>r.body===body).reduce((a,b)=>Math.abs(b.swissFiniteRate[0])<Math.abs(a.swissFiniteRate[0])?b:a);
    return {body,year:row.year,month:row.month,engineLongitudeRate:row.engineRate[0],swissFiniteLongitudeRate:row.swissFiniteRate[0]};
  });
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-swiss-seasonal-speeds/1',status:'EXPLORATORY',kernelSha256:expected,swissVersion:swiss.swe_version(),
    notes:'Apparent geocentric ecliptic of date, TDB vs approximate TT, shared DE441, Swiss model/frame differ. Fourth-order finite differences are not independent orbital truth.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
module.exports={extrapolatedRate};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
