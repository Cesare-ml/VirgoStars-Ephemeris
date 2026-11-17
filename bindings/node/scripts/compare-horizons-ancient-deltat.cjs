'use strict';
// SPDX-License-Identifier: CPAL-1.0. Historical model comparison, not truth.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {julday}=require('../index.js');
const {parseHistoricalDeltaT}=require('./compare-horizons-historical-deltat.cjs');
const {deltaAtTt}=require('./compare-historical-station-ut1.cjs');
const {parabolas}=require('./audit-ancient-delta-models.cjs');
const years=[-9998,-9000,-5000,-2000,-1000,-719];
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
function swissAtTt(jd,swiss){
  let delta=swiss.swe_deltat(jd)?.delta*86400;
  for(let i=0;i<12;i++){
    if(!Number.isFinite(delta))throw new Error('invalid Swiss Delta T');
    const next=swiss.swe_deltat(jd-delta/86400)?.delta*86400;
    if(!Number.isFinite(next))throw new Error('invalid Swiss Delta T');
    if(Math.abs(next-delta)<1e-5)return next;
    delta=next;
  }
  throw new Error('Swiss Delta T inversion did not converge');
}
async function main(){
  const [kernel,expected,swissPackage,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!swissPackage||!out||extra.length
      ||path.resolve(kernel)===path.resolve(out)||fs.existsSync(out))throw new Error('usage: <de441> <sha256> <swisseph> <new-report>');
  if(await sha(kernel)!==expected)throw new Error('kernel hash mismatch');
  const swiss=require(path.resolve(swissPackage));swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  const epochs=years.map(y=>julday(y,7,1,12,true));
  const params=new URLSearchParams({format:'json',COMMAND:"'10'",OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",
    TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'30'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'"});
  const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});
  if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
  const raw=(await response.json()).result,ref=parseHistoricalDeltaT(raw,epochs);
  const rows=years.map((year,i)=>{
    const newer=deltaAtTt(epochs[i],y=>parabolas(y).stephenson2016Seconds);
    const older=deltaAtTt(epochs[i],y=>parabolas(y).espenakMeeus2006Seconds);
    const swissSeconds=swissAtTt(epochs[i],swiss);
    return {year,jdTt:epochs[i],horizonsSeconds:ref[i],swissSeconds,
      oldParabolaSeconds:older.seconds,newParabolaSeconds:newer.seconds,
      swissMinusHorizonsSeconds:swissSeconds-ref[i],newMinusHorizonsSeconds:newer.seconds-ref[i],
      oldMinusHorizonsSeconds:older.seconds-ref[i]};
  });
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const notes='Fixed TT instants, Horizons quantity 30 is approximately TT−UT1 before 1962; models may share observations. NASA 2006 parabola before -1999 and 2016 parabola before its fitted era are extrapolations, not uncertainty bounds or observations. Proleptic Gregorian decimal year inferred at UT1.';
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-ancient-deltat/1',status:'EXPLORATORY',kernelSha256:expected,notes,url,raw,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({cases:rows.length,first:rows[0],last:rows.at(-1)}));
}
module.exports={swissAtTt};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
