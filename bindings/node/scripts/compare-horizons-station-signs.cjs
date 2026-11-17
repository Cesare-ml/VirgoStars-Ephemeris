'use strict';
// SPDX-License-Identifier: CPAL-1.0. Explicit online directional check around selected stations.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine}=require('../index.js'),{parseEcliptic}=require('./compare-horizons-ecliptic-stations.cjs'),{extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
async function fileSha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
const bodies=[[2,'199','Mercury'],[3,'299','Venus'],[4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],[7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter'],[9,'9','Pluto Barycenter']];
function robustSign(rate,bound){
  if(!Number.isFinite(rate)||!Number.isFinite(bound)||bound<0)throw new Error('invalid rate or bound');
  return Math.abs(rate)>bound?Math.sign(rate):0;
}
function expectedSigns(kind){if(kind==='retrograde')return [1,-1];if(kind==='direct')return [-1,1];throw new Error('invalid station kind');}
async function main(){
  const [kernel,kernelSha,report,reportSha,out,step,...extra]=process.argv.slice(2),h=step===undefined?0.5:Number(step);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!report||!/^[a-f0-9]{64}$/.test(reportSha||'')||!out||extra.length||![0.25,0.5,1].includes(h)||fs.existsSync(out)||new Set([kernel,report,out].map(p=>path.resolve(p))).size!==3||await fileSha(kernel)!==kernelSha)throw new Error('usage: <de441> <kernel-sha> <ecliptic-report> <report-sha> <new-report> [0.25|0.5|1]');
  const bytes=fs.readFileSync(report);if(sha(bytes)!==reportSha)throw new Error('reference report hash mismatch');const data=JSON.parse(bytes);
  if(data.schema!=='virgostars-horizons-ecliptic-stations/1'||data.kernelSha256!==kernelSha||!Array.isArray(data.rows)||data.rows.length!==112)throw new Error('invalid input report');
  const engine=new Engine(path.resolve(kernel)),d=120/86400,bound=1.5*0.5e-7*3600/h,rows=[],sources=[];
  // Interleaved offsets are sorted chronologically; no hidden root search.
  const offsets=[-2*h-d,-2*h+d,-h-d,-h+d,h-d,h+d,2*h-d,2*h+d];
  for(const [id,command,name] of bodies){
    const events=data.rows.filter(r=>r.body===name).sort((a,b)=>a.jdTdb-b.jdTdb);
    if(events.length!==14||new Set(events.map(r=>`${r.seedYear}:${r.kind}`)).size!==14)throw new Error('invalid event selection');
    const requests=[];
    for(let start=0;start<events.length;start+=6){
      const group=events.slice(start,start+6),epochs=group.flatMap(r=>offsets.map(o=>tt(r.jdTdb+o)));
      const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'31'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
      const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
      const raw=(await response.json()).result,ref=parseEcliptic(raw,name,command,epochs);requests.push({url,raw});
      for(let i=0;i<group.length;i++){
        const e=group[i],p=ref.slice(i*8,(i+1)*8),expect=expectedSigns(e.kind),horizons=[],local=[];
        for(let side=0;side<2;side++){
          const center=e.jdTdb+(side===0?-d:d),v=[0,2,4,6].map(j=>p[j+side].lon),
            rate=extrapolatedRate(...v,center,h,true)*3600,engineRate=engine.experimentalLongTermApparentTdb(center,id).xx[3]*3600;
          horizons.push({rateArcsecDay:rate,sign:robustSign(rate,bound)});local.push({rateArcsecDay:engineRate,sign:robustSign(engineRate,0)});
        }
        rows.push({body:name,seedYear:e.seedYear,kind:e.kind,jdTdb:e.jdTdb,expected:expect,horizons,engine:local,
          jplBothExpected:horizons.every((v,i)=>v.sign===expect[i]),engineBothExpected:local.every((v,i)=>v.sign===expect[i])});
      }
    }
    sources.push({name,command,requests});
  }
  if(await fileSha(kernel)!==kernelSha||sha(fs.readFileSync(report))!==reportSha)throw new Error('input changed');
  const summary={cases:rows.length,sideOffsetSeconds:120,halfStepTdbDays:h,assumedRoundingBoundArcsecDay:bound,
    jplBothExpected:rows.filter(r=>r.jplBothExpected).length,engineBothExpected:rows.filter(r=>r.engineBothExpected).length,
    jplAmbiguous:rows.filter(r=>r.horizons.some(v=>v.sign===0)).length,
    jplUnexpected:rows.filter(r=>r.horizons.some((v,i)=>v.sign!==0&&v.sign!==r.expected[i])).length};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-station-signs/1',status:'EXPLORATORY',kernelSha256:kernelSha,referenceSha256:reportSha,
    notes:'JPL apparent ecliptic positions use different orientation than engine but share DE441; signs only test derivatives at ±120 s. Rounding bound assumes nearest rounding of seven decimals and excludes truncation/model/time errors.',summary,rows,sources},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
  if(summary.jplUnexpected||summary.engineBothExpected!==summary.cases)process.exitCode=1;
}
module.exports={robustSign,expectedSigns};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
