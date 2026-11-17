'use strict';
// SPDX-License-Identifier: CPAL-1.0. Occulted-case numerical diagnostic.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,hash,executable,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(hash||'')||!executable||!out||extra.length||fs.existsSync(out)||new Set([kernel,executable,out].map(x=>path.resolve(x))).size!==3)throw new Error('usage: <de441> <sha256> <diagnostic-executable> <new-report>');
  if(await sha(kernel)!==hash)throw new Error('kernel hash mismatch');
  const epochs=Array.from({length:257},(_,i)=>-1474169.125+i/2048),input=epochs.map(jd=>`7,${jd}`).join('\n')+'\n';
  const text=execFileSync(path.resolve(executable),[path.resolve(kernel),'--normalized'],{input,encoding:'utf8',maxBuffer:1048576,timeout:120000});
  const lines=text.trim().split(/\r?\n/),header='body,jd_tdb,elongation_deg,denominator,crosses_floor,old_error_as_per_day,analytic_error_as_per_day,reference_disagreement_as_per_day';
  if(lines.shift()!==header)throw new Error('invalid header');
  const seen=new Set(),requested=new Set(epochs);
  const rows=lines.map(line=>{const f=line.split(','),jd=Number(f[1]);if(f.length!==8||Number(f[0])!==7||!requested.has(jd)||seen.has(jd)||!['true','false'].includes(f[4])||![f[2],f[3],f[5],f[6],f[7]].every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid row');seen.add(jd);return {jdTdb:jd,elongationDeg:Number(f[2]),denominator:Number(f[3]),crossesFloor:f[4]==='true',oldResidual:Number(f[5]),analyticResidual:Number(f[6]),referenceDisagreement:Number(f[7])};});
  const missing=epochs.filter(x=>!seen.has(x));
  if(await sha(kernel)!==hash)throw new Error('kernel changed');
  fs.writeFileSync(out,JSON.stringify({schema:'deflection-floor-sweep/1',status:'EXPLORATORY',kernelSha256:hash,notes:'Uranus, occulted geometry. Missing epochs have no candidate (e.g. nondifferentiable guard tolerance or non-finite derivative); finite differences across the floor do not approximate one smooth derivative.',missing,rows},null,2)+'\n',{flag:'wx'});
  const crossing=rows.filter(x=>x.crossesFloor);
  console.log(JSON.stringify({requested:epochs.length,returned:rows.length,missing:missing.length,crossing:crossing.length,first:crossing[0],last:crossing.at(-1)}));
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
