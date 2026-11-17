'use strict';
// SPDX-License-Identifier: CPAL-1.0. Exploratory rate-contribution comparison.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{spawnSync}=require('node:child_process');
const {icrfAngularRates}=require('./icrf-angular-rates.cjs');
const {julday}=require('../index.js');
const {validationYears}=require('./planet-validation-years.cjs');
const labels=['Sun','Moon','Mercury','Venus','Mars Barycenter','Jupiter Barycenter','Saturn Barycenter','Uranus Barycenter','Neptune Barycenter','Pluto Barycenter'];
const variants=['full','base','no_earth_acceleration','no_retarded_rate','neither','untransformed','untransformed_no_retard'];
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,kernelSha,reference,referenceSha,executable,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!reference||!/^[a-f0-9]{64}$/.test(referenceSha||'')||!executable||!out||extra.length
    ||new Set([kernel,reference,executable,out].map(p=>path.resolve(p))).size!==4||fs.existsSync(out))
    throw new Error('usage: <kernel> <kernel-sha> <reference-json> <reference-sha> <diagnostic-executable> <new-report>');
  const bytes=fs.readFileSync(reference);if(bytes.length>10e6||hash(bytes)!==referenceSha||await sha(kernel)!==kernelSha)throw new Error('source mismatch');
  const data=JSON.parse(bytes);
  if(data.schema!=='virgostars-horizons-icrf-rates/1'||data.kernelSha256!==kernelSha||!Array.isArray(data.records)||![610,3010].includes(data.records.length))
    throw new Error('invalid reference report');
  const years=validationYears(data.records.length===610?'0-3000-step-50':'0-3000-step-10');
  const seen=new Set(),input=data.records.map(r=>{
    const id=labels.indexOf(r.body),key=`${id}:${r.jdTdb}`;
    if(id<0||!years.includes(r.year)||r.jdTdb!==julday(r.year,7,1,12,true)||seen.has(key)||!r.reference||![r.reference.raCosDecArcsecHour,r.reference.decArcsecHour].every(Number.isFinite))
      throw new Error('invalid reference case');
    seen.add(key);return `${id},${r.jdTdb}`;
  }).join('\n')+'\n';
  const binarySha=await sha(executable),result=spawnSync(path.resolve(executable),[path.resolve(kernel)],{input,encoding:'utf8',maxBuffer:8e6});
  if(result.error||result.status!==0)throw new Error(`diagnostic executable failed: ${result.error?.message||result.stderr}`);
  const lines=result.stdout.trim().split(/\r?\n/);
  if(lines.length!==data.records.length*variants.length)throw new Error('unexpected output length');
  const output=[],makeAcc=()=>Object.fromEntries(variants.map(v=>[v,{count:0,sumSquared:0,maxAbs:0,byBody:{}}]));
  const acc=makeAcc(),holdout=makeAcc();
  for(let i=0;i<data.records.length;i++)for(let j=0;j<variants.length;j++){
    const f=lines[i*variants.length+j].split(','),r=data.records[i],id=labels.indexOf(r.body);
    if(f.length!==9||Number(f[0])!==id||Number(f[1])!==r.jdTdb||f[2]!==variants[j])throw new Error('diagnostic output mismatch');
    const values=f.slice(3).map(Number);if(!values.every(Number.isFinite))throw new Error('nonfinite diagnostic state');
    const actual=icrfAngularRates(values.slice(0,3),values.slice(3)),
      ra=actual.raCosDecArcsecHour-r.reference.raCosDecArcsecHour,dec=actual.decArcsecHour-r.reference.decArcsecHour,
      magnitude=Math.hypot(ra,dec),a=acc[variants[j]];
    a.count++;a.sumSquared+=magnitude*magnitude;a.maxAbs=Math.max(a.maxAbs,magnitude);
    const group=a.byBody[r.body]??{count:0,sumSquared:0,maxAbs:0};group.count++;group.sumSquared+=magnitude*magnitude;
    group.maxAbs=Math.max(group.maxAbs,magnitude);a.byBody[r.body]=group;
    if(r.year%50!==0){const h=holdout[variants[j]],g=h.byBody[r.body]??{count:0,sumSquared:0,maxAbs:0};
      h.count++;h.sumSquared+=magnitude*magnitude;h.maxAbs=Math.max(h.maxAbs,magnitude);
      g.count++;g.sumSquared+=magnitude*magnitude;g.maxAbs=Math.max(g.maxAbs,magnitude);h.byBody[r.body]=g;}
    output.push({body:r.body,year:r.year,jdTdb:r.jdTdb,variant:variants[j],raResidualArcsecHour:ra,decResidualArcsecHour:dec});
  }
  const finish=x=>({count:x.count,rmsArcsecHour:Math.sqrt(x.sumSquared/x.count),maxArcsecHour:x.maxAbs});
  const summary=Object.fromEntries(variants.map(v=>[v,{...finish(acc[v]),byBody:Object.fromEntries(Object.entries(acc[v].byBody).map(([b,x])=>[b,finish(x)]))}]));
  const holdoutSummary=data.records.length===3010?Object.fromEntries(variants.map(v=>[v,{...finish(holdout[v]),byBody:Object.fromEntries(Object.entries(holdout[v].byBody).map(([b,x])=>[b,finish(x)]))}])):null;
  if(await sha(kernel)!==kernelSha||hash(fs.readFileSync(reference))!==referenceSha||await sha(executable)!==binarySha)throw new Error('input changed');
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-rate-components-diagnostic/1',status:'HYPOTHESIS_ONLY',kernelSha256:kernelSha,referenceSha256:referenceSha,binarySha256:binarySha,
    notes:'Incomplete variants are intentional experiments, never physical results or proof of Horizons internals. Differences are relative to printed JPL rates. The holdout excludes all years divisible by 50, used to select the hypothesis.',summary,holdoutSummary,output},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({summary,holdoutSummary}));
}
main().catch(e=>{console.error(e.message);process.exitCode=1});
