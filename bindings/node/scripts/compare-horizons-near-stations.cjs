'use strict';
// SPDX-License-Identifier: CPAL-1.0. Explicit online position-derivative comparison at selected stations.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{spawnSync}=require('node:child_process');
const {Engine}=require('../index.js'),{parseIcrfTable}=require('./horizons-icrf-parser.cjs'),{icrfAngles}=require('./icrf-angular-rates.cjs');
const {extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
async function fileSha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
const bodies=[[2,'199','Mercury'],[3,'299','Venus'],[4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],[7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter'],[9,'9','Pluto Barycenter']];
const wrap=x=>((x%360)+540)%360-180;
async function main(){
  const [kernel,kernelSha,matrix,matrixSha,executable,out,step,...extra]=process.argv.slice(2),h=step===undefined?0.02:Number(step);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!matrix||!/^[a-f0-9]{64}$/.test(matrixSha||'')||!executable||!out||extra.length||!Number.isFinite(h)||h<=0||h>0.05||fs.existsSync(out)||new Set([kernel,matrix,executable,out].map(p=>path.resolve(p))).size!==4||await fileSha(kernel)!==kernelSha)throw new Error('usage: <de441> <kernel-sha> <matrix> <matrix-sha> <projection-executable> <new-report> [half-step-days]');
  const bytes=fs.readFileSync(matrix);if(sha(bytes)!==matrixSha)throw new Error('matrix hash mismatch');const data=JSON.parse(bytes);
  if(data.schema!=='virgostars-swiss-station-matrix/1'||data.kernelSha256!==kernelSha||data.rows.length!==944)throw new Error('invalid matrix');
  const engine=new Engine(path.resolve(kernel)),binarySha=await fileSha(executable),cases=[],requests=[];
  for(const [id,command,name] of bodies){
    const matched=data.rows.filter(r=>r.year===1500&&r.body===id&&r.kind==='retrograde'&&r.status==='ok');
    if(matched.length!==1)throw new Error('missing selected event');
    const station=matched[0],center=(station.engineBracket.lo+station.engineBracket.hi)/2,
      tdb=[center-2*h,center-h,center+h,center+2*h],epochs=tdb.map(tt);
    const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",
      TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'45'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
    const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});
    if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
    const raw=(await response.json()).result,reference=parseIcrfTable(raw,name,command,epochs);
    cases.push({id,name,center,h,tdb,epochs,reference});requests.push({url,raw});
  }
  const input=[];
  for(const c of cases)for(let i=0;i<4;i++){
    const a=c.reference[i],own=icrfAngles(engine.experimentalApparentIcrfTdb(c.tdb[i],c.id).positionKm);
    input.push(`${c.epochs[i]},${a.raDeg},${a.decDeg}`,`${c.epochs[i]},${own.raDeg},${own.decDeg}`);
  }
  const result=spawnSync(path.resolve(executable),[],{input:input.join('\n')+'\n',encoding:'utf8'});
  if(result.error||result.status!==0)throw new Error(`projection executable failed: ${result.error?.message||result.stderr}`);
  const lines=result.stdout.trim().split(/\r?\n/);if(lines.length!==input.length)throw new Error('projection row count mismatch');
  const parsed=lines.map(line=>{const f=line.split(',');if(f.length!==2||!f.every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid projection');return f.map(Number);});
  const rows=[];
  for(let n=0;n<cases.length;n++){
    const c=cases[n],jpl=[],own=[];
    for(let i=0;i<4;i++){
      jpl.push(parsed[n*8+2*i][0]);own.push(parsed[n*8+2*i+1][0]);
      const actual=engine.experimentalLongTermApparentTdb(c.tdb[i],c.id).xx[0];
      if(Math.abs(wrap(own[i]-actual))>5e-8)throw new Error('projection differs from engine orientation');
    }
    const rate=extrapolatedRate(...jpl,c.center,c.h,true),local=engine.experimentalLongTermApparentTdb(c.center,c.id).xx[3];
    rows.push({body:c.name,jdTdb:c.center,halfStepTdbDays:c.h,engineLongitudeRateArcsecDay:local*3600,horizonsPositionDerivativeArcsecDay:rate*3600,residualArcsecDay:(local-rate)*3600});
  }
  if(await fileSha(kernel)!==kernelSha||sha(fs.readFileSync(matrix))!==matrixSha||await fileSha(executable)!==binarySha)throw new Error('input changed');
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-station-position-derivatives/1',status:'EXPLORATORY',kernelSha256:kernelSha,matrixSha256:matrixSha,binarySha256:binarySha,
    notes:'Horizons printed ICRF apparent directions projected using the engine experimental true ecliptic-of-date orientation. Shared DE441 and shared orientation; no independent precession/nutation validation.',rows,requests},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(rows));
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
