'use strict';
// SPDX-License-Identifier: CPAL-1.0. Exploratory comparison of JPL outputs.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {icrfFiniteRates}=require('./icrf-angular-rates.cjs');
const [input,retarded,out,...extra]=process.argv.slice(2);
if(!input||!retarded||!out||extra.length||fs.existsSync(out)||new Set([input,retarded,out].map(x=>path.resolve(x))).size!==3)throw Error('usage: <cases-json> <retarded-json> <new-report>');
const bytes=fs.readFileSync(input),rb=fs.readFileSync(retarded),data=JSON.parse(bytes),rd=JSON.parse(rb);
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function main(){
 const cases=[];
 for(const item of data.rows){
  if(item.name!=='Mercury'||!Number.isFinite(item.jdTdb))throw Error('bad case');
  const prior=rd.results.filter(x=>x.center==='Solar System Barycenter'&&x.name===item.name&&x.year===item.year);
  if(prior.length!==1)throw Error('missing or ambiguous Rust result');
  const tdb=[item.jdTdb-0.01,item.jdTdb-0.002,item.jdTdb,item.jdTdb+0.002,item.jdTdb+0.01],epochs=tdb.map(tt);
  const params=new URLSearchParams({format:'json',COMMAND:"'199'",OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@0'",TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'1,45,46'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
  const url='https://ssd.jpl.nasa.gov/api/horizons.api?'+params,res=await fetch(url,{signal:AbortSignal.timeout(30000)});if(!res.ok)throw Error('HTTP '+res.status);
  const raw=(await res.json()).result;
  if(!/Target body name: Mercury \(199\).*\{source: DE441\}/.test(raw)||!/Center body name: Solar System Barycenter \(0\).*\{source: DE441\}/.test(raw))throw Error('wrong source or center');
  const header=raw.split(/\r?\n/).find(x=>x.includes('RA_(ICRF-a-app)'));if(!header||!header.includes('Date_________JDTT'))throw Error('bad header');
  const cols=header.split(',').map(x=>x.trim()),idx=['R.A.___(ICRF)','DEC____(ICRF)','RA_(ICRF-a-app)','DEC_(ICRF-a-app)','I_dRA*cosD','I_d(DEC)/dt'].map(x=>cols.indexOf(x));if(idx.some(x=>x<0))throw Error('missing columns');
  const start=raw.indexOf('$$SOE'),end=raw.indexOf('$$EOE');if(start<0||end<=start)throw Error('missing ephemeris');
  const lines=raw.slice(start+5,end).trim().split(/\r?\n/);if(lines.length!==5)throw Error('bad count');
  const rows=lines.map((line,i)=>{const f=line.split(',').map(x=>x.trim()),v=idx.map(x=>Number(f[x]));if(!f[0]||idx.some(x=>!f[x])||!Number.isFinite(Number(f[0]))||Math.abs(Number(f[0])-epochs[i])>1e-8||!v.every(Number.isFinite))throw Error('bad row');return {q1:{raDeg:v[0],decDeg:v[1]},q45:{raDeg:v[2],decDeg:v[3]},q46:{ra:v[4],dec:v[5]}}});
  const rates=[0,1].map(i=>({halfStepDays:tdb[2]-tdb[i],q1:icrfFiniteRates(rows[i].q1,rows[2].q1,rows[4-i].q1,tdb[4-i]-tdb[i]),q45:icrfFiniteRates(rows[i].q45,rows[2].q45,rows[4-i].q45,tdb[4-i]-tdb[i])}));
  const small=rates[1].q1,full=prior[0].full,deltaRa=full.raCosDecArcsecHour-small.raCosDecArcsecHour,deltaDec=full.decArcsecHour-small.decArcsecHour;
  cases.push({year:item.year,jdTdb:item.jdTdb,rates,q46:rows[2].q46,rust:prior[0],deltaRa,deltaDec,overDiagnosticLimit:Math.max(Math.abs(deltaRa),Math.abs(deltaDec))>0.001,url,raw});
 }
 if(hash(fs.readFileSync(input))!==hash(bytes)||hash(fs.readFileSync(retarded))!==hash(rb))throw Error('input changed');
 fs.writeFileSync(out,JSON.stringify({status:'EXPLORATORY',inputSha256:hash(bytes),retardedSha256:hash(rb),capturedAtUtc:new Date().toISOString(),limitArcsecHour:0.001,failures:cases.filter(x=>x.overDiagnosticLimit).length,cases},null,2)+'\n',{flag:'wx'});
 console.log(JSON.stringify(cases.map(({year,rates,deltaRa,deltaDec,overDiagnosticLimit})=>({year,rates,deltaRa,deltaDec,overDiagnosticLimit}))));
}
main().catch(e=>{console.error(e);process.exitCode=1});
