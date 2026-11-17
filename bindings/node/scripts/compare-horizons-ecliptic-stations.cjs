'use strict';
// SPDX-License-Identifier: CPAL-1.0. Explicit online comparison, different orientation models.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine}=require('../index.js'),{extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs'),{stats}=require('./compare-swiss-station-matrix.cjs');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
async function fileSha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
const wrap=x=>((x%360)+540)%360-180;
const bodies=[[2,'199','Mercury'],[3,'299','Venus'],[4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],[7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter'],[9,'9','Pluto Barycenter']];
function parseEcliptic(raw,name,command,epochs){
  if(typeof raw!=='string'||!Array.isArray(epochs)||!epochs.length||!epochs.every(Number.isFinite)||!raw.includes(`Target body name: ${name} (${command})`)
    ||!/Target body name:.*\{source: DE441\}/.test(raw)||!/Center body name: Earth \(399\).*\{source: DE441\}/.test(raw)
    ||!raw.includes('Observer-centered IAU76/80 ecliptic-of-date')||!raw.includes('Atmos refraction: NO (AIRLESS)'))throw new Error('invalid ecliptic metadata');
  const header=raw.split(/\r?\n/).find(s=>s.includes('Date_________JDTT')&&s.includes('ObsEcLon')&&s.includes('ObsEcLat')),
    start=raw.indexOf('$$SOE'),end=raw.indexOf('$$EOE');
  if(!header||start<0||end<=start||raw.indexOf('$$SOE',start+5)!==-1||raw.indexOf('$$EOE',end+5)!==-1)throw new Error('invalid ecliptic table');
  const cols=header.split(',').map(x=>x.trim()),lon=cols.indexOf('ObsEcLon'),lat=cols.indexOf('ObsEcLat');
  const lines=raw.slice(start+5,end).trim().split(/\r?\n/);if(lon<1||lat<1||lines.length!==epochs.length)throw new Error('invalid ecliptic columns or row count');
  return lines.map((line,i)=>{const f=line.split(',').map(x=>x.trim());
    if(!f[0]||!/^\d+\.\d{7}$/.test(f[lon]||'')||!f[lat]||![Number(f[0]),Number(f[lon]),Number(f[lat])].every(Number.isFinite)||Math.abs(Number(f[0])-epochs[i])>1e-8||Math.abs(Number(f[lat]))>90)throw new Error('invalid ecliptic row or longitude precision');
    return {lon:Number(f[lon]),lat:Number(f[lat])};});
}
// Five-point derivative and curvature from same-frame longitudes. Estimated
// Newton shift is diagnostic only; this is not a bracketed independent root.
function estimate(values,center,h){
  if(!Array.isArray(values)||values.length!==5||!values.every(Number.isFinite)||!Number.isFinite(center)||!Number.isFinite(h)||h<=0)throw new Error('invalid stencil');
  const [m2,m1,c,p1,p2]=values,rate=extrapolatedRate(m2,m1,p1,p2,center,h,true),
    curvature=(16*(wrap(p1-c)+wrap(m1-c))-wrap(p2-c)-wrap(m2-c))/(12*h*h);
  if(!Number.isFinite(curvature)||Math.abs(curvature)<1e-12)throw new Error('invalid curvature');
  // Assuming each printed seven-decimal longitude is rounded to nearest, its
  // absolute error is <= 0.5e-7 degree. The five-point rate coefficients have
  // L1 norm 18/(12h), producing a 1.5*error/h rate bound. This is not a
  // station-time bound: curvature, model and truncation errors remain.
  const rateRoundingBoundArcsecDay=1.5*0.5e-7*3600/h;
  return {rateArcsecDay:rate*3600,curvatureArcsecDay2:curvature*3600,newtonShiftSeconds:-rate/curvature*86400,
    assumedRateRoundingBoundArcsecDay:rateRoundingBoundArcsecDay,firstOrderRoundingScaleSeconds:rateRoundingBoundArcsecDay/Math.abs(curvature*3600)*86400};
}
async function main(){
  const [kernel,kernelSha,matrix,matrixSha,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!matrix||!/^[a-f0-9]{64}$/.test(matrixSha||'')||!out||extra.length||fs.existsSync(out)||new Set([kernel,matrix,out].map(p=>path.resolve(p))).size!==3||await fileSha(kernel)!==kernelSha)throw new Error('usage: <de441> <kernel-sha> <matrix> <matrix-sha> <new-report>');
  const bytes=fs.readFileSync(matrix);if(sha(bytes)!==matrixSha)throw new Error('matrix hash mismatch');const data=JSON.parse(bytes);
  if(data.schema!=='virgostars-swiss-station-matrix/1'||data.kernelSha256!==kernelSha||!Array.isArray(data.rows)||data.rows.length!==944)throw new Error('invalid matrix');
  const engine=new Engine(path.resolve(kernel)),years=[100,500,1000,1500,1900,2000,2500],offsets=[-2,-1,-0.5,-0.25,0,0.25,0.5,1,2],steps=[0.25,0.5,1],rows=[],sources=[];
  for(const [id,command,name] of bodies){
    const events=data.rows.filter(r=>years.includes(r.year)&&r.body===id&&r.status==='ok').sort((a,b)=>a.engineBracket.lo-b.engineBracket.lo);
    if(events.length!==14||new Set(events.map(r=>`${r.year}:${r.kind}`)).size!==14)throw new Error('invalid event selection');
    const requests=[];
    for(let start=0;start<events.length;start+=6){
      const group=events.slice(start,start+6),tdb=group.flatMap(r=>{const center=(r.engineBracket.lo+r.engineBracket.hi)/2;return offsets.map(o=>center+o);}),epochs=tdb.map(tt);
      const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",
        TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'31'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
      const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
      const raw=(await response.json()).result,ref=parseEcliptic(raw,name,command,epochs);requests.push({url,raw});
      for(let i=0;i<group.length;i++){
        const event=group[i],center=(event.engineBracket.lo+event.engineBracket.hi)/2,refs=ref.slice(i*offsets.length,(i+1)*offsets.length),
          rates=Object.fromEntries(steps.map(h=>[h,estimate([-2*h,-h,0,h,2*h].map(o=>refs[offsets.indexOf(o)].lon),center,h)])),
          engineRateArcsecDay=engine.experimentalLongTermApparentTdb(center,id).xx[3]*3600;
        if(!Number.isFinite(engineRateArcsecDay))throw new Error('invalid engine rate');
        rows.push({body:name,seedYear:event.year,kind:event.kind,jdTdb:center,engineRateArcsecDay,estimates:rates});
      }
    }
    sources.push({name,command,requests});
  }
  if(await fileSha(kernel)!==kernelSha||sha(fs.readFileSync(matrix))!==matrixSha)throw new Error('input changed');
  const summary={cases:rows.length,byStep:Object.fromEntries(steps.map(h=>[h,{rateDifferenceArcsecDay:stats(rows.map(r=>r.estimates[h].rateArcsecDay-r.engineRateArcsecDay)),
    newtonShiftSeconds:stats(rows.map(r=>r.estimates[h].newtonShiftSeconds)),firstOrderRoundingScaleSeconds:stats(rows.map(r=>r.estimates[h].firstOrderRoundingScaleSeconds)),
    outsideStencil:rows.filter(r=>Math.abs(r.estimates[h].newtonShiftSeconds)>h*86400).length}]))};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-ecliptic-stations/1',status:'EXPLORATORY',kernelSha256:kernelSha,matrixSha256:matrixSha,
    notes:'JPL observer ecliptic-of-date coordinates vs engine ecliptic rates; different orientation/nutation models, shared DE441, approximate TT. Newton shifts are not independently bracketed station times.',summary,rows,sources},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
module.exports={parseEcliptic,estimate};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
