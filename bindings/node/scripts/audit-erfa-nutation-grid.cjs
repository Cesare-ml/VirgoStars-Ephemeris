'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Comparing extrapolated models does not validate ancient Earth orientation.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync}=require('node:child_process');
const {julday}=require('../index.js');
function epochGrid(){
  const records=new Map(),years=[-12999,...Array.from({length:679},(_,i)=>-12975+i*25),3999];
  const add=(year,month,dayOffset,cohort)=>{
    const jdTt=month===null?julday(year,1,1,12,true)+dayOffset:julday(year,month,1,12,true);
    const existing=records.get(jdTt);
    if(existing){if(!existing.cohorts.includes(cohort))existing.cohorts.push(cohort);}
    else records.set(jdTt,{jdTt,year,month,dayOffset,cohorts:[cohort]});
  };
  for(const year of years)for(const month of [1,4,7,10])add(year,month,null,'seasonal');
  for(const year of [-12999,-12500,-10000,-7000,-5000,-1000,0,1000,1900,2000,2100,3000,3999])for(let month=1;month<=12;month++)add(year,month,null,'monthly');
  for(const year of [-12999,2000,3999])for(let day=0;day<365;day++)add(year,null,day,'daily');
  return [...records.values()].sort((a,b)=>a.jdTt-b.jdTt);
}
function summarize(rows){
  if(!Array.isArray(rows)||!rows.length||!rows.every(r=>Number.isFinite(r.jdTt)&&Number.isInteger(r.year)&&[r.localPsi,r.localEps,r.linearPsi,r.linearEps,r.aPsi,r.aEps,r.bPsi,r.bEps].every(Number.isFinite)))throw new Error('invalid rows');
  const pairs={localMinusA:r=>[r.localPsi-r.aPsi,r.localEps-r.aEps],localMinusB:r=>[r.localPsi-r.bPsi,r.localEps-r.bEps],localMinusLinear:r=>[r.localPsi-r.linearPsi,r.localEps-r.linearEps],linearMinusB:r=>[r.linearPsi-r.bPsi,r.linearEps-r.bEps],bMinusA:r=>[r.bPsi-r.aPsi,r.bEps-r.aEps]};
  const max=(list,fn,index)=>{const r=list.reduce((a,b)=>Math.abs(fn(b)[index])>Math.abs(fn(a)[index])?b:a);return {absoluteArcsec:Math.abs(fn(r)[index]),jdTt:r.jdTt,year:r.year,month:r.month,dayOffset:r.dayOffset,cohorts:r.cohorts};};
  const group=list=>{if(!list.length)throw new Error('empty subgroup');return {cases:list.length,comparisons:Object.fromEntries(Object.entries(pairs).map(([name,fn])=>[name,{longitude:max(list,fn,0),obliquity:max(list,fn,1)}]))};};
  return {all:group(rows),preHorizons:group(rows.filter(r=>r.year< -9998)),modern:group(rows.filter(r=>r.year>=1900&&r.year<=2100))};
}
function checkLinearConformance(summary){
  const result=summary?.all?.comparisons?.linearMinusB;
  if(!result||![result.longitude?.absoluteArcsec,result.obliquity?.absoluteArcsec].every(x=>Number.isFinite(x)&&x>=0&&x<1e-8))throw new Error('linear-argument ERFA 2000B conformance failed');
}
function digest(file){const h=crypto.createHash('sha256'),fd=fs.openSync(file,'r'),buf=Buffer.allocUnsafe(1024*1024);try{for(;;){const n=fs.readSync(fd,buf,0,buf.length,null);if(!n)break;h.update(buf.subarray(0,n));}}finally{fs.closeSync(fd);}return h.digest('hex');}
function main(){
  const [python,projector,out,...extra]=process.argv.slice(2),helper=path.resolve(__dirname,'../../../scripts/compare_erfa_nutation_grid.py');
  if(!python||!projector||!out||extra.length||fs.existsSync(out)||new Set([python,projector,out,helper].map(p=>path.resolve(p))).size!==4)throw new Error('usage: PYTHON PROJECTOR NEW_REPORT (pyerfa 2.0.1.5 must be importable)');
  const inputs=[path.resolve(python),path.resolve(projector),helper],hashes=inputs.map(digest),grid=epochGrid(),epochs=grid.map(r=>r.jdTt);
  const py=JSON.parse(execFileSync(inputs[0],[helper],{input:JSON.stringify({epochs}),encoding:'utf8',timeout:120000,maxBuffer:8*1048576}));
  const refHashes=['erfaInitSha256','erfaCoreSha256','erfaBinarySha256','numpyInitSha256','numpyBinarySha256'];
  if(py.erfaVersion!=='2.0.1.5'||typeof py.erfaLibraryVersion!=='string'||typeof py.sofaVersion!=='string'||!Array.isArray(py.rows)||py.rows.length!==grid.length||!refHashes.every(k=>/^[a-f0-9]{64}$/.test(py[k]||'')))throw new Error('invalid ERFA response');
  const lines=execFileSync(inputs[1],[],{input:epochs.join('\n')+'\n',encoding:'utf8',timeout:30000,maxBuffer:2*1048576}).trim().split(/\r?\n/);
  if(lines.length!==grid.length)throw new Error('wrong local result count');
  const rows=grid.map((r,i)=>{
    const e=py.rows[i],fields=lines[i].split(',');
    if(!Array.isArray(e)||e.length!==5||!e.every(Number.isFinite)||e[0]!==r.jdTt||fields.length!==5||!fields.every(x=>x!==''&&Number.isFinite(Number(x)))||Number(fields[0])!==r.jdTt)throw new Error(`invalid result row ${i}`);
    return {...r,localPsi:Number(fields[1]),localEps:Number(fields[2]),linearPsi:Number(fields[3]),linearEps:Number(fields[4]),aPsi:e[1],aEps:e[2],bPsi:e[3],bEps:e[4]};
  });
  const anchor=rows.find(r=>r.jdTt===2451545.0);
  if(!anchor||Math.abs(anchor.localPsi-anchor.bPsi)>1e-7||Math.abs(anchor.localEps-anchor.bEps)>1e-7)throw new Error('J2000 implementation anchor mismatch');
  const summary=summarize(rows);checkLinearConformance(summary);
  if(inputs.some((p,i)=>digest(p)!==hashes[i]))throw new Error('input changed');
  const report={schema:'erfa-nutation-grid/1',status:'EXPLORATORY',pythonSha256:hashes[0],projectorSha256:hashes[1],helperSha256:hashes[2],erfaVersion:py.erfaVersion,erfaLibraryVersion:py.erfaLibraryVersion,sofaVersion:py.sofaVersion,numpyVersion:py.numpyVersion,pythonVersion:py.pythonVersion,...Object.fromEntries(refHashes.map(k=>[k,py[k]])),
    notes:'TT dates; local 77 terms with full IERS and linear arguments versus ERFA 2000A and standard 2000B. All distant epochs are extrapolations; model differences are not physical error bounds. Linear 2000B comparison uses a 1e-8 arcsec implementation-conformance gate, not a physical tolerance.',summary,rows};
  fs.writeFileSync(out,JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report.summary));
}
module.exports={epochGrid,summarize,checkLinearConformance};
if(require.main===module){try{main();}catch(e){console.error(e.message);process.exitCode=1;}}
