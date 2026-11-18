'use strict';
// SPDX-License-Identifier: CPAL-1.0. Reference-model stability diagnostic.
const fs=require('node:fs'),path=require('node:path'),{execFileSync}=require('node:child_process');
const {julday}=require('../index.js');
const {parse}=require('./compare-horizons-orientation-components.cjs');
const {unit}=require('./compare-horizons-frame-isolation.cjs');
const {fitSmallRotation}=require('./analyze-horizons-frame-rotation.cjs');
function seriesSummary(rows,key){
  if(!Array.isArray(rows)||!rows.length)throw new Error('empty series');
  if(!rows.every(r=>Array.isArray(r[key]?.rotationArcsec)&&r[key].rotationArcsec.length===3&&r[key].rotationArcsec.every(Number.isFinite)&&Number.isFinite(r[key].rmsResidualArcsec)&&Number.isFinite(r[key].minPivot)))throw new Error('invalid series');
  const mean=[0,1,2].map(i=>rows.reduce((s,r)=>s+r[key].rotationArcsec[i],0)/rows.length);
  return {meanArcsec:mean,magnitudeArcsec:Math.hypot(...mean),maxDeviationArcsec:Math.max(...rows.map(r=>Math.hypot(...r[key].rotationArcsec.map((x,i)=>x-mean[i])))),maxFitRmsArcsec:Math.max(...rows.map(r=>r[key].rmsResidualArcsec)),minPivot:Math.min(...rows.map(r=>r[key].minPivot))};
}
async function main(){
  const [executable,out,...extra]=process.argv.slice(2);
  if(!executable||!out||extra.length||fs.existsSync(out)||path.resolve(executable)===path.resolve(out))throw new Error('usage: <projection-executable> <new-report>');
  const years=[-9998,2000],epochs=years.flatMap(y=>Array.from({length:13},(_,i)=>julday(y,7,1,12,true)+30*i));
  const bodies=[['Sun','10'],['Moon','301'],['Venus','299'],['Mars Barycenter','4'],['Jupiter Barycenter','5'],['Saturn Barycenter','6']],responses=[];
  for(const [name,id] of bodies){
    const q=new URLSearchParams({format:'json',COMMAND:`'${id}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'2,31,45'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
    const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${q}`;let response;for(let n=0;n<3;n++){response=await fetch(url,{signal:AbortSignal.timeout(30000)});if(response.ok||![429,502,503,504].includes(response.status))break;if(n<2)await new Promise(r=>setTimeout(r,1500*(n+1)));}
    if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);const raw=(await response.json()).result;
    responses.push({name,id,url,raw,coordinates:parse(raw,name,id,epochs)});
  }
  const input=responses.flatMap(r=>r.coordinates.map((c,i)=>`${epochs[i]},${c.raDeg},${c.decDeg}`)).join('\n')+'\n';
  const lines=execFileSync(path.resolve(executable),[],{input,encoding:'utf8',maxBuffer:1048576,timeout:30000}).trim().split(/\r?\n/);
  if(lines.length!==epochs.length*bodies.length)throw new Error('wrong projection count');
  const values=lines.map(line=>{const f=line.split(',');if(f.length!==10||!f.every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid projection');return f.map(Number);});
  const rows=epochs.map((tt,i)=>{
    const ref=responses.map(r=>unit(r.coordinates[i].ra,r.coordinates[i].dec)),mean=responses.map((r,b)=>unit(values[b*epochs.length+i][0],values[b*epochs.length+i][1])),actual=responses.map((r,b)=>unit(values[b*epochs.length+i][2],values[b*epochs.length+i][3]));
    return {year:years[Math.floor(i/13)],sample:i%13,jdTt:tt,meanFit:fitSmallRotation(mean,ref),trueFit:fitSmallRotation(actual,ref)};
  });
  const summary=years.map(year=>{const group=rows.filter(r=>r.year===year);return {year,mean:seriesSummary(group,'meanFit'),true:seriesSummary(group,'trueFit')};});
  fs.writeFileSync(out,JSON.stringify({schema:'horizons-frame-stability/1',status:'EXPLORATORY',notes:'Thirteen TT samples 30 days apart in each era; small-rotation fit to six bodies, local mean and true vs Horizons true. Temporal variation includes different nutation and model conventions; not isolated physical precession error.',summary,rows,responses},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
module.exports={seriesSummary};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
