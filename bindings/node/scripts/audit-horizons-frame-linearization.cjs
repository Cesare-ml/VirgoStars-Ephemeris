'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Offline audit of the earlier single-linearization residuals.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
const {julday}=require('../index.js');
const {parse}=require('./compare-horizons-orientation-components.cjs');
const {unit}=require('./compare-horizons-frame-isolation.cjs');
const {fitSmallRotation}=require('./analyze-horizons-frame-rotation.cjs');
const {fitRotation}=require('./sample-horizons-frame-epochs.cjs');
const bodies=[['Sun','10'],['Moon','301'],['Venus','299'],['Mars Barycenter','4'],['Jupiter Barycenter','5'],['Saturn Barycenter','6']];
function main() {
  const [source,expected,exe,out,...extra]=process.argv.slice(2);
  if(!source||!/^[a-f0-9]{64}$/.test(expected||'')||!exe||!out||extra.length||fs.existsSync(out)||new Set([source,exe,out].map(p=>path.resolve(p))).size!==3)throw new Error('usage: <prior-report> <sha256> <projection-executable> <new-report>');
  const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
  if(sha(source)!==expected)throw new Error('source hash mismatch');
  const exeHash=sha(exe),prior=JSON.parse(fs.readFileSync(source,'utf8'));
  if(prior.schema!=='horizons-frame-stability/1'||!Array.isArray(prior.rows)||prior.rows.length!==26||!Array.isArray(prior.responses)||prior.responses.length!==6)throw new Error('invalid prior report');
  const epochs=prior.rows.map((r,i)=>{
    const year=i<13?-9998:2000,sample=i%13,tt=julday(year,7,1,12,true)+30*sample;
    if(r.year!==year||r.sample!==sample||r.jdTt!==tt)throw new Error('invalid prior grid');
    return tt;
  });
  const groups=bodies.map(([name,id])=>{const r=prior.responses.find(x=>x.name===name&&x.id===id);if(!r||typeof r.raw!=='string')throw new Error(`missing ${name}`);return {name,coordinates:parse(r.raw,name,id,epochs)};});
  const input=groups.flatMap(r=>r.coordinates.map((c,i)=>`${epochs[i]},${c.raDeg},${c.decDeg}`)).join('\n')+'\n';
  const lines=execFileSync(path.resolve(exe),[],{input,encoding:'utf8',maxBuffer:1048576,timeout:30000}).trim().split(/\r?\n/);
  if(lines.length!==epochs.length*bodies.length)throw new Error('wrong projection count');
  const projected=lines.map(line=>{const f=line.split(',');if(f.length!==10||!f.every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid projection');return f.map(Number);});
  const rows=epochs.map((jdTt,i)=>{
    const ref=groups.map(g=>unit(g.coordinates[i].ra,g.coordinates[i].dec));
    const local=groups.map((_,b)=>unit(projected[b*epochs.length+i][2],projected[b*epochs.length+i][3]));
    const onePass=fitSmallRotation(local,ref),iterated=fitRotation(local,ref),record=prior.rows[i].trueFit;
    if(!record||!Number.isFinite(record.rmsResidualArcsec)||Math.abs(record.rmsResidualArcsec-onePass.rmsResidualArcsec)>1e-9)throw new Error('prior fit did not reproduce');
    return {year:prior.rows[i].year,sample:prior.rows[i].sample,jdTt,onePass,iterated};
  });
  const summary=[-9998,2000].map(year=>{const group=rows.filter(r=>r.year===year);return {year,
    maxOnePassRmsArcsec:Math.max(...group.map(r=>r.onePass.rmsResidualArcsec)),
    maxIteratedRmsArcsec:Math.max(...group.map(r=>r.iterated.rmsResidualArcsec)),
    maxRotationChangeArcsec:Math.max(...group.map(r=>Math.hypot(...r.iterated.rotationArcsec.map((x,j)=>x-r.onePass.rotationArcsec[j])))),
    minPivot:Math.min(...group.map(r=>r.iterated.minPivot))};});
  if(sha(source)!==expected||sha(exe)!==exeHash)throw new Error('input changed');
  fs.writeFileSync(out,JSON.stringify({schema:'horizons-frame-linearization-audit/1',status:'EXPLORATORY',sourceSha256:expected,executableSha256:exeHash,notes:'Same six directions and Horizons responses; numerical estimator audit, not independent astronomical accuracy.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
if(require.main===module)main();
