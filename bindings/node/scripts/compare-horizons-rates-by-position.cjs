'use strict';
// SPDX-License-Identifier: CPAL-1.0. Independent sampled direction derivatives.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {validationYears}=require('./planet-validation-years.cjs');
const {parseIcrfTable}=require('./horizons-icrf-parser.cjs');
const {icrfAngularRates,icrfAngles,icrfFiniteRates}=require('./icrf-angular-rates.cjs');
const bodies=[[0,'10','Sun'],[1,'301','Moon'],[2,'199','Mercury'],[3,'299','Venus'],
  [4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],
  [7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter'],[9,'9','Pluto Barycenter']];
const tt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function sha(f){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(f))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,expected,out,grid,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!out||extra.length||fs.existsSync(out)||await sha(kernel)!==expected)
    throw new Error('usage: <kernel> <sha256> <new-report> [grid]');
  const years=validationYears(grid??'0-3000-step-50'),engine=new Engine(path.resolve(kernel)),h=0.001;
  // Diagnostic only: angular components in arcseconds per hour, fixed before fetching.
  const limitArcsecHour=0.005,records=[],failures=[],sources=[];
  for(const [id,command,name] of bodies){
    const requests=[];
    for(let offset=0;offset<years.length;offset+=20){
      const group=years.slice(offset,offset+20),centers=group.map(y=>julday(y,7,1,12,true));
      const tdb=centers.flatMap(jd=>[jd-h,jd,jd+h]),epochs=tdb.map(tt);
      const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",
        CENTER:"'500@399'",TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",
        QUANTITIES:"'45'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
      const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});
      if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
      const raw=(await response.json()).result,ref=parseIcrfTable(raw,name,command,epochs);
      for(let i=0;i<group.length;i++){
        const j=i*3,span=tdb[j+2]-tdb[j],r=icrfFiniteRates(ref[j],ref[j+1],ref[j+2],span);
        const states=tdb.slice(j,j+3).map(t=>engine.experimentalApparentIcrfTdb(t,id));
        const actual=icrfAngularRates(states[1].positionKm,states[1].velocityKmPerDay),p=states.map(s=>icrfAngles(s.positionKm));
        const self=icrfFiniteRates(p[0],p[1],p[2],span);
        const row={body:name,year:group[i],jdTdb:centers[i],referencePositionRate:r,engineRate:actual,enginePositionRate:self,
          referenceRaResidual:actual.raCosDecArcsecHour-r.raCosDecArcsecHour,
          referenceDecResidual:actual.decArcsecHour-r.decArcsecHour,
          ownRaResidual:actual.raCosDecArcsecHour-self.raCosDecArcsecHour,
          ownDecResidual:actual.decArcsecHour-self.decArcsecHour};
        records.push(row);
        if(Math.max(Math.abs(row.referenceRaResidual),Math.abs(row.referenceDecResidual))>limitArcsecHour)failures.push(row);
      }
      requests.push({url,raw});
    }
    sources.push({name,command,requests});
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const maximum=key=>{const r=records.reduce((a,b)=>Math.abs(b[key])>Math.abs(a[key])?b:a);return {absolute:Math.abs(r[key]),body:r.body,year:r.year};};
  const perBody=bodies.map(([, ,name])=>{const rows=records.filter(x=>x.body===name),m=key=>Math.max(...rows.map(r=>Math.abs(r[key])));
    return {body:name,raArcsecHour:m('referenceRaResidual'),decArcsecHour:m('referenceDecResidual')};});
  const summary={cases:records.length,epochs:years.length,bodies:10,stepTdbDays:h,limitArcsecHour,failures:failures.length,
    maxReferenceRa:maximum('referenceRaResidual'),maxReferenceDec:maximum('referenceDecResidual'),
    maxOwnRa:maximum('ownRaResidual'),maxOwnDec:maximum('ownDecResidual'),perBody};
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-horizons-position-differentiation/1',status:'EXPLORATORY',kernelSha256:expected,
    notes:'TDB vs approximate TT; finite differences of printed ICRF positions; same DE441; rate comparison is not independent orbital truth.',summary,records,failures,sources},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));if(failures.length)process.exitCode=1;
}
main().catch(e=>{console.error(e.message);process.exitCode=1});
