'use strict';
// SPDX-License-Identifier: CPAL-1.0. Historical models at selected events; no UTC assumption.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {julday,revjul,experimentalEspenakMeeus2006DeltaT}=require('../index.js');
const {parseHistoricalDeltaT}=require('./compare-horizons-historical-deltat.cjs');
const {stats}=require('./compare-swiss-station-matrix.cjs');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
async function fileSha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
function decimalGregorian(jd){
  if(!Number.isFinite(jd))throw new Error('invalid JD');const year=revjul(jd,true).year,a=julday(year,1,1,0,true),b=julday(year+1,1,1,0,true);
  return year+(jd-a)/(b-a);
}
// TT is fixed. Iteration makes the caller's decimal-year convention explicit:
// the model is evaluated at the inferred UT1 fraction of the Gregorian year.
function deltaAtTt(jdTt,estimate){
  if(!Number.isFinite(jdTt)||typeof estimate!=='function')throw new Error('invalid model input');
  let delta=estimate(decimalGregorian(jdTt));
  for(let i=0;i<10;i++){
    if(!Number.isFinite(delta))throw new Error('invalid Delta T');
    const year=decimalGregorian(jdTt-delta/86400),next=estimate(year);
    if(!Number.isFinite(next))throw new Error('invalid Delta T');
    if(Math.abs(next-delta)<1e-6)return {seconds:next,decimalYearUt1:year};delta=next;
  }
  throw new Error('Delta T inversion did not converge');
}
function ut1DifferenceSeconds(deltaA,deltaB){
  if(![deltaA,deltaB].every(Number.isFinite))throw new Error('invalid Delta T');return deltaB-deltaA;
}
// NASA/Espenak discussion of Morrison & Stephenson 2004: illustrative only.
function oldIllustrativeSigma(year){
  if(!Number.isFinite(year))throw new Error('invalid year');
  return year>=-999&&year<=1200?0.8*((year-1820)/100)**2:null;
}
async function main(){
  const [kernel,kernelSha,matrix,matrixSha,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!matrix||!/^[a-f0-9]{64}$/.test(matrixSha||'')||!out||extra.length||fs.existsSync(out)||new Set([kernel,matrix,out].map(p=>path.resolve(p))).size!==3||await fileSha(kernel)!==kernelSha)throw new Error('usage: <de441> <kernel-sha> <matrix> <matrix-sha> <new-report>');
  const bytes=fs.readFileSync(matrix);if(sha(bytes)!==matrixSha)throw new Error('matrix hash mismatch');const data=JSON.parse(bytes);
  if(data.schema!=='virgostars-swiss-station-matrix/1'||data.kernelSha256!==kernelSha||!Array.isArray(data.rows)||data.rows.length!==944)throw new Error('invalid matrix');
  const years=[100,500,1000,1500,1900],events=data.rows.filter(r=>years.includes(r.year)&&r.status==='ok').sort((a,b)=>a.engineBracket.lo-b.engineBracket.lo);
  if(events.length!==80||new Set(events.map(r=>`${r.year}:${r.body}:${r.kind}`)).size!==80)throw new Error('invalid event selection');
  const rows=[],requests=[];
  for(let offset=0;offset<events.length;offset+=60){
    const group=events.slice(offset,offset+60),epochs=group.map(r=>tt((r.engineBracket.lo+r.engineBracket.hi)/2));
    if(new Set(epochs.map(x=>x.toFixed(9))).size!==epochs.length)throw new Error('duplicate TT epochs');
    const params=new URLSearchParams({format:'json',COMMAND:"'10'",OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",
      TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'30'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'"});
    const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
    const raw=(await response.json()).result,ref=parseHistoricalDeltaT(raw,epochs);requests.push({url,raw});
    for(let i=0;i<group.length;i++){
      const r=group[i],model=deltaAtTt(epochs[i],year=>experimentalEspenakMeeus2006DeltaT(year).seconds),
        sigma=oldIllustrativeSigma(model.decimalYearUt1);
      rows.push({body:r.body,seedYear:r.year,kind:r.kind,jdTt:epochs[i],deltaModelSeconds:model.seconds,deltaHorizonsSeconds:ref[i],
        modelMinusHorizonsUt1Seconds:ut1DifferenceSeconds(model.seconds,ref[i]),oldIllustrativeSigmaSeconds:sigma});
    }
  }
  if(await fileSha(kernel)!==kernelSha||sha(fs.readFileSync(matrix))!==matrixSha)throw new Error('input changed');
  const summary={cases:rows.length,ut1DifferenceSeconds:stats(rows.map(r=>r.modelMinusHorizonsUt1Seconds)),
    bySeed:Object.fromEntries(years.map(y=>[y,stats(rows.filter(r=>r.seedYear===y).map(r=>r.modelMinusHorizonsUt1Seconds))])),
    oldIllustrativeSigmaRangeSeconds:Object.fromEntries(years.map(y=>{const a=rows.filter(r=>r.seedYear===y&&r.oldIllustrativeSigmaSeconds!==null).map(r=>r.oldIllustrativeSigmaSeconds);return [y,a.length?{min:Math.min(...a),max:Math.max(...a)}:null];}))};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-historical-station-ut1/1',status:'ILLUSTRATIVE',kernelSha256:kernelSha,matrixSha256:matrixSha,
    notes:'Same dynamical TT events; Delta T model differences are not uncertainty bounds. Historical 2004 sigma is an illustrative old estimate, not attached to Horizons or a current guarantee. UT1 is not UTC.',summary,rows,requests},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
module.exports={decimalGregorian,deltaAtTt,ut1DifferenceSeconds,oldIllustrativeSigma};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
