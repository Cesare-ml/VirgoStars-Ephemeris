'use strict';
// SPDX-License-Identifier: CPAL-1.0. Local comparison with public JPL reference.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {validationYears}=require('./planet-validation-years.cjs');
const {parseIcrfRatesTable}=require('./horizons-icrf-parser.cjs');
const {icrfAngularRates}=require('./icrf-angular-rates.cjs');
const bodies=[[0,'10','Sun'],[1,'301','Moon'],[2,'199','Mercury'],[3,'299','Venus'],
  [4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],
  [7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter'],[9,'9','Pluto Barycenter']];
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,expected,out,grid,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!out||extra.length)
    throw new Error('usage: <kernel> <sha256> <new-report> [grid]');
  const years=validationYears(grid??'0-3000-step-50');
  if(fs.existsSync(out)||await sha(kernel)!==expected)throw new Error('output exists or kernel mismatch');
  const engine=new Engine(path.resolve(kernel)),tdb=years.map(y=>julday(y,7,1,12,true)),ttList=tdb.map(tt);
  // Diagnostic threshold selected before fetching; not a release accuracy budget.
  const limitArcsecHour=0.01,rows=[],records=[],failures=[];
  for(const [id,command,name] of bodies){
    const requests=[];
    for(let offset=0;offset<years.length;offset+=61){
      const epochs=ttList.slice(offset,offset+61);
      const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",
        CENTER:"'500@399'",TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",
        QUANTITIES:"'45,46'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
      const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`;
      const response=await fetch(url,{signal:AbortSignal.timeout(30000)});if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
      const raw=(await response.json()).result,reference=parseIcrfRatesTable(raw,name,command,epochs);
      for(let j=0;j<reference.length;j++){
        const i=offset+j,state=engine.experimentalApparentIcrfTdb(tdb[i],id);
        const got=icrfAngularRates(state.positionKm,state.velocityKmPerDay),ref=reference[j];
        const row={body:name,year:years[i],jdTdb:tdb[i],engine:got,reference:ref,
          raCosDecDifferenceArcsecHour:got.raCosDecArcsecHour-ref.raCosDecArcsecHour,
          decDifferenceArcsecHour:got.decArcsecHour-ref.decArcsecHour};
        records.push(row);
        if(Math.max(Math.abs(row.raCosDecDifferenceArcsecHour),Math.abs(row.decDifferenceArcsecHour))>limitArcsecHour)failures.push(row);
      }
      requests.push({url,raw});
    }
    rows.push({name,command,requests});
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const maximum=key=>{const r=records.reduce((a,b)=>Math.abs(b[key])>Math.abs(a[key])?b:a);return {absolute:Math.abs(r[key]),body:r.body,year:r.year};};
  const perBody=bodies.map(([, ,name])=>{const r=records.filter(x=>x.body===name);const m=key=>Math.max(...r.map(x=>Math.abs(x[key])));return {body:name,maxRaCosDecArcsecHour:m('raCosDecDifferenceArcsecHour'),maxDecArcsecHour:m('decDifferenceArcsecHour')};});
  const summary={cases:records.length,epochs:years.length,bodies:bodies.length,limitArcsecHour,failures:failures.length,
    maxRaCosDecArcsecHour:maximum('raCosDecDifferenceArcsecHour'),maxDecArcsecHour:maximum('decDifferenceArcsecHour'),perBody};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-icrf-rates/1',status:'EXPLORATORY',kernelSha256:expected,
    notes:'Geocentric apparent ICRF, TDB vs approximate TT; Horizons prints target-dependent decimal precision; shared DE441.',summary,records,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));if(failures.length)process.exitCode=1;
}
main().catch(e=>{console.error(e.message);process.exitCode=1;});
