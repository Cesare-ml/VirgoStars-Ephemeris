'use strict';
// SPDX-License-Identifier: CPAL-1.0. Illustrative propagation at fixed TT, not a forecast guarantee.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {experimentalEspenakMeeus2006DeltaT}=require('../index.js');
const {deltaAtTt}=require('./compare-historical-station-ut1.cjs');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
async function fileSha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
// NASA, Uncertainty in Delta T (2007), Huber's future extrapolation calibrated at 2005.
function legacyFutureSigma(year){
  if(!Number.isFinite(year)||year<2005||year>3001)throw new Error('year outside illustrative range');
  const n=year-2005;return 365.25*n*Math.sqrt((n*0.058/3)*(1+n/2500))/1000;
}
// UT1 = TT - Delta T: calculate offsets to avoid subtracting large Julian dates.
function ut1Offsets(delta,sigma){
  if(!Number.isFinite(delta)||!Number.isFinite(sigma)||sigma<0)throw new Error('invalid interval');
  return {lowerSeconds:-delta-sigma,centerSeconds:-delta,upperSeconds:-delta+sigma};
}
async function main(){
  const [kernel,kernelSha,matrix,matrixSha,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!matrix||!/^[a-f0-9]{64}$/.test(matrixSha||'')||!out||extra.length||fs.existsSync(out)||new Set([kernel,matrix,out].map(p=>path.resolve(p))).size!==3||await fileSha(kernel)!==kernelSha)throw new Error('usage: <de441> <kernel-sha> <matrix> <matrix-sha> <new-report>');
  const bytes=fs.readFileSync(matrix);if(sha(bytes)!==matrixSha)throw new Error('matrix hash mismatch');const data=JSON.parse(bytes);
  if(data.schema!=='virgostars-swiss-station-matrix/1'||data.kernelSha256!==kernelSha||!Array.isArray(data.rows)||data.rows.length!==944)throw new Error('invalid matrix');
  const years=[2500,2950],events=data.rows.filter(r=>years.includes(r.year)&&r.status==='ok');
  if(events.length!==32||new Set(events.map(r=>`${r.year}:${r.body}:${r.kind}`)).size!==32)throw new Error('invalid event selection');
  const rows=events.map(r=>{
    const jdTt=tt((r.engineBracket.lo+r.engineBracket.hi)/2),model=deltaAtTt(jdTt,year=>experimentalEspenakMeeus2006DeltaT(year).seconds),sigma=legacyFutureSigma(model.decimalYearUt1);
    return {body:r.body,seedYear:r.year,kind:r.kind,jdTt,decimalYearUt1:model.decimalYearUt1,deltaModelSeconds:model.seconds,oldIllustrativeSigmaSeconds:sigma,ut1MinusTtSeconds:ut1Offsets(model.seconds,sigma)};
  });
  if(await fileSha(kernel)!==kernelSha||sha(fs.readFileSync(matrix))!==matrixSha)throw new Error('input changed');
  const summary={cases:rows.length,bySeed:Object.fromEntries(years.map(y=>{const a=rows.filter(r=>r.seedYear===y);return [y,{cases:a.length,sigmaSeconds:{min:Math.min(...a.map(r=>r.oldIllustrativeSigmaSeconds)),max:Math.max(...a.map(r=>r.oldIllustrativeSigmaSeconds))},deltaSeconds:{min:Math.min(...a.map(r=>r.deltaModelSeconds)),max:Math.max(...a.map(r=>r.deltaModelSeconds))}}];}))};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-future-station-ut1/1',status:'ILLUSTRATIVE',kernelSha256:kernelSha,matrixSha256:matrixSha,
    source:'https://eclipse.gsfc.nasa.gov/SEhelp/uncertainty2004.html',notes:'Old NASA/Huber future sigma applied hypothetically around a separate 2006 Delta T model at fixed TT; not a calibrated confidence interval or a current uncertainty guarantee. UT1 is not UTC or civil time. Station and frame errors are separate.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
module.exports={legacyFutureSigma,ut1Offsets};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
