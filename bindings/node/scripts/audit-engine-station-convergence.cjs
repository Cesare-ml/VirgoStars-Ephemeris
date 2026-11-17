'use strict';
// SPDX-License-Identifier: CPAL-1.0. Experimental engine self-consistency, not an event API.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine}=require('../index.js');
const {bisect}=require('./compare-swiss-station-roots.cjs');
const {extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs');
const {stats}=require('./compare-swiss-station-matrix.cjs');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
async function fileSha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
function signChanges(values){
  if(!Array.isArray(values)||values.length<2||!values.every(Number.isFinite))throw new Error('invalid speed sequence');
  let previous=0,count=0;for(const v of values){const s=Math.sign(v);if(!s)continue;if(previous&&s!==previous)count++;previous=s;}return count;
}
async function main(){
  const [kernel,kernelSha,matrix,matrixSha,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!matrix||!/^[a-f0-9]{64}$/.test(matrixSha||'')||!out||extra.length||fs.existsSync(out)||new Set([kernel,matrix,out].map(p=>path.resolve(p))).size!==3||await fileSha(kernel)!==kernelSha)throw new Error('usage: <de441> <kernel-sha> <station-matrix> <matrix-sha> <new-report>');
  const bytes=fs.readFileSync(matrix);if(sha(bytes)!==matrixSha)throw new Error('matrix hash mismatch');const data=JSON.parse(bytes);
  if(data.schema!=='virgostars-swiss-station-matrix/1'||data.kernelSha256!==kernelSha||!Array.isArray(data.rows)||data.rows.length!==944)throw new Error('invalid matrix');
  const selected=data.rows.filter(r=>[100,500,1500,2500,2900].includes(r.year)&&r.body>=2&&r.body<=9&&r.status==='ok');
  if(selected.length!==80||new Set(selected.map(r=>`${r.year}:${r.body}:${r.kind}`)).size!==80)throw new Error('invalid selected events');
  const engine=new Engine(path.resolve(kernel)),tol=1e-7,hValues=[0.005,0.01,0.02,0.04];
  const state=(jd,body)=>{const x=engine.experimentalLongTermApparentTdb(jd,body).xx;
    if(x.length!==6||!x.every(Number.isFinite))throw new Error('invalid engine state');return x;};
  const speed=(jd,body)=>state(jd,body)[3],finite=(jd,body,h)=>{
    const lon=t=>state(t,body)[0];return extrapolatedRate(lon(jd-2*h),lon(jd-h),lon(jd+h),lon(jd+2*h),jd,h,true);};
  const offsets=[[-0.031,0.029],[-0.0327,0.0273],[-0.0279,0.0321]],rows=[];
  function roots(fn,center){return offsets.map(([lo,hi])=>bisect(fn,center+lo,center+hi,tol));}
  const midpoint=b=>(b.lo+b.hi)/2,span=roots=>{const a=roots.map(midpoint);return (Math.max(...a)-Math.min(...a))*86400;};
  for(const item of selected){
    const anchor=(item.engineBracket.lo+item.engineBracket.hi)/2;
    try{
      const direct=roots(jd=>speed(jd,item.body),anchor),directCenter=midpoint(direct[0]),steps={};
      for(const h of hValues){const r=roots(jd=>finite(jd,item.body,h),anchor);
        steps[h]={brackets:r,offsetFromDirectSeconds:(midpoint(r[0])-directCenter)*86400,bracketSpanSeconds:span(r),
          rateResidualArcsecDay:(speed(directCenter,item.body)-finite(directCenter,item.body,h))*3600};}
      const directSeries=[],finiteSeries=[];
      for(let k=-20;k<=20;k++){const jd=directCenter+k*0.1/86400;directSeries.push(speed(jd,item.body));finiteSeries.push(finite(jd,item.body,0.02));}
      rows.push({year:item.year,body:item.body,kind:item.kind,status:'ok',directBrackets:direct,directSpanSeconds:span(direct),steps,
        signChanges:{direct:signChanges(directSeries),positionDerivative:signChanges(finiteSeries)}});
    }catch(e){rows.push({year:item.year,body:item.body,kind:item.kind,status:'unresolved',reason:String(e.message)});}
  }
  if(await fileSha(kernel)!==kernelSha||sha(fs.readFileSync(matrix))!==matrixSha)throw new Error('input changed');
  const ok=rows.filter(r=>r.status==='ok'),summary={expected:80,ok:ok.length,unresolved:rows.length-ok.length,
    directBracketSpanSeconds:stats(ok.map(r=>r.directSpanSeconds)),
    steps:Object.fromEntries(hValues.map(h=>[h,{rootOffsetSeconds:stats(ok.map(r=>r.steps[h].offsetFromDirectSeconds)),
      bracketSpanSeconds:stats(ok.map(r=>r.steps[h].bracketSpanSeconds)),rateResidualArcsecDay:stats(ok.map(r=>r.steps[h].rateResidualArcsecDay))}])),
    sampledMultipleSignChanges:{direct:ok.filter(r=>r.signChanges.direct>1).length,positionDerivative:ok.filter(r=>r.signChanges.positionDerivative>1).length}};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-engine-station-convergence/1',status:'EXPLORATORY',kernelSha256:kernelSha,matrixSha256:matrixSha,
    notes:'Self-consistency only: all rates and positions use the same engine, ephemeris, frame and corrections; numerical root spans are not physical timing uncertainty.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));if(summary.unresolved||summary.ok!==summary.expected)process.exitCode=1;
}
module.exports={signChanges};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
