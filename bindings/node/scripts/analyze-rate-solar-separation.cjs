'use strict';
// SPDX-License-Identifier: CPAL-1.0. Exploratory grouping, not a physical correction.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
function separationDegrees(a,b){
  if(![a?.raDeg,a?.decDeg,b?.raDeg,b?.decDeg].every(Number.isFinite)||Math.abs(a.decDeg)>90||Math.abs(b.decDeg)>90)throw new Error('invalid direction');
  const rad=Math.PI/180,vec=p=>[Math.cos(p.decDeg*rad)*Math.cos(p.raDeg*rad),Math.cos(p.decDeg*rad)*Math.sin(p.raDeg*rad),Math.sin(p.decDeg*rad)];
  const x=vec(a),y=vec(b),cross=[x[1]*y[2]-x[2]*y[1],x[2]*y[0]-x[0]*y[2],x[0]*y[1]-x[1]*y[0]];
  return Math.atan2(Math.hypot(...cross),x[0]*y[0]+x[1]*y[1]+x[2]*y[2])/rad;
}
function summarize(reference,diagnostic){
  const positions=new Map();
  for(const r of reference.records){const key=`${r.body}:${r.year}`;if(positions.has(key))throw new Error('duplicate reference');positions.set(key,r.reference);}
  if(positions.size!==3010)throw new Error('unexpected reference size');
  const variants=['untransformed_no_retard','untransformed_no_retard_no_deflection_rate'],bins={};
  for(const variant of variants)bins[variant]=Object.fromEntries(['0-5','5-10','10-30','30-180'].map(k=>[k,{count:0,sumSquared:0,max:0}]));
  const seen=new Set();
  for(const r of diagnostic.output){
    if(!variants.includes(r.variant)||r.body==='Sun'||r.year%50===0)continue;
    const key=`${r.variant}:${r.body}:${r.year}`;if(seen.has(key))throw new Error('duplicate diagnostic');seen.add(key);
    if(![r.raResidualArcsecHour,r.decResidualArcsecHour].every(Number.isFinite))throw new Error('invalid residual');
    const angle=separationDegrees(positions.get(`${r.body}:${r.year}`),positions.get(`Sun:${r.year}`));
    const bin=angle<5?'0-5':angle<10?'5-10':angle<30?'10-30':'30-180',m=Math.hypot(r.raResidualArcsecHour,r.decResidualArcsecHour),x=bins[r.variant][bin];
    x.count++;x.sumSquared+=m*m;x.max=Math.max(x.max,m);
  }
  if(seen.size!==4320)throw new Error('missing held-out cases');
  return Object.fromEntries(variants.map(v=>[v,Object.fromEntries(Object.entries(bins[v]).map(([k,x])=>[k,{count:x.count,rmsArcsecHour:Math.sqrt(x.sumSquared/x.count),maxArcsecHour:x.max}]))]));
}
function main(){
  const [reference,referenceSha,diagnostic,diagnosticSha,out,...extra]=process.argv.slice(2);
  if(!reference||!/^[a-f0-9]{64}$/.test(referenceSha||'')||!diagnostic||!/^[a-f0-9]{64}$/.test(diagnosticSha||'')||!out||extra.length||fs.existsSync(out)||new Set([reference,diagnostic,out].map(p=>path.resolve(p))).size!==3)throw new Error('usage: <reference> <sha256> <component-report> <sha256> <new-report>');
  const rb=fs.readFileSync(reference),db=fs.readFileSync(diagnostic);
  if(sha(rb)!==referenceSha||sha(db)!==diagnosticSha)throw new Error('source hash mismatch');
  const r=JSON.parse(rb),d=JSON.parse(db);
  if(r.schema!=='virgostars-horizons-icrf-rates/1'||d.schema!=='virgostars-rate-components-diagnostic/1'||d.referenceSha256!==referenceSha)throw new Error('source schema or linkage mismatch');
  const summary=summarize(r,d);
  if(sha(fs.readFileSync(reference))!==referenceSha||sha(fs.readFileSync(diagnostic))!==diagnosticSha)throw new Error('source changed');
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-rate-solar-separation/1',referenceSha256:referenceSha,diagnosticSha256:diagnosticSha,status:'EXPLORATORY',summary},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
module.exports={separationDegrees,summarize};
if(require.main===module)try{main()}catch(e){console.error(e.message);process.exitCode=1;}
