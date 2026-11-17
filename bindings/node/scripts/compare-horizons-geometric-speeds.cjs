'use strict';
// SPDX-License-Identifier: CPAL-1.0. Explicit online comparison of geometric ICRF states.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {validationYears}=require('./planet-validation-years.cjs');
const bodies=[[0,'10','Sun'],[1,'301','Moon'],[2,'199','Mercury'],[3,'299','Venus'],[4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],[7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter'],[9,'9','Pluto Barycenter']];
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
function parseVectors(raw,name,command,epochs){
  if(typeof raw!=='string'||!Array.isArray(epochs)||!epochs.length||!epochs.every(Number.isFinite)
    ||!raw.includes(`Target body name: ${name} (${command})`)||!/Target body name:.*\{source: DE441\}/.test(raw)
    ||!/Center body name: Earth \(399\).*\{source: DE441\}/.test(raw)||!raw.includes('Output units    : KM-S')
    ||!raw.includes('Output type     : GEOMETRIC cartesian states')||!raw.includes('Reference frame : ICRF'))throw new Error('invalid vector metadata');
  const start=raw.indexOf('$$SOE'),end=raw.indexOf('$$EOE'),header=raw.split(/\r?\n/).find(x=>x.includes('JDTDB')&&x.includes('Calendar Date (TDB)'));
  if(!header||start<0||end<=start||raw.indexOf('$$SOE',start+5)!==-1||raw.indexOf('$$EOE',end+5)!==-1)throw new Error('invalid vector table');
  const cols=header.split(',').map(x=>x.trim()),indices=['X','Y','Z','VX','VY','VZ'].map(x=>cols.indexOf(x));
  if(indices.some(x=>x<2)||new Set(indices).size!==6)throw new Error('missing vector columns');
  const lines=raw.slice(start+5,end).trim().split(/\r?\n/);if(lines.length!==epochs.length)throw new Error('wrong vector row count');
  return lines.map((line,i)=>{const f=line.split(',').map(x=>x.trim());
    if(!f[0]||!Number.isFinite(Number(f[0]))||Math.abs(Number(f[0])-epochs[i])>1e-8||indices.some(j=>!f[j]||!Number.isFinite(Number(f[j]))))throw new Error('invalid vector row');
    return indices.map(j=>Number(f[j]));});
}
async function main(){
  const [kernel,expected,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!out||extra.length||fs.existsSync(out)||path.resolve(kernel)===path.resolve(out)||await sha(kernel)!==expected)throw new Error('usage: <de441> <sha256> <new-report>');
  const engine=new Engine(path.resolve(kernel)),years=validationYears('0-3000-step-50');
  const epochs=years.flatMap(year=>[1,4,10].map(month=>({year,month,jd:julday(year,month,1,12,true)})));
  // Investigative per-axis thresholds, fixed before the Horizons requests.
  const velocityLimitKmS=1e-7,positionLimitKm=0.1,rows=[],sources=[];
  for(const [id,command,name] of bodies){const requests=[];
    for(let offset=0;offset<epochs.length;offset+=60){
      const group=epochs.slice(offset,offset+60),params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'VECTORS'",CENTER:"'500@399'",TLIST:`'${group.map(x=>x.jd.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TDB'",REF_SYSTEM:"'ICRF'",REF_PLANE:"'FRAME'",OUT_UNITS:"'KM-S'",VEC_CORR:"'NONE'",VEC_TABLE:"'2'",CSV_FORMAT:"'YES'"});
      const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});
      if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
      const raw=(await response.json()).result,ref=parseVectors(raw,name,command,group.map(x=>x.jd));requests.push({url,raw});
      for(let i=0;i<group.length;i++){
        const epoch=group[i],state=engine.geometricStateTdb(epoch.jd,id);
        if(state.jdTdb!==epoch.jd||state.center!=='Earth'||state.frame!=='ICRF'||state.positionKm.length!==3||state.velocityKmPerDay.length!==3||![...state.positionKm,...state.velocityKmPerDay].every(Number.isFinite))throw new Error('invalid engine state');
        const dp=state.positionKm.map((x,k)=>x-ref[i][k]),dv=state.velocityKmPerDay.map((x,k)=>x/86400-ref[i][k+3]);
        rows.push({body:name,year:epoch.year,month:epoch.month,jdTdb:epoch.jd,positionResidualKm:dp,velocityResidualKmS:dv});
      }
    }
    sources.push({name,command,requests});
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const summarize=items=>{
    let maxP={absolute:-1},maxV={absolute:-1},failures=0,sum=0;
    for(const x of items){if(x.positionResidualKm.some(v=>Math.abs(v)>positionLimitKm)||x.velocityResidualKmS.some(v=>Math.abs(v)>velocityLimitKmS))failures++;
      for(let i=0;i<3;i++){let a=Math.abs(x.positionResidualKm[i]),b=Math.abs(x.velocityResidualKmS[i]);sum+=b*b;
        if(a>maxP.absolute)maxP={absolute:a,body:x.body,year:x.year,month:x.month,axis:i};
        if(b>maxV.absolute)maxV={absolute:b,body:x.body,year:x.year,month:x.month,axis:i};}}
    return {cases:items.length,failures,maxPositionKm:maxP,maxVelocityKmS:maxV,rmsVelocityComponentKmS:Math.sqrt(sum/(items.length*3))};
  };
  const summary={...summarize(rows),epochs:epochs.length,years:years.length,months:[1,4,10],positionLimitKm,velocityLimitKmS,
    perBody:Object.fromEntries(bodies.map(([, ,name])=>[name,summarize(rows.filter(r=>r.body===name))]))};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-geometric-speeds/1',status:'EXPLORATORY',kernelSha256:expected,
    notes:'Geometric Earth-centered ICRF, TDB, no light-time/aberration; same DE441. Per-axis limits are investigative, not release criteria.',summary,rows,sources},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));if(summary.failures)process.exitCode=1;
}
module.exports={parseVectors};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
