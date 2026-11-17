'use strict';
// SPDX-License-Identifier: CPAL-1.0. Historical Delta T sensitivity, not an uncertainty certification.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday,experimentalEspenakMeeus2006DeltaT}=require('../index.js');
const cases=[{year:0,sigma:265},{year:500,sigma:139},{year:1000,sigma:54},
  {year:1200,sigma:31},{year:2500,sigma:612},{year:3000,sigma:1885}];
const bodies=['Sun','Moon','Mercury','Venus','Mars barycenter','Jupiter barycenter',
  'Saturn barycenter','Uranus barycenter','Neptune barycenter','Pluto barycenter'];
const wrap=x=>((x%360)+540)%360-180;
async function sha(f){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(f))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,expected,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!out||extra.length||fs.existsSync(out)||await sha(kernel)!==expected)
    throw new Error('usage: <kernel> <sha256> <new-report>');
  const engine=new Engine(path.resolve(kernel)),rows=[];
  for(const {year,sigma} of cases){
    const ut1=julday(year,7,1,12,true),decimalYear=year+(7-0.5)/12;
    const delta=experimentalEspenakMeeus2006DeltaT(decimalYear).seconds;
    for(let body=0;body<10;body++){
      const calc=value=>engine.experimentalLongTermApparentUt1(ut1,body,value,sigma,'NASA 2004/2006 illustrative historical estimate');
      const before=calc(delta-sigma),center=calc(delta),after=calc(delta+sigma);
      if(![before,center,after].every(r=>r.xx.length===6&&r.xx.every(Number.isFinite)&&r.deltaTUncertaintySeconds===sigma))
        throw new Error('invalid sensitivity result');
      rows.push({year,body:bodies[body],jdUt1:ut1,deltaTSeconds:delta,sigmaSeconds:sigma,
        minusLongitudeArcsec:wrap(before.xx[0]-center.xx[0])*3600,
        plusLongitudeArcsec:wrap(after.xx[0]-center.xx[0])*3600,
        minusLatitudeArcsec:(before.xx[1]-center.xx[1])*3600,
        plusLatitudeArcsec:(after.xx[1]-center.xx[1])*3600});
    }
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const summary=cases.map(({year,sigma})=>{const x=rows.filter(r=>r.year===year),moon=x.find(r=>r.body==='Moon');
    const max=x.reduce((a,b)=>Math.max(Math.abs(b.minusLongitudeArcsec),Math.abs(b.plusLongitudeArcsec))>
      Math.max(Math.abs(a.minusLongitudeArcsec),Math.abs(a.plusLongitudeArcsec))?b:a);
    return {year,sigmaSeconds:sigma,deltaTSeconds:moon.deltaTSeconds,moonMinusArcsec:moon.minusLongitudeArcsec,
      moonPlusArcsec:moon.plusLongitudeArcsec,maxLongitudeBody:max.body,
      maxLongitudeArcsec:Math.max(Math.abs(max.minusLongitudeArcsec),Math.abs(max.plusLongitudeArcsec))};});
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-historical-time-sensitivity/1',status:'ILLUSTRATIVE',kernelSha256:expected,
    sources:['https://eclipse.gsfc.nasa.gov/SEhelp/deltatpoly2004.html','https://eclipse.gsfc.nasa.gov/SEhelp/uncertainty2004.html'],
    notes:'Historical sigma and projection are old illustrative estimates, not validated current uncertainty; TT-UT1 not UTC; angle response is nonlinear; no coverage between samples.',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
main().catch(e=>{console.error(e.message);process.exitCode=1});
