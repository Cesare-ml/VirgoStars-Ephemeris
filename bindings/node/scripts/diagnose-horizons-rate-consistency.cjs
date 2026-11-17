'use strict';
// SPDX-License-Identifier: CPAL-1.0. Compare reported rates with position differences.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {parseIcrfRatesTable}=require('./horizons-icrf-parser.cjs');
const {icrfAngularRates,icrfAngles,icrfFiniteRates}=require('./icrf-angular-rates.cjs');
const cases=[[1,'301','Moon',100],[1,'301','Moon',500],[1,'301','Moon',2000],[2,'199','Mercury',0],
  [5,'5','Jupiter Barycenter',2000],[0,'10','Sun',2000]];
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function sha(f){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(f))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,expected,out,step,...extra]=process.argv.slice(2),h=step===undefined?0.001:Number(step);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!out||extra.length||fs.existsSync(out)
      ||!Number.isFinite(h)||h<=0||h>0.05||await sha(kernel)!==expected)
    throw new Error('usage: <kernel> <sha256> <new-report> [step-days]');
  const engine=new Engine(path.resolve(kernel)),rows=[];
  for(const [id,command,name,year] of cases){
    const jd=julday(year,7,1,12,true),tdb=[jd-h,jd,jd+h],epochs=tdb.map(tt),span=tdb[2]-tdb[0];
    const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",
      CENTER:"'500@399'",TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",
      QUANTITIES:"'45,46'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
    const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});
    if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
    const raw=(await response.json()).result,ref=parseIcrfRatesTable(raw,name,command,epochs);
    const states=tdb.map(t=>engine.experimentalApparentIcrfTdb(t,id)),own=states.map(s=>icrfAngles(s.positionKm));
    const engineRate=icrfAngularRates(states[1].positionKm,states[1].velocityKmPerDay);
    const row={name,year,jdTdb:jd,referenceRate:{raCosDecArcsecHour:ref[1].raCosDecArcsecHour,decArcsecHour:ref[1].decArcsecHour},
      referencePositionDifference:icrfFiniteRates(ref[0],ref[1],ref[2],span),engineRate,enginePositionDifference:icrfFiniteRates(own[0],own[1],own[2],span),url,raw};
    rows.push(row);console.log(JSON.stringify({name,year,referenceRate:row.referenceRate,referencePositionDifference:row.referencePositionDifference,
      engineRate,enginePositionDifference:row.enginePositionDifference}));
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-rates-consistency/1',status:'EXPLORATORY',kernelSha256:expected,hDays:h,rows},null,2)+'\n',{flag:'wx'});
}
main().catch(e=>{console.error(e.message);process.exitCode=1});
