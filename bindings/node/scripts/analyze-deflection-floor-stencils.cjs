'use strict';
// SPDX-License-Identifier: CPAL-1.0. Exploratory sample classification.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,kernelHash,source,sourceHash,executable,out,...extra]=process.argv.slice(2);
  if(!kernel||!source||!executable||!out||extra.length||![kernelHash,sourceHash].every(x=>/^[a-f0-9]{64}$/.test(x||''))||fs.existsSync(out)||new Set([kernel,source,executable,out].map(x=>path.resolve(x))).size!==4)throw new Error('usage: <de441> <sha256> <floor-report> <sha256> <executable> <new-report>');
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input hash mismatch');
  const report=JSON.parse(fs.readFileSync(source));
  if(report.schema!=='deflection-floor-sweep/1'||report.kernelSha256!==kernelHash||!Array.isArray(report.rows)||!report.rows.length||!report.rows.every(r=>Number.isFinite(r.jdTdb)))throw new Error('invalid source');
  const input=report.rows.map(r=>`7,${r.jdTdb}`).join('\n')+'\n';
  const lines=execFileSync(path.resolve(executable),[path.resolve(kernel),'--floor-details'],{input,encoding:'utf8',maxBuffer:1048576,timeout:120000}).trim().split(/\r?\n/);
  const header='body,jd_tdb,denominator,old_crosses,fine_crosses,finer_crosses,old_error_as_per_day,analytic_error_as_per_day,reference_disagreement_as_per_day';
  if(lines.shift()!==header||lines.length!==report.rows.length)throw new Error('invalid output');
  const rows=lines.map((line,i)=>{const f=line.split(',');if(f.length!==9||Number(f[0])!==7||Number(f[1])!==report.rows[i].jdTdb||![f[3],f[4],f[5]].every(x=>['true','false'].includes(x))||![f[2],f[6],f[7],f[8]].every(x=>x!==''&&Number.isFinite(Number(x))))throw new Error('invalid row');return {jdTdb:Number(f[1]),denominator:Number(f[2]),oldCross:f[3]==='true',fineCross:f[4]==='true',finerCross:f[5]==='true',oldResidual:Number(f[6]),analyticResidual:Number(f[7]),referenceDisagreement:Number(f[8])};});
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input changed');
  const oldOnly=rows.filter(r=>r.oldCross&&!r.fineCross&&!r.finerCross),fineCross=rows.filter(r=>r.fineCross||r.finerCross);
  const max=(items,key)=>items.length?Math.max(...items.map(r=>r[key])):null;
  const summary={rows:rows.length,oldOnly:oldOnly.length,oldOnlyBelow:oldOnly.filter(r=>r.denominator<1e-6).length,oldOnlyAbove:oldOnly.filter(r=>r.denominator>1e-6).length,oldOnlyOldMax:max(oldOnly,'oldResidual'),oldOnlyAnalyticMax:max(oldOnly,'analyticResidual'),oldOnlyReferenceMax:max(oldOnly,'referenceDisagreement'),referenceCross:fineCross.length,referenceDisagreementMax:max(fineCross,'referenceDisagreement')};
  fs.writeFileSync(out,JSON.stringify({schema:'deflection-floor-stencil-classification/1',status:'EXPLORATORY',kernelSha256:kernelHash,sourceSha256:sourceHash,notes:'Crossing labels compare sampled denominator signs, not continuous interval guarantees. Reference stencils must lie on one smooth side to compare a derivative.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
