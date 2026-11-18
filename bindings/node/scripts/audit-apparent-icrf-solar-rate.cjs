'use strict';
// SPDX-License-Identifier: CPAL-1.0. Same-model apparent ICRF rate diagnostic.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,kernelHash,source,sourceHash,executable,out,mode='full',...extra]=process.argv.slice(2);
  if(!kernel||!source||!executable||!out||extra.length||!['full','increment'].includes(mode)||![kernelHash,sourceHash].every(x=>/^[a-f0-9]{64}$/.test(x||''))||fs.existsSync(out)||new Set([kernel,source,executable,out].map(x=>path.resolve(x))).size!==4)throw new Error('usage: <de441> <sha256> <source-report> <source-sha256> <executable> <new-report> [full|increment]');
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input hash mismatch');
  const sourceData=JSON.parse(fs.readFileSync(source));
  if(sourceData.schema!=='solar-deflection-rate-candidates/1'||sourceData.kernelSha256!==kernelHash||!Array.isArray(sourceData.rows)||!sourceData.rows.length||!sourceData.rows.every(r=>Number.isInteger(r.id)&&r.id>=2&&r.id<=9&&Number.isFinite(r.jd)))throw new Error('invalid source');
  const input=sourceData.rows.map(r=>`${r.id},${r.jd}`).join('\n')+'\n';
  const lines=execFileSync(path.resolve(executable),[path.resolve(kernel),mode==='full'?'--apparent-icrf':'--apparent-increment'],{input,encoding:'utf8',maxBuffer:1048576,timeout:120000}).trim().split(/\r?\n/);
  const header='body,jd_tdb,elongation_deg,denominator,crosses_floor,old_error_as_per_day,analytic_error_as_per_day,reference_disagreement_as_per_day';
  if(lines.shift()!==header||lines.length!==sourceData.rows.length)throw new Error('invalid output');
  const rows=lines.map((line,i)=>{const f=line.split(',');if(f.length!==8||Number(f[0])!==sourceData.rows[i].id||Number(f[1])!==sourceData.rows[i].jd||!['true','false'].includes(f[4])||![f[2],f[3],f[5],f[6],f[7]].every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid row');
    return {id:Number(f[0]),jd:Number(f[1]),elongationDeg:Number(f[2]),denominator:Number(f[3]),crossesFloor:f[4]==='true',oldResidual:Number(f[5]),analyticResidual:Number(f[6]),referenceDisagreement:Number(f[7])};});
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input changed');
  const valid=rows.filter(r=>!r.crossesFloor),max=k=>Math.max(...valid.map(r=>r[k]));
  const summary={cases:rows.length,distinct:new Set(rows.map(r=>`${r.id}:${r.jd}`)).size,crossing:rows.length-valid.length,oldMax:max('oldResidual'),analyticMax:max('analyticResidual'),referenceMax:max('referenceDisagreement'),improved:valid.filter(r=>r.analyticResidual<r.oldResidual).length};
  fs.writeFileSync(out,JSON.stringify({schema:'apparent-icrf-solar-rate/1',mode,status:'EXPLORATORY',kernelSha256:kernelHash,sourceSha256:sourceHash,notes:'Light-time, solar deflection and aberration; full ICRF rate or its deflection increment vs fourth-order derivatives of the same pipeline; single-JD rounding and floor crossings limit reference. No physical accuracy guarantee.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
