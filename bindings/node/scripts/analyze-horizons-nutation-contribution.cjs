'use strict';
// SPDX-License-Identifier: CPAL-1.0. Same-direction orientation sensitivity.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
const {parse}=require('./compare-horizons-orientation-components.cjs');
const {unit,separation}=require('./compare-horizons-frame-isolation.cjs');
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const rad=Math.PI/180,wrap=x=>((x%360)+540)%360-180;
async function main(){
  const [source,sourceHash,executable,out,...extra]=process.argv.slice(2);
  if(!source||!/^[a-f0-9]{64}$/.test(sourceHash||'')||!executable||!out||extra.length||fs.existsSync(out)||new Set([source,executable,out].map(x=>path.resolve(x))).size!==3)throw new Error('usage: <orientation-report> <sha256> <projection-executable> <new-report>');
  if(await sha(source)!==sourceHash)throw new Error('input hash mismatch');
  const prior=JSON.parse(fs.readFileSync(source));
  if(prior.schema!=='horizons-orientation-components/1'||!Array.isArray(prior.rows)||prior.rows.length!==21||!Array.isArray(prior.responses)||prior.responses.length!==3)throw new Error('invalid source');
  const bodies=[['Sun','10'],['Moon','301'],['Venus','299']],groups=bodies.map(([name,id])=>{
    const response=prior.responses.find(r=>r.name===name&&r.id===id),rows=prior.rows.filter(r=>r.body===name);
    if(!response||rows.length!==7||!rows.every(r=>Number.isFinite(r.jdTt)&&Number.isFinite(r.year)))throw new Error('invalid source group');
    const tt=rows.map(r=>r.jdTt),ref=parse(response.raw,name,id,tt);
    return {name,rows,ref};
  });
  const input=groups.flatMap(g=>g.ref.map((r,i)=>`${g.rows[i].jdTt},${r.raDeg},${r.decDeg}`)).join('\n')+'\n';
  const lines=execFileSync(path.resolve(executable),[],{input,encoding:'utf8',maxBuffer:1048576,timeout:30000}).trim().split(/\r?\n/);
  if(lines.length!==21)throw new Error('wrong projection count');
  const rows=groups.flatMap((g,b)=>g.ref.map((ref,i)=>{
    const f=lines[b*7+i].split(',');if(f.length!==10||!f.every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid projection');
    const [meanRa,meanDec,trueRa,trueDec]=f.map(Number),horizons=unit(ref.ra,ref.dec),mean=unit(meanRa,meanDec),actual=unit(trueRa,trueDec);
    return {body:g.name,year:g.rows[i].year,jdTt:g.rows[i].jdTt,meanMinusHorizonsRaCosDecArcsec:wrap(meanRa-ref.ra)*Math.cos(ref.dec*rad)*3600,
      trueMinusHorizonsRaCosDecArcsec:wrap(trueRa-ref.ra)*Math.cos(ref.dec*rad)*3600,
      meanMinusHorizonsSeparationArcsec:separation(mean,horizons),trueMinusHorizonsSeparationArcsec:separation(actual,horizons),localNutationShiftArcsec:separation(mean,actual)};
  }));
  if(await sha(source)!==sourceHash)throw new Error('input changed');
  fs.writeFileSync(out,JSON.stringify({schema:'horizons-local-nutation-contribution/1',status:'EXPLORATORY',sourceSha256:sourceHash,notes:'Same Horizons ICRF directions projected to local mean and true equators; Horizons reference is true of date. Mean-minus-Horizons includes Horizons nutation and frame differences, not pure precession error.',rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(rows));
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
