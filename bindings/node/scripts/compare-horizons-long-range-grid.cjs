'use strict';
// SPDX-License-Identifier: CPAL-1.0. Manual JPL Horizons diagnostic only.
// Horizons and this engine use different ecliptic-of-date models: absolute
// residuals are cross-model comparisons, never automatically release criteria.
const fs = require('node:fs');
const crypto = require('node:crypto');
const path = require('node:path');
const { Engine, julday } = require('../index.js');
const bodies = [[0,'10','Sun'],[1,'301','Moon'],[2,'199','Mercury'],[3,'299','Venus'],
  [4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],
  [7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter'],[9,'9','Pluto Barycenter']];
const years = Array.from({length:21},(_,i)=>i*250);
const rad = Math.PI/180;
const halfStep = 0.01;
const tdbToTt = jd => jd - 0.001657 * Math.sin((357.5291092 + 35999.0502909*((jd-2451545)/36525))*rad)/86400;
const wrap = x => ((x%360)+540)%360-180;
const unit = (a,b) => [Math.cos(b*rad)*Math.cos(a*rad),Math.cos(b*rad)*Math.sin(a*rad),Math.sin(b*rad)];
function angle(a,b) {
  const c=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
  return Math.atan2(Math.hypot(...c),a[0]*b[0]+a[1]*b[1]+a[2]*b[2])/rad*3600;
}
async function sha(file) { const h=crypto.createHash('sha256'); for await (const chunk of fs.createReadStream(file)) h.update(chunk); return h.digest('hex'); }
async function main() {
  const [kernel,expected,output,...extra] = process.argv.slice(2);
  if (!kernel || !/^[a-f0-9]{64}$/.test(expected||'') || !output || extra.length) throw new Error('usage: <kernel> <sha256> <new-report.json>');
  if (fs.existsSync(output) || await sha(kernel)!==expected) throw new Error('output exists or kernel hash mismatch');
  const engine = new Engine(path.resolve(kernel));
  const tdb=years.map(y=>julday(y,7,1,12,true));
  const samples=tdb.flatMap(jd=>[jd-halfStep,jd,jd+halfStep]);
  const tt=samples.map(tdbToTt);
  const rows=[];
  for (const [id,command,name] of bodies) {
    const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",
      CENTER:"'500@399'",TLIST:`'${tt.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'2,31'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'"});
    const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`;
    const response=await fetch(url,{signal:AbortSignal.timeout(30000)}); if (!response.ok) throw new Error(`Horizons HTTP ${response.status}`);
    const raw=(await response.json()).result;
    if (typeof raw!=='string' || !raw.includes(`Target body name: ${name} (${command})`)
        || !/Target body name:.*\{source: DE441\}/.test(raw) || !/Center body name: Earth \(399\).*\{source: DE441\}/.test(raw)) throw new Error(`wrong target metadata: ${name}`);
    const header=raw.split('\n').find(line=>line.includes('R.A.__(a-app)'));
    const start=raw.indexOf('$$SOE'); const end=raw.indexOf('$$EOE');
    if (!header || !header.includes('Date_________JDTT') || start<0 || end<=start) throw new Error(`invalid table: ${name}`);
    const cols=header.split(',').map(x=>x.trim()); const indexes=['R.A.__(a-app)','DEC___(a-app)','ObsEcLon','ObsEcLat'].map(x=>cols.indexOf(x));
    if (indexes.some(x=>x<0)) throw new Error(`missing columns: ${name}`);
    const table=raw.slice(start+5,end).trim().split('\n').map(line=>line.split(',').map(x=>x.trim()));
    if (table.length!==samples.length) throw new Error(`wrong row count: ${name} (${table.length})`);
    for (let i=0;i<table.length;i++) {
      if (!table[i][0] || !Number.isFinite(Number(table[i][0])) || Math.abs(Number(table[i][0])-tt[i])>1e-8) throw new Error(`wrong TT epoch: ${name} row ${i}`);
    }
    const values=table.map(fields=>indexes.map(i=>{if (!fields[i] || !Number.isFinite(Number(fields[i]))) throw new Error(`invalid angle: ${name}`); return Number(fields[i]);}));
    const records=years.map((year,i)=>{
      const our=engine.experimentalLongTermApparentTdb(tdb[i],id).xx;
      if (!Array.isArray(our) || our.length !== 6 || !our.every(Number.isFinite)) throw new Error(`invalid engine result: ${name} ${year}`);
      const before=values[3*i], actual=values[3*i+1], after=values[3*i+2];
      const lonRate=wrap(after[2]-before[2])/(2*halfStep);
      const latRate=(after[3]-before[3])/(2*halfStep);
      return {year,jdTdb:tdb[i],referenceUnit:unit(actual[0],actual[1]),engineUnit:unit(our[0],our[1]),
        longitudeDifferenceArcsec:wrap(our[0]-actual[2])*3600, latitudeDifferenceArcsec:(our[1]-actual[3])*3600,
        longitudeRateDifferenceArcsecPerDay:(our[3]-lonRate)*3600,latitudeRateDifferenceArcsecPerDay:(our[4]-latRate)*3600};
    });
    rows.push({name,command,url,raw,records});
  }
  let moonMaxArcsec=0, otherMaxArcsec=0, maxLongitudeArcsec=0,maxLatitudeArcsec=0,maxLongitudeRateArcsecPerDay=0,maxLatitudeRateArcsecPerDay=0;
  for (const row of rows) for (const p of row.records) {
    maxLongitudeArcsec=Math.max(maxLongitudeArcsec,Math.abs(p.longitudeDifferenceArcsec)); maxLatitudeArcsec=Math.max(maxLatitudeArcsec,Math.abs(p.latitudeDifferenceArcsec));
    maxLongitudeRateArcsecPerDay=Math.max(maxLongitudeRateArcsecPerDay,Math.abs(p.longitudeRateDifferenceArcsecPerDay));
    maxLatitudeRateArcsecPerDay=Math.max(maxLatitudeRateArcsecPerDay,Math.abs(p.latitudeRateDifferenceArcsecPerDay));
  }
  for (let i=0;i<rows.length;i++) for (let j=i+1;j<rows.length;j++) for (let k=0;k<years.length;k++) {
    const error=Math.abs(angle(rows[i].records[k].referenceUnit,rows[j].records[k].referenceUnit)-angle(rows[i].records[k].engineUnit,rows[j].records[k].engineUnit));
    if (i===1 || j===1) moonMaxArcsec=Math.max(moonMaxArcsec,error); else otherMaxArcsec=Math.max(otherMaxArcsec,error);
  }
  if (await sha(kernel)!==expected) throw new Error('kernel changed');
  const limits={moonSeparationArcsec:0.05,otherSeparationArcsec:0.01};
  const passed=moonMaxArcsec<=limits.moonSeparationArcsec && otherMaxArcsec<=limits.otherSeparationArcsec;
  const summary={epochs:years.length,bodies:bodies.length,separationCases:45*years.length,moonMaxArcsec,otherMaxArcsec,
    maxLongitudeArcsec,maxLatitudeArcsec,maxLongitudeRateArcsecPerDay,maxLatitudeRateArcsecPerDay,limits,passed};
  fs.writeFileSync(output,`${JSON.stringify({schema:'virgostars-horizons-long-range-grid/1',kernelSha256:expected,halfStepDays:halfStep,years,summary,rows},null,2)}\n`,{flag:'wx'});
  console.log(JSON.stringify(summary)); if (!passed) process.exitCode=1;
}
main().catch(error=>{console.error(error.message);process.exitCode=1;});
