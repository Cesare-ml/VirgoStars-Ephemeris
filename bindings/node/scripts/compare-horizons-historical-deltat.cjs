'use strict';
// SPDX-License-Identifier: CPAL-1.0. Historical reconstruction comparison, not an uncertainty estimate.
const fs=require('node:fs');
const {julday,experimentalEspenakMeeus2006DeltaT}=require('../index.js');
function parseHistoricalDeltaT(raw,epochs){
  if(typeof raw!=='string'||!Array.isArray(epochs)||!epochs.length||!epochs.every(Number.isFinite)
    ||!raw.includes('Target body name: Sun (10)')||!raw.includes('Center body name: Earth (399)')
    ||!raw.includes('Prior to 1962, the difference is')||!raw.includes('respect to UT1'))throw new Error('invalid source metadata');
  const header=raw.split(/\r?\n/).find(line=>line.includes('Date_________JDTT')&&line.includes('TDB-UT'));
  const start=raw.indexOf('$$SOE'),end=raw.indexOf('$$EOE');
  if(!header||start<0||end<=start||raw.indexOf('$$SOE',start+5)!==-1||raw.indexOf('$$EOE',end+5)!==-1)throw new Error('invalid table');
  const cols=header.split(',').map(x=>x.trim()),ix=cols.indexOf('TDB-UT');
  if(ix<1)throw new Error('missing time column');
  const rows=raw.slice(start+5,end).trim().split(/\r?\n/);
  if(rows.length!==epochs.length)throw new Error('wrong row count');
  return rows.map((line,i)=>{
    const f=line.split(',').map(x=>x.trim());
    if(!f[0]||!f[ix]||![Number(f[0]),Number(f[ix])].every(Number.isFinite)||Math.abs(Number(f[0])-epochs[i])>1e-8)throw new Error('invalid row or TT epoch');
    return Number(f[ix]);
  });
}
async function main(){
  const [out,...extra]=process.argv.slice(2);
  if(!out||extra.length||fs.existsSync(out))throw new Error('usage: <new-report>');
  const years=Array.from({length:39},(_,i)=>i*50),epochs=years.map(y=>julday(y,7,1,12,true));
  const params=new URLSearchParams({format:'json',COMMAND:"'10'",OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",
    TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'30'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'"});
  const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`;
  const response=await fetch(url,{signal:AbortSignal.timeout(30000)});if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
  const raw=(await response.json()).result,reference=parseHistoricalDeltaT(raw,epochs);
  const rows=years.map((year,i)=>{
    const jan=julday(year,1,1,0,true),next=julday(year+1,1,1,0,true),decimalYear=year+(epochs[i]-jan)/(next-jan);
    const modelSeconds=experimentalEspenakMeeus2006DeltaT(decimalYear).seconds;
    return {year,jdTt:epochs[i],decimalYear,modelSeconds,horizonsSeconds:reference[i],differenceSeconds:modelSeconds-reference[i]};
  });
  const worst=rows.reduce((a,b)=>Math.abs(b.differenceSeconds)>Math.abs(a.differenceSeconds)?b:a);
  const summary={count:rows.length,maxAbsoluteDifferenceSeconds:Math.abs(worst.differenceSeconds),maxYear:worst.year,
    rmsDifferenceSeconds:Math.sqrt(rows.reduce((s,r)=>s+r.differenceSeconds**2,0)/rows.length)};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-historical-deltat/1',status:'EXPLORATORY',url,raw,summary,rows,
    notes:'Pre-1962 Horizons quantity 30 is approximately TT−UT1; dates use proleptic Gregorian fractional years. Models may share observations. Differences are not independent truth or uncertainty bounds.'},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
module.exports={parseHistoricalDeltaT};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
