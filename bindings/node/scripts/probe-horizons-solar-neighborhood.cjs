'use strict';
// SPDX-License-Identifier: CPAL-1.0. New epochs near selected large residuals.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{spawnSync}=require('node:child_process');
const {julday}=require('../index.js'),{parseIcrfRatesTable}=require('./horizons-icrf-parser.cjs'),{icrfAngularRates}=require('./icrf-angular-rates.cjs');
const targets=[[3,'299','Venus',[2430]],[6,'6','Saturn Barycenter',[120,2180]],[7,'7','Uranus Barycenter',[610]],[8,'8','Neptune Barycenter',[2070]]];
const variants=['full','base','no_earth_acceleration','no_retarded_rate','neither','untransformed','untransformed_no_retard','untransformed_no_retard_no_deflection_rate'];
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function main(){
  const [kernel,kernelSha,executable,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!executable||!out||extra.length||fs.existsSync(out)||new Set([kernel,executable,out].map(p=>path.resolve(p))).size!==3||await sha(kernel)!==kernelSha)throw new Error('usage: <kernel> <sha256> <diagnostic-executable> <new-report>');
  const binarySha=await sha(executable),cases=[],requests=[];
  for(const [id,command,name,years] of targets){
    const epochs=years.flatMap(year=>[-1,1].map(offset=>({year,offset,jd:julday(year,7,1,12,true)+offset})));
    const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",
      TLIST:`'${epochs.map(x=>tt(x.jd).toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'45,46'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
    const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});
    if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
    const raw=(await response.json()).result,ref=parseIcrfRatesTable(raw,name,command,epochs.map(x=>tt(x.jd)));
    requests.push({url,raw});for(let i=0;i<epochs.length;i++)cases.push({id,name,...epochs[i],reference:ref[i]});
  }
  const input=cases.map(x=>`${x.id},${x.jd}`).join('\n')+'\n',result=spawnSync(path.resolve(executable),[path.resolve(kernel)],{input,encoding:'utf8'});
  if(result.error||result.status!==0)throw new Error('diagnostic executable failed');
  const lines=result.stdout.trim().split(/\r?\n/);if(lines.length!==cases.length*variants.length)throw new Error('unexpected output length');
  const rows=[];
  for(let i=0;i<cases.length;i++)for(let j=0;j<variants.length;j++){
    const x=cases[i],f=lines[i*variants.length+j].split(',');
    if(f.length!==9||Number(f[0])!==x.id||Number(f[1])!==x.jd||f[2]!==variants[j])throw new Error('diagnostic output mismatch');
    const state=f.slice(3).map(Number);if(!state.every(Number.isFinite))throw new Error('invalid state');
    const rate=icrfAngularRates(state.slice(0,3),state.slice(3)),residual=Math.hypot(rate.raCosDecArcsecHour-x.reference.raCosDecArcsecHour,rate.decArcsecHour-x.reference.decArcsecHour);
    rows.push({body:x.name,year:x.year,dayOffset:x.offset,variant:variants[j],residualArcsecHour:residual});
  }
  const summary=Object.fromEntries(variants.map(v=>{const x=rows.filter(r=>r.variant===v);return [v,{count:x.length,rmsArcsecHour:Math.sqrt(x.reduce((s,r)=>s+r.residualArcsecHour**2,0)/x.length),maxArcsecHour:Math.max(...x.map(r=>r.residualArcsecHour))}]}));
  if(await sha(kernel)!==kernelSha||await sha(executable)!==binarySha)throw new Error('input changed');
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-solar-neighborhood/1',status:'EXPLORATORY',kernelSha256:kernelSha,binarySha256:binarySha,
    notes:'New epochs are one day before/after selected prior outliers; they are not a random or independent ephemeris sample. Variants are not physical states.',summary,rows,requests},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
