'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Same DE441, different implementations and frames; not an orbit accuracy bound.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
const {Engine}=require('../index.js');
const {epochGrid}=require('./sample-horizons-frame-epochs.cjs');
const {parse}=require('./compare-horizons-orientation-components.cjs');
const {unit,separation}=require('./compare-horizons-frame-isolation.cjs');
const bodies=[[0,'10','Sun'],[1,'301','Moon'],[2,'199','Mercury'],[3,'299','Venus'],[4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],[7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter'],[9,'9','Pluto Barycenter']];
const extraIds=new Set(['199','7','8','9']);
function ttToTdb(tt){
  if(!Number.isFinite(tt))throw new Error('invalid TT');
  return tt+0.001657*Math.sin((357.5291092+35999.0502909*((tt-2451545)/36525))*Math.PI/180)/86400;
}
function direction(position){
  if(!Array.isArray(position)||position.length!==3||!position.every(Number.isFinite))throw new Error('invalid Cartesian position');
  const length=Math.hypot(...position);if(!Number.isFinite(length)||length<=0)throw new Error('invalid Cartesian range');
  return position.map(x=>x/length);
}
function checkCoordinates(rows){
  if(!Array.isArray(rows)||!rows.length||!rows.every(r=>r&&[r.raDeg,r.decDeg,r.ra,r.dec,r.lon,r.lat].every(Number.isFinite)&&[r.raDeg,r.ra,r.lon].every(x=>x>=0&&x<360)&&[r.decDeg,r.dec,r.lat].every(x=>Math.abs(x)<=90)))throw new Error('invalid Horizons angles');
  return rows;
}
// Numerical sanity check only, not a scientific acceptance tolerance.
function checkConsistency(r){
  const keys=['icrfArcsec','poleArcsec','directArcsec','samePoleArcsec','sameDirectArcsec','poleFrameArcsec','directFrameArcsec'];
  if(!keys.every(k=>Number.isFinite(r[k])&&r[k]>=0))throw new Error('invalid separations');
  const epsilon=1e-7;
  if(Math.abs(r.icrfArcsec-r.samePoleArcsec)>epsilon||Math.abs(r.icrfArcsec-r.sameDirectArcsec)>epsilon||
    Math.abs(r.poleArcsec-r.poleFrameArcsec)>r.samePoleArcsec+epsilon||Math.abs(r.directArcsec-r.directFrameArcsec)>r.sameDirectArcsec+epsilon)
    throw new Error('inconsistent frame decomposition');
}
function summarize(rows){
  if(!Array.isArray(rows)||!rows.length||!rows.every(r=>Number.isInteger(r.id)&&[r.icrfArcsec,r.poleArcsec,r.directArcsec,r.samePoleArcsec,r.sameDirectArcsec].every(x=>Number.isFinite(x)&&x>=0)))throw new Error('invalid results');
  const max=(list,key)=>{const r=list.reduce((a,b)=>b[key]>a[key]?b:a);return {arcsec:r[key],year:r.year,month:r.month,id:r.id};};
  return {cases:rows.length,byBody:bodies.map(([id,,name])=>{const list=rows.filter(r=>r.id===id);if(!list.length)throw new Error(`missing ${name}`);return {id,name,cases:list.length,icrf:max(list,'icrfArcsec'),pole:max(list,'poleArcsec'),direct:max(list,'directArcsec'),samePole:max(list,'samePoleArcsec'),sameDirect:max(list,'sameDirectArcsec')};}),
    all:Object.fromEntries(['icrfArcsec','poleArcsec','directArcsec','samePoleArcsec','sameDirectArcsec'].map(k=>[k,max(rows,k)])),
    maxSameAxisInvarianceDifferenceArcsec:Math.max(...rows.flatMap(r=>[Math.abs(r.icrfArcsec-r.samePoleArcsec),Math.abs(r.icrfArcsec-r.sameDirectArcsec)]))};
}
function digest(file){const h=crypto.createHash('sha256'),fd=fs.openSync(file,'r'),buf=Buffer.allocUnsafe(1024*1024);try{for(;;){const n=fs.readSync(fd,buf,0,buf.length,null);if(!n)break;h.update(buf.subarray(0,n));}}finally{fs.closeSync(fd);}return h.digest('hex');}
async function main(){
  const [kernel,kernelHash,source,sourceHash,exe,out,prior,priorHash,...extra]=process.argv.slice(2);
  const hashes=[kernelHash,sourceHash,...(prior?[priorHash]:[])];
  if(!kernel||!source||!exe||!out||extra.length||Boolean(prior)!==Boolean(priorHash)||!hashes.every(x=>/^[a-f0-9]{64}$/.test(x||''))||fs.existsSync(out)||new Set([kernel,source,exe,out,...(prior?[prior]:[])].map(p=>path.resolve(p))).size!==(prior?5:4))throw new Error('usage: KERNEL KERNEL_SHA FRAME_REPORT FRAME_SHA PROJECTOR NEW_REPORT [PRIOR_PLANET_REPORT PRIOR_SHA]');
  if(digest(kernel)!==kernelHash||digest(source)!==sourceHash||(prior&&digest(prior)!==priorHash))throw new Error('input hash mismatch');
  const addon=path.resolve(__dirname,'../native/virgostars_ephemeris.node');
  if([kernel,source,exe,out,...(prior?[prior]:[])].some(p=>path.resolve(p)===addon))throw new Error('addon must be a distinct input');
  const exeHash=digest(exe),addonHash=digest(addon),frame=JSON.parse(fs.readFileSync(source,'utf8'));
  if(frame.schema!=='horizons-frame-epoch-holdout/1'||!Array.isArray(frame.responses)||frame.responses.length!==6)throw new Error('invalid frame report');
  const previous=prior?JSON.parse(fs.readFileSync(prior,'utf8')):null;
  if(previous&&(previous.schema!=='horizons-planet-frame-grid/1'||previous.frameReportSha256!==sourceHash||previous.kernelSha256!==kernelHash||!Array.isArray(previous.extraResponses)))throw new Error('invalid prior planet report');
  const grid=epochGrid(),epochs=grid.map(x=>x.jdTt),responses=[];
  for(const [id,command,name] of bodies){
    const parts=[];
    for(let offset=0;offset<epochs.length;offset+=30){
      const batch=epochs.slice(offset,offset+30);
      let raw,url;
      if(!extraIds.has(command)||previous){
        const list=extraIds.has(command)?previous.extraResponses:frame.responses;
        const matches=list.filter(r=>r.name===name&&r.id===command);
        if(matches.length!==1||!Array.isArray(matches[0].parts)||matches[0].parts.length!==2)throw new Error(`missing ${name} response`);
        const part=matches[0].parts[offset/30];
        if(!part||typeof part.raw!=='string'||typeof part.url!=='string')throw new Error(`invalid ${name} response`);
        ({raw,url}=part);
      }else{
        const q=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",TLIST:`'${batch.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'2,31,45'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
        url=`https://ssd.jpl.nasa.gov/api/horizons.api?${q}`;
        let response;
        for(let attempt=0;attempt<3;attempt++){response=await fetch(url,{signal:AbortSignal.timeout(30000)});if(response.ok||![429,502,503,504].includes(response.status))break;if(attempt<2)await new Promise(resolve=>setTimeout(resolve,1500*(attempt+1)));}
        if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
        const payload=await response.json();if(payload.error)throw new Error(`Horizons: ${payload.error}`);raw=payload.result;
      }
      parts.push({url,raw,coordinates:checkCoordinates(parse(raw,name,command,batch))});
    }
    responses.push({id:command,bodyId:id,name,parts,coordinates:parts.flatMap(p=>p.coordinates)});
  }
  const input=responses.flatMap(r=>r.coordinates.map((c,i)=>`${epochs[i]},${c.raDeg},${c.decDeg}`)).join('\n')+'\n';
  const lines=execFileSync(path.resolve(exe),[],{input,encoding:'utf8',maxBuffer:2*1048576,timeout:30000}).trim().split(/\r?\n/);
  if(lines.length!==bodies.length*epochs.length)throw new Error('wrong projection count');
  const projected=lines.map(line=>{const f=line.split(',');if(f.length!==10||!f.every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid projection');return f.map(Number);});
  const engine=new Engine(path.resolve(kernel)),sun=responses[0],rows=[];
  for(let b=0;b<responses.length;b++)for(let i=0;i<epochs.length;i++){
    const ref=responses[b].coordinates[i],p=projected[b*epochs.length+i],tdb=ttToTdb(epochs[i]);
    const icrf=engine.experimentalApparentIcrfTdb(tdb,responses[b].bodyId),pole=engine.experimentalLongTermApparentTdb(tdb,responses[b].bodyId),direct=engine.experimentalLongTermDirectObliquityTdb(tdb,responses[b].bodyId);
    if(![pole.xx,direct.xx].every(v=>v.length===6&&v.every(Number.isFinite)))throw new Error('invalid local output');
    const refIcrf=unit(ref.raDeg,ref.decDeg),refEcliptic=unit(ref.lon,ref.lat),sunIcrf=unit(sun.coordinates[i].raDeg,sun.coordinates[i].decDeg);
    const localPole=unit(pole.xx[0],pole.xx[1]),localDirect=unit(direct.xx[0],direct.xx[1]),projectedPole=unit(p[4],p[5]),projectedDirect=unit(p[6],p[7]);
    const row={...grid[i],id:responses[b].bodyId,body:responses[b].name,jdTdb:tdb,solarElongationDeg:separation(refIcrf,sunIcrf)/3600,
      icrfArcsec:separation(direction(icrf.positionKm),refIcrf),poleArcsec:separation(localPole,refEcliptic),directArcsec:separation(localDirect,refEcliptic),
      samePoleArcsec:separation(localPole,projectedPole),sameDirectArcsec:separation(localDirect,projectedDirect),
      poleFrameArcsec:separation(projectedPole,refEcliptic),directFrameArcsec:separation(projectedDirect,refEcliptic)};
    checkConsistency(row);rows.push(row);
  }
  if(digest(kernel)!==kernelHash||digest(source)!==sourceHash||digest(exe)!==exeHash||digest(addon)!==addonHash||(prior&&digest(prior)!==priorHash))throw new Error('input changed');
  const report={schema:'horizons-planet-frame-grid/1',status:'EXPLORATORY',kernelSha256:kernelHash,frameReportSha256:sourceHash,projectorSha256:exeHash,addonSha256:addonHash,priorReportSha256:priorHash||null,
    notes:'Horizons and local states use DE441; geocentric airless apparent directions. Fixed TT approximately converted to TDB; frame conventions differ. Same-axis comparisons use Horizons ICRF projected by local orientation, not independent orbit accuracy. No tolerance accepted.',
    summary:summarize(rows),rows,extraResponses:responses.filter(r=>extraIds.has(r.id)).map(({id,name,parts})=>({id,name,parts}))};
  fs.writeFileSync(out,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(report.summary));
}
module.exports={ttToTdb,direction,checkCoordinates,checkConsistency,summarize};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
