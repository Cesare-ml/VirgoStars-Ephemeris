'use strict';
// SPDX-License-Identifier: CPAL-1.0. Exploratory Horizons comparison.
// Local reference-data orchestration; uses existing coordinate-only diagnostic.
const fs=require('fs'),path=require('path'),crypto=require('crypto');
const {icrfFiniteRates}=require(path.resolve('bindings/node/scripts/icrf-angular-rates.cjs'));
const [input,out]=process.argv.slice(2);
if(!input||!out||fs.existsSync(out))throw Error('usage: <existing-six-cases-json> <new-output-json>');
const previous=JSON.parse(fs.readFileSync(input));
const commands={'Sun':'10','Moon':'301','Mercury':'199','Jupiter Barycenter':'5'};
const centers=[['Earth','399'],['Solar System Barycenter','0']];
const offsets=[-0.01,-0.002,-0.0005,0,0.0005,0.002,0.01];
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function main(){
 const results=[];
 for(const item of previous.rows)for(const [center,id] of centers){
  const epoch=item.jdTdb,tdb=offsets.map(x=>epoch+x),epochs=tdb.map(tt),command=commands[item.name];
  const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:`'500@${id}'`,TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'45,46'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
  const url='https://ssd.jpl.nasa.gov/api/horizons.api?'+params,res=await fetch(url,{signal:AbortSignal.timeout(30000)});
  if(!res.ok)throw Error('HTTP '+res.status);
  const raw=(await res.json()).result;
  if(!raw.includes(`Target body name: ${item.name} (${command})`)||!new RegExp(`Center body name: ${center} \\(${id}\\).*\\{source: DE441\\}`).test(raw)||!/Target body name:.*\{source: DE441\}/.test(raw))throw Error('metadata mismatch '+item.name+' '+center);
  const lines=raw.split(/\r?\n/),header=lines.find(x=>x.includes('RA_(ICRF-a-app)'));
  if(!header||!header.includes('Date_________JDTT'))throw Error('bad header');
  const cols=header.split(',').map(x=>x.trim()),idx=['RA_(ICRF-a-app)','DEC_(ICRF-a-app)','I_dRA*cosD','I_d(DEC)/dt'].map(x=>cols.indexOf(x));
  if(idx.some(x=>x<0))throw Error('missing columns');
  const a=raw.indexOf('$$SOE'),b=raw.indexOf('$$EOE'); if(a<0||b<=a)throw Error('missing output');
  const rows=raw.slice(a+5,b).trim().split(/\r?\n/).map((line,i)=>{let c=line.split(',').map(x=>x.trim()),v=idx.map(x=>Number(c[x]));if(!c[0]||idx.some(x=>!c[x])||Math.abs(Number(c[0])-epochs[i])>1e-8||!v.every(Number.isFinite))throw Error('bad row');return {raDeg:v[0],decDeg:v[1],raCosDecArcsecHour:v[2],decArcsecHour:v[3]}});
  if(rows.length!==7)throw Error('wrong count');
  const steps=[0,1,2].map(i=>({h:Math.abs(offsets[i]),derivative:icrfFiniteRates(rows[i],rows[3],rows[6-i],tdb[6-i]-tdb[i])}));
  results.push({name:item.name,year:item.year,center,tdb:epoch,q46:{ra:rows[3].raCosDecArcsecHour,dec:rows[3].decArcsecHour},steps,url,raw});
 }
 fs.writeFileSync(out,JSON.stringify({status:'EXPLORATORY',plan:'Q46_INVESTIGATION_PLAN_2026-11-18.md',sourceSha256:crypto.createHash('sha256').update(fs.readFileSync(input)).digest('hex'),capturedAtUtc:new Date().toISOString(),results},null,2)+'\n',{flag:'wx'});
 console.log(JSON.stringify(results.map(({name,year,center,q46,steps})=>({name,year,center,q46,steps}))));
}
main().catch(e=>{console.error(e);process.exitCode=1});
