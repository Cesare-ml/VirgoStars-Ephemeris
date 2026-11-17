'use strict';
// SPDX-License-Identifier: CPAL-1.0. Explicit online sign samples, not certified event intervals.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine}=require('../index.js'),{parseEcliptic}=require('./compare-horizons-ecliptic-stations.cjs'),{extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs');
const {robustSign,expectedSigns}=require('./compare-horizons-station-signs.cjs');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
async function fileSha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
const bodies=[[2,'199','Mercury'],[3,'299','Venus'],[4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],[7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter'],[9,'9','Pluto Barycenter']];
function stencilOffsets(radii,h){
  if(!Array.isArray(radii)||!radii.length||new Set(radii).size!==radii.length||!radii.every(v=>Number.isFinite(v)&&v>0)||!Number.isFinite(h)||h<=0)throw new Error('invalid stencil');
  const entries=radii.flatMap(radius=>[-1,1].flatMap(side=>[-2,-1,1,2].map(k=>({radius,side,k,offset:k*h+side*radius/86400})))).sort((a,b)=>a.offset-b.offset);
  if(new Set(entries.map(e=>e.offset)).size!==entries.length)throw new Error('duplicate offsets');return entries;
}
async function main(){
  const [kernel,kernelSha,report,reportSha,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!report||!/^[a-f0-9]{64}$/.test(reportSha||'')||!out||extra.length||fs.existsSync(out)||new Set([kernel,report,out].map(p=>path.resolve(p))).size!==3||await fileSha(kernel)!==kernelSha)throw new Error('usage: <de441> <kernel-sha> <ecliptic-report> <report-sha> <new-report>');
  const bytes=fs.readFileSync(report);if(sha(bytes)!==reportSha)throw new Error('report hash mismatch');const data=JSON.parse(bytes);
  if(data.schema!=='virgostars-horizons-ecliptic-stations/1'||data.kernelSha256!==kernelSha||!Array.isArray(data.rows)||data.rows.length!==112)throw new Error('invalid reference report');
  const engine=new Engine(path.resolve(kernel)),radii=[30,60,120],h=0.5,bound=1.5*0.5e-7*3600/h,offsets=stencilOffsets(radii,h),rows=[],sources=[];
  for(const [id,command,name] of bodies){
    const events=data.rows.filter(r=>r.body===name&&[100,1000,1900,2500].includes(r.seedYear)).sort((a,b)=>a.jdTdb-b.jdTdb);
    if(events.length!==8||new Set(events.map(r=>`${r.seedYear}:${r.kind}`)).size!==8)throw new Error('invalid event selection');
    const requests=[];
    for(let start=0;start<events.length;start+=2){
      const group=events.slice(start,start+2),epochs=group.flatMap(r=>offsets.map(o=>tt(r.jdTdb+o.offset)));
      const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'31'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
      const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
      const raw=(await response.json()).result,ref=parseEcliptic(raw,name,command,epochs);requests.push({url,raw});
      for(let i=0;i<group.length;i++){
        const event=group[i],p=ref.slice(i*offsets.length,(i+1)*offsets.length),expected=expectedSigns(event.kind),windows={};
        for(const radius of radii){const jpl=[],local=[];
          for(const side of [-1,1]){
            const center=event.jdTdb+side*radius/86400,values=[-2,-1,1,2].map(k=>p[offsets.findIndex(o=>o.radius===radius&&o.side===side&&o.k===k)].lon),
              rate=extrapolatedRate(...values,center,h,true)*3600,engineRate=engine.experimentalLongTermApparentTdb(center,id).xx[3]*3600;
            jpl.push({rateArcsecDay:rate,sign:robustSign(rate,bound)});local.push({rateArcsecDay:engineRate,sign:robustSign(engineRate,0)});
          }
          windows[radius]={jpl,engine:local,jplBothExpected:jpl.every((v,j)=>v.sign===expected[j]),engineBothExpected:local.every((v,j)=>v.sign===expected[j])};
        }
        rows.push({body:name,seedYear:event.seedYear,kind:event.kind,jdTdb:event.jdTdb,expected,windows});
      }
    }
    sources.push({name,command,requests});
  }
  if(await fileSha(kernel)!==kernelSha||sha(fs.readFileSync(report))!==reportSha)throw new Error('input changed');
  const summarize=items=>Object.fromEntries(radii.map(r=>[r,{cases:items.length,jplBothExpected:items.filter(x=>x.windows[r].jplBothExpected).length,
    jplAmbiguous:items.filter(x=>x.windows[r].jpl.some(v=>v.sign===0)).length,jplUnexpected:items.filter(x=>x.windows[r].jpl.some((v,i)=>v.sign!==0&&v.sign!==x.expected[i])).length,
    engineBothExpected:items.filter(x=>x.windows[r].engineBothExpected).length}]));
  const summary={cases:rows.length,halfStepTdbDays:h,assumedRoundingBoundArcsecDay:bound,windows:summarize(rows),
    byBody:Object.fromEntries(bodies.map(([, ,name])=>[name,summarize(rows.filter(r=>r.body===name))]))};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-station-windows/1',status:'EXPLORATORY',kernelSha256:kernelSha,referenceSha256:reportSha,
    notes:'Sampled signs in the JPL orientation at selected endpoints, not certified station intervals. Rounding allowance assumes nearest rounding; finite-step, time-scale and physical model errors are separate.',summary,rows,sources},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
  if(rows.length!==64||radii.some(r=>summary.windows[r].jplUnexpected||summary.windows[r].engineBothExpected!==rows.length))process.exitCode=1;
}
module.exports={stencilOffsets};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
