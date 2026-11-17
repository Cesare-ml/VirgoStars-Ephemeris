'use strict';
// SPDX-License-Identifier: CPAL-1.0. Sampled local smoothness experiment, not an event solver.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine}=require('../index.js');
const {extrapolatedRate}=require('./compare-swiss-seasonal-speeds.cjs');
const {stats}=require('./compare-swiss-station-matrix.cjs');
const {bisect}=require('./compare-swiss-station-roots.cjs');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
async function fileSha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
// Residuals after removing a best-fit line; describes the sampled series, not a confidence interval.
function detrend(x,y){
  if(!Array.isArray(x)||!Array.isArray(y)||x.length!==y.length||x.length<3||![...x,...y].every(Number.isFinite))throw new Error('invalid series');
  const n=x.length,mx=x.reduce((a,b)=>a+b,0)/n,my=y.reduce((a,b)=>a+b,0)/n;
  let xx=0,xy=0;for(let i=0;i<n;i++){xx+=(x[i]-mx)**2;xy+=(x[i]-mx)*(y[i]-my);}if(!(xx>0))throw new Error('degenerate times');
  const slope=xy/xx,residuals=y.map((v,i)=>v-my-slope*(x[i]-mx));
  let changes=0;for(let i=1;i<n;i++)if(y[i-1]*y[i]<0)changes++;
  return {rms:Math.sqrt(residuals.reduce((a,b)=>a+b*b,0)/n),max:Math.max(...residuals.map(Math.abs)),signChanges:changes,
    slopePerSecond:slope,interceptAtZero:my-slope*mx};
}
async function main(){
  const [kernel,kernelSha,swissPackage,matrix,matrixSha,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!swissPackage||!matrix||!/^[a-f0-9]{64}$/.test(matrixSha||'')||!out||extra.length||fs.existsSync(out)||new Set([kernel,matrix,out].map(p=>path.resolve(p))).size!==3||await fileSha(kernel)!==kernelSha)throw new Error('usage: <de441> <kernel-sha256> <swisseph> <matrix> <matrix-sha256> <new-report>');
  const bytes=fs.readFileSync(matrix);if(sha(bytes)!==matrixSha)throw new Error('matrix hash mismatch');const data=JSON.parse(bytes);
  if(data.schema!=='virgostars-swiss-station-matrix/1'||data.kernelSha256!==kernelSha||!Array.isArray(data.rows)||data.rows.length!==944)throw new Error('invalid matrix');
  const swiss=require(path.resolve(swissPackage)),engine=new Engine(path.resolve(kernel));swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  if(data.swissVersion!==swiss.swe_version())throw new Error('Swiss version differs');
  const modes={apparent:0,noDeflection:swiss.SEFLG_NOGDEFL,noAberration:swiss.SEFLG_NOABERR,noNutation:swiss.SEFLG_NONUT,truePosition:swiss.SEFLG_TRUEPOS};
  if(Object.entries(modes).some(([name,v])=>name!=='apparent'&&(!Number.isInteger(v)||!v)))throw new Error('missing Swiss flag');
  const ref=(jd,body,mode)=>{const flags=swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED|modes[mode],r=swiss.swe_calc(tt(jd),body,flags);
    if(!r||r.error||(r.rflag&flags)!==flags||![r.longitude,r.longitudeSpeed].every(Number.isFinite))throw new Error('invalid Swiss state');return r;};
  // Three previously observed bracket-sensitive cases supplement the fixed 48-case grid.
  const targeted=new Set(['1750:8:retrograde','1100:8:direct','500:9:direct']);
  const selected=data.rows.filter(r=>([50,1500,2950].includes(r.year)||targeted.has(`${r.year}:${r.body}:${r.kind}`))&&r.body>=2&&r.body<=9&&r.status==='ok');
  if(selected.length!==51||new Set(selected.map(r=>`${r.year}:${r.body}:${r.kind}`)).size!==51)throw new Error('invalid selection');
  const rows=[];
  for(const item of selected){
    const center=(item.publishedBracket.lo+item.publishedBracket.hi)/2,x=[],samples=Object.fromEntries([...Object.keys(modes),'positionDerivative','engine'].map(k=>[k,[]]));
    for(let k=-60;k<=60;k++){
      const jd=center+k*0.5/86400,h=0.02,at=t=>ref(t,item.body,'apparent').longitude;
      const sw=extrapolatedRate(at(jd-2*h),at(jd-h),at(jd+h),at(jd+2*h),jd,h,true),v=engine.experimentalLongTermApparentTdb(jd,item.body).xx;
      if(v.length!==6||!v.every(Number.isFinite))throw new Error('invalid engine state');
      x.push((jd-center)*86400);samples.positionDerivative.push(sw*3600);samples.engine.push(v[3]*3600);
      for(const mode of Object.keys(modes))samples[mode].push(ref(jd,item.body,mode).longitudeSpeed*3600);
    }
    const values=[];
    for(let i=0;i<20;i++){
      if(i%3===1)ref(center+1,item.body,'noNutation');
      if(i%3===2)ref(center-1,item.body===2?3:2,'apparent');
      values.push(ref(center,item.body,'apparent').longitudeSpeed);
    }
    const finiteRate=jd=>{const h=0.02,at=t=>ref(t,item.body,'apparent').longitude;return extrapolatedRate(at(jd-2*h),at(jd-h),at(jd+h),at(jd+2*h),jd,h,true);};
    const rates={published:jd=>ref(jd,item.body,'apparent').longitudeSpeed,positionDerivative:finiteRate,
      engine:jd=>engine.experimentalLongTermApparentTdb(jd,item.body).xx[3]};
    // Distinct, non-dyadically aligned brackets avoid merely reproducing the same bisection grid.
    const ranges=[[-0.493,0.507],[-0.731,0.269],[-0.317,0.683]],rootSpans={};
    for(const [name,fn] of Object.entries(rates)){
      const centers=ranges.map(([lo,hi])=>{const b=bisect(fn,center+lo,center+hi,0.00001);return (b.lo+b.hi)/2;});
      rootSpans[name]=(Math.max(...centers)-Math.min(...centers))*86400;
    }
    const metrics=Object.fromEntries(Object.entries(samples).map(([k,v])=>[k,detrend(x,v)]));
    const offset=-metrics.apparent.interceptAtZero/metrics.apparent.slopePerSecond;
    rows.push({year:item.year,body:item.body,kind:item.kind,centerJdTdb:center,metrics,rootSpansSeconds:rootSpans,
      publishedLinearZeroOffsetSeconds:Number.isFinite(offset)&&Math.abs(offset)<=30?offset:null,repeatedSameEpochUnique:new Set(values).size});
  }
  if(await fileSha(kernel)!==kernelSha||sha(fs.readFileSync(matrix))!==matrixSha)throw new Error('input changed');
  const names=Object.keys(rows[0].metrics),summary={cases:rows.length,pointsPerCase:121,spacingSeconds:0.5,
    modeRmsArcsecPerDay:Object.fromEntries(names.map(k=>[k,stats(rows.map(r=>r.metrics[k].rms))])),
    publishedSignChanges:Object.fromEntries([0,1,2,3,4,5,6,7,8].map(n=>[n,rows.filter(r=>r.metrics.apparent.signChanges===n).length])),
    publishedNineOrMoreSignChanges:rows.filter(r=>r.metrics.apparent.signChanges>=9).length,
    multipleSignChangeCases:rows.filter(r=>r.metrics.apparent.signChanges>1).length,
    bisectionSpanSeconds:Object.fromEntries(['published','positionDerivative','engine'].map(k=>[k,stats(rows.map(r=>r.rootSpansSeconds[k]))])),
    publishedLinearZeroOffsetSeconds:stats(rows.filter(r=>r.publishedLinearZeroOffsetSeconds!==null).map(r=>r.publishedLinearZeroOffsetSeconds)),
    repeatedSameEpochDifferent:rows.filter(r=>r.repeatedSameEpochUnique>1).length};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-swiss-station-smoothness/1',status:'EXPLORATORY',kernelSha256:kernelSha,matrixSha256:matrixSha,swissVersion:swiss.swe_version(),
    notes:'Detrended short-window residuals measure sampled smoothness, not physical uncertainties. Modes alter correction models; same DE441 and approximate TT.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
module.exports={detrend};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
