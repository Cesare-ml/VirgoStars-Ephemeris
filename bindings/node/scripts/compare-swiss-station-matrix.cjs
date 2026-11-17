'use strict';
// SPDX-License-Identifier: CPAL-1.0. Offline sampled station matrix; not an event API.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs');
const {bisect,separation}=require('./compare-swiss-station-roots.cjs');
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
// Find closest sign-changing bracket of each type to the seed. No claim about unobserved crossings.
function nearestBrackets(f,seed,radius,step){
  if(typeof f!=='function'||![seed,radius,step].every(Number.isFinite)||step<=0||radius<=0||!Number.isInteger(2*radius/step))throw new Error('invalid scan');
  const found={retrograde:[],direct:[]};let lo=seed-radius,prev=f(lo);
  if(!Number.isFinite(prev))throw new Error('nonfinite speed');
  for(let i=1;i<=2*radius/step;i++){
    const hi=seed-radius+i*step,next=f(hi);if(!Number.isFinite(next))throw new Error('nonfinite speed');
    if(prev>0&&next<=0)found.retrograde.push([lo,hi]);
    if(prev<0&&next>=0)found.direct.push([lo,hi]);
    lo=hi;prev=next;
  }
  return Object.fromEntries(Object.entries(found).map(([kind,items])=>[kind,items.sort((a,b)=>Math.abs((a[0]+a[1])/2-seed)-Math.abs((b[0]+b[1])/2-seed))[0]??null]));
}
function stats(values){
  if(!values.length)return null;
  const a=values.map(Math.abs).sort((x,y)=>x-y),p=n=>a[Math.ceil(a.length*n)-1];
  return {count:a.length,median:p(0.5),p95:p(0.95),p99:p(0.99),max:a.at(-1)};
}
async function main(){
  const [kernel,expected,swissPackage,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!swissPackage||!out||extra.length||fs.existsSync(out)||await sha(kernel)!==expected)throw new Error('usage: <de441> <sha256> <swisseph> <new-report>');
  const swiss=require(path.resolve(swissPackage)),engine=new Engine(path.resolve(kernel)),tol=0.00001;
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  const reference=(jd,body)=>{const flags=swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED,r=swiss.swe_calc(tt(jd),body,flags);
    if(!r||r.error||(r.rflag&flags)!==flags||![r.longitude,r.longitudeSpeed].every(Number.isFinite))throw new Error(`Swiss JPL unavailable: ${body} ${jd}`);return r;};
  const finite=(jd,body,h)=>{const at=d=>reference(d,body).longitude;return extrapolatedRate(at(jd-2*h),at(jd-h),at(jd+h),at(jd+2*h),jd,h,true);};
  const speed=(jd,body)=>{const r=engine.experimentalLongTermApparentTdb(jd,body).xx;if(r.length!==6||!r.every(Number.isFinite))throw new Error('invalid engine state');return r[3];};
  const center=b=>(b.lo+b.hi)/2,rows=[];
  for(let year=50;year<=2950;year+=50)for(let body=2;body<=9;body++){
    const seed=julday(year,7,1,12,true),found=nearestBrackets(jd=>finite(jd,body,0.02),seed,500,5);
    for(const kind of ['retrograde','direct']){
      const initial=found[kind];if(!initial){rows.push({year,body,kind,status:'missing-bracket'});continue;}
      try{
        // Expand by one day to accommodate modest differences between definitions.
        const [lo,hi]=[initial[0]-1,initial[1]+1],engineBracket=bisect(jd=>speed(jd,body),lo,hi,tol),
          finiteBracket=bisect(jd=>finite(jd,body,0.02),lo,hi,tol),publishedBracket=bisect(jd=>reference(jd,body).longitudeSpeed,lo,hi,tol);
        const steps=[0.01,0.04].map(h=>{const b=bisect(jd=>finite(jd,body,h),lo,hi,tol);return {halfStepDays:h,bracket:b};});
        const er=center(engineBracket),fr=center(finiteBracket),pr=center(publishedBracket),mid=(er+pr)/2,
          pos=engine.experimentalApparentIcrfTdb(er,body).positionKm,sun=engine.experimentalApparentIcrfTdb(er,0).positionKm,
          eSign=Math.sign(speed(mid,body)),sSign=Math.sign(reference(mid,body).longitudeSpeed);
        rows.push({year,body,kind,status:'ok',initial,engineBracket,finiteBracket,publishedBracket,otherSteps:steps,
          engineMinusFiniteSeconds:(er-fr)*86400,publishedMinusFiniteSeconds:(pr-fr)*86400,
          publishedMinusEngineSeconds:(pr-er)*86400,sunSeparationDegrees:separation(pos,sun),
          midpointEngineSign:eSign,midpointSwissPublishedSign:sSign});
      }catch(e){rows.push({year,body,kind,status:'unresolved',initial,reason:String(e.message)});}
    }
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const ok=rows.filter(r=>r.status==='ok'),usable=ok.filter(r=>r.sunSeparationDegrees>=5),measure=a=>({count:a.length,
    engineVsPositionSeconds:stats(a.map(r=>r.engineMinusFiniteSeconds)),publishedVsPositionSeconds:stats(a.map(r=>r.publishedMinusFiniteSeconds)),
    publishedVsEngineSeconds:stats(a.map(r=>r.publishedMinusEngineSeconds)),
    midpointSignMismatch:a.filter(r=>Math.abs(r.publishedMinusEngineSeconds)>3&&r.midpointEngineSign!==r.midpointSwissPublishedSign).length,
    midpointChecked:a.filter(r=>Math.abs(r.publishedMinusEngineSeconds)>3).length});
  const summary={expected:59*8*2,records:rows.length,ok:ok.length,unresolved:rows.filter(r=>r.status!=='ok').length,
    withinFiveDegrees:ok.length-usable.length,all:measure(ok),atLeastFiveDegrees:measure(usable),
    byBody:Object.fromEntries(Array.from({length:8},(_,i)=>[i+2,measure(usable.filter(r=>r.body===i+2))])),
    byDirection:Object.fromEntries(['retrograde','direct'].map(k=>[k,measure(usable.filter(r=>r.kind===k))]))};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-swiss-station-matrix/1',status:'EXPLORATORY',kernelSha256:expected,swissVersion:swiss.swe_version(),
    notes:'Nearest crossings in ±500 days around each sampled TDB epoch, Swiss position derivatives vs engine rate and Swiss reported speed; same DE441, frame differences, approximate TT. Numerical brackets are not physical accuracy.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));if(summary.unresolved||summary.records!==summary.expected)process.exitCode=1;
}
module.exports={nearestBrackets,stats};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
