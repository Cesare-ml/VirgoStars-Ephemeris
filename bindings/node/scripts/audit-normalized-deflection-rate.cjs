'use strict';
// SPDX-License-Identifier: CPAL-1.0. Offline consistency diagnostic only.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,kernelHash,source,sourceHash,executable,out,...extra]=process.argv.slice(2);
  if(!kernel||!source||!executable||!out||extra.length||![kernelHash,sourceHash].every(x=>/^[a-f0-9]{64}$/.test(x||''))||fs.existsSync(out)||new Set([kernel,source,executable,out].map(x=>path.resolve(x))).size!==4)throw new Error('usage: <de441> <sha256> <source-report> <source-sha256> <executable> <new-report>');
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input hash mismatch');
  const previous=JSON.parse(fs.readFileSync(source));
  if(previous.schema!=='solar-deflection-rate-candidates/1'||previous.kernelSha256!==kernelHash||!Array.isArray(previous.rows)||!previous.rows.length||!previous.rows.every(r=>Number.isInteger(r.id)&&r.id>=2&&r.id<=9&&Number.isFinite(r.jd)))throw new Error('invalid source report');
  const input=previous.rows.map(r=>`${r.id},${r.jd}`).join('\n')+'\n';
  const text=execFileSync(path.resolve(executable),[path.resolve(kernel),'--normalized'],{input,encoding:'utf8',maxBuffer:1048576,timeout:120000});
  const lines=text.trim().split(/\r?\n/),expected='body,jd_tdb,elongation_deg,denominator,crosses_floor,old_error_as_per_day,analytic_error_as_per_day,reference_disagreement_as_per_day';
  if(lines.shift()!==expected||lines.length!==previous.rows.length)throw new Error('invalid diagnostic output');
  const rows=lines.map((line,i)=>{const f=line.split(',');if(f.length!==8||Number(f[0])!==previous.rows[i].id||Number(f[1])!==previous.rows[i].jd||!['true','false'].includes(f[4])||![f[2],f[3],f[5],f[6],f[7]].every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error(`invalid row ${i}`);
    return {id:Number(f[0]),jd:Number(f[1]),elongationDeg:Number(f[2]),denominator:Number(f[3]),crossesFloor:f[4]==='true',oldResidual:Number(f[5]),analyticResidual:Number(f[6]),referenceDisagreement:Number(f[7])};});
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input changed');
  fs.writeFileSync(out,JSON.stringify({schema:'normalized-deflection-rate/1',status:'EXPLORATORY',kernelSha256:kernelHash,sourceSha256:sourceHash,notes:'Angular ICRF rate difference for normalized deflection correction vs numerical differentiation of same model; no physical accuracy guarantee.',rows},null,2)+'\n',{flag:'wx'});
  const valid=rows.filter(r=>!r.crossesFloor);const max=k=>Math.max(...valid.map(r=>r[k]));
  console.log(JSON.stringify({cases:rows.length,distinct:new Set(rows.map(r=>`${r.id}:${r.jd}`)).size,crossing:rows.length-valid.length,oldMax:max('oldResidual'),analyticMax:max('analyticResidual'),referenceMax:max('referenceDisagreement'),improved:valid.filter(r=>r.analyticResidual<r.oldResidual).length}));
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
