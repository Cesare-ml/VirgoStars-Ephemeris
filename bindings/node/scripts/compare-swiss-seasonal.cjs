'use strict';
// SPDX-License-Identifier: CPAL-1.0. Optional caller-provided Swiss diagnostic.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {validationYears}=require('./planet-validation-years.cjs');
const wrap=x=>((x%360)+540)%360-180;
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,expected,swissPackage,output,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!swissPackage||!output||extra.length)
    throw new Error('usage: <de441> <sha256> <swisseph> <new-report>');
  if(fs.existsSync(output)||await sha(kernel)!==expected)throw new Error('output exists or kernel hash mismatch');
  const swiss=require(path.resolve(swissPackage)),engine=new Engine(path.resolve(kernel));
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  // Investigation threshold, selected before running; not a release accuracy requirement.
  const limitArcsec=1,rows=[],failures=[];
  for(const year of validationYears('0-3000-step-10'))for(const month of [1,4,7,10])for(let body=0;body<10;body++){
    const jd=julday(year,month,1,12,true),actual=engine.experimentalLongTermApparentTdb(jd,body).xx;
    const ref=swiss.swe_calc(tt(jd),body,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);
    if(!ref||ref.error||!(ref.rflag&swiss.SEFLG_JPLEPH)||![ref.longitude,ref.latitude].every(Number.isFinite)
      ||!Array.isArray(actual)||actual.length!==6||!actual.every(Number.isFinite))throw new Error(`unavailable or nonfinite sample: ${year}/${month}, ${body}`);
    const row={year,month,body,jdTdb:jd,longitudeArcsec:wrap(actual[0]-ref.longitude)*3600,
      latitudeArcsec:(actual[1]-ref.latitude)*3600};
    rows.push(row);if(Math.abs(row.longitudeArcsec)>limitArcsec||Math.abs(row.latitudeArcsec)>limitArcsec)failures.push(row);
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const maxima=Object.fromEntries(['longitudeArcsec','latitudeArcsec'].map(key=>{
    const r=rows.reduce((a,b)=>Math.abs(b[key])>Math.abs(a[key])?b:a);
    return [key,{absolute:Math.abs(r[key]),year:r.year,month:r.month,body:r.body}];
  }));
  const summary={cases:rows.length,years:301,months:[1,4,7,10],bodies:10,limitArcsec,failures:failures.length,maxima};
  fs.writeFileSync(output,JSON.stringify({schema:'virgostars-swiss-seasonal-diagnostic/1',status:'EXPLORATORY',kernelSha256:expected,
    notes:'Experimental ecliptic of date; TDB versus approximate TT; shared DE441; diagnostic not scientific acceptance.',summary,failures,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));if(failures.length)process.exitCode=1;
}
main().catch(error=>{console.error(error.message);process.exitCode=1;});
