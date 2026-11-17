'use strict';
// SPDX-License-Identifier: CPAL-1.0. Caller-provided USNO table, no data bundled.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {experimentalEspenakMeeus2006DeltaT}=require('../index.js');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
function auditHistoricalDeltaT(text,estimate){
  if(typeof text!=='string'||typeof estimate!=='function')throw new Error('invalid input');
  const rows=[];
  for(const line of text.split(/\r?\n/)){
    const trimmed=line.trim();if(!trimmed)continue;
    if(!/^\d/.test(trimmed))continue;
    const fields=trimmed.split(/\s+/),values=fields.map(Number);
    if(fields.length<3||!values.slice(0,3).every(Number.isFinite)||values[2]<0)throw new Error('invalid historical row');
    const [year,referenceSeconds,referenceErrorSeconds]=values;
    if(year<1657||year>1984.5||(rows.length&&year-rows.at(-1).year!==0.5))throw new Error('invalid sequence');
    const model=estimate(year);
    if(!Number.isFinite(model))throw new Error('invalid estimate');
    rows.push({year,referenceSeconds,referenceErrorSeconds,modelSeconds:model,differenceSeconds:model-referenceSeconds});
  }
  if(!rows.length)throw new Error('empty table');
  const summarize=items=>{
    if(!items.length)return null;
    const sorted=items.map(x=>Math.abs(x.differenceSeconds)).sort((a,b)=>a-b);
    const max=items.reduce((a,b)=>Math.abs(b.differenceSeconds)>Math.abs(a.differenceSeconds)?b:a);
    return {count:items.length,firstYear:items[0].year,lastYear:items.at(-1).year,
      maxAbsoluteSeconds:Math.abs(max.differenceSeconds),maxYear:max.year,
      rmsSeconds:Math.sqrt(items.reduce((a,x)=>a+x.differenceSeconds*x.differenceSeconds,0)/items.length),
      p95AbsoluteSeconds:sorted[Math.ceil(sorted.length*0.95)-1]};
  };
  return {rows,summary:{all:summarize(rows),pre1800:summarize(rows.filter(r=>r.year<1800)),
    y1800to1900:summarize(rows.filter(r=>r.year>=1800&&r.year<1900)),
    y1900to1972:summarize(rows.filter(r=>r.year>=1900&&r.year<1972)),
    y1972on:summarize(rows.filter(r=>r.year>=1972))}};
}
function main(){
  const [source,expected,out,...extra]=process.argv.slice(2);
  if(!source||!/^[a-f0-9]{64}$/.test(expected||'')||!out||extra.length||path.resolve(source)===path.resolve(out)||fs.existsSync(out))
    throw new Error('usage: <historic_deltat.data> <sha256> <new-report>');
  const bytes=fs.readFileSync(source);if(bytes.length>1e6||hash(bytes)!==expected)throw new Error('source hash mismatch');
  const result=auditHistoricalDeltaT(bytes.toString('utf8'),y=>experimentalEspenakMeeus2006DeltaT(y).seconds);
  if(result.rows.length!==656||result.rows[0].year!==1657||result.rows.at(-1).year!==1984.5)throw new Error('unexpected table coverage');
  if(hash(fs.readFileSync(source))!==expected)throw new Error('source changed');
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-historical-deltat-comparison/1',status:'EXPLORATORY',sourceSha256:expected,
    notes:'Differences are polynomial minus caller-supplied USNO table at the numeric years. Historical reconstructions and model estimates are not independent truth; no physical uncertainty is inferred.',...result},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(result.summary));
}
module.exports={auditHistoricalDeltaT};
if(require.main===module)try{main()}catch(e){console.error(e.message);process.exitCode=1}
