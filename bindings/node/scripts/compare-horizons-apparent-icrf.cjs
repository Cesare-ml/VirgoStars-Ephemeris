'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Manual independent-implementation diagnostic; Horizons and this use DE441.
const fs = require('node:fs');
const crypto = require('node:crypto');
const path = require('node:path');
const { Engine, julday } = require('../index.js');
const { validationYears } = require('./planet-validation-years.cjs');
const { parseIcrfTable } = require('./horizons-icrf-parser.cjs');
const bodies = [[0,'10','Sun'],[1,'301','Moon'],[2,'199','Mercury'],[3,'299','Venus'],
  [4,'4','Mars Barycenter'],[5,'5','Jupiter Barycenter'],[6,'6','Saturn Barycenter'],
  [7,'7','Uranus Barycenter'],[8,'8','Neptune Barycenter'],[9,'9','Pluto Barycenter']];
const rad = Math.PI/180;
const tdbToTt = jd => jd - 0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*rad)/86400;
const unit = (ra,dec) => [Math.cos(dec*rad)*Math.cos(ra*rad),Math.cos(dec*rad)*Math.sin(ra*rad),Math.sin(dec*rad)];
function separation(a,b) {
  const c=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
  return Math.atan2(Math.hypot(...c),a[0]*b[0]+a[1]*b[1]+a[2]*b[2])/rad*3600;
}
async function sha(file) { const h=crypto.createHash('sha256'); for await (const c of fs.createReadStream(file)) h.update(c); return h.digest('hex'); }
async function main() {
  const [kernel,expected,output,grid,...extra]=process.argv.slice(2);
  if (!kernel || !/^[a-f0-9]{64}$/.test(expected||'') || !output || extra.length) throw new Error('usage: <kernel> <sha256> <new-report> [grid]');
  const years=validationYears(grid);
  if (fs.existsSync(output) || await sha(kernel)!==expected) throw new Error('output exists or kernel hash mismatch');
  const engine=new Engine(path.resolve(kernel));
  const tdb=years.map(y=>julday(y,7,1,12,true)); const tt=tdb.map(tdbToTt);
  // Chosen before evaluating results, for investigation, not an approved accuracy requirement.
  const limits={moonArcsec:0.05,otherArcsec:0.01};
  const rows=[]; let moonMaxArcsec=0, otherMaxArcsec=0;
  for (const [id,command,name] of bodies) {
    const records=[],requests=[];
    // Keep request URLs bounded; validate each returned epoch independently.
    for (let offset=0;offset<years.length;offset+=61) {
      const batchTt=tt.slice(offset,offset+61);
      const params=new URLSearchParams({format:'json',COMMAND:`'${command}'`,OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",
        CENTER:"'500@399'",TLIST:`'${batchTt.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",
        QUANTITIES:"'45'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
      const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`;
      const response=await fetch(url,{signal:AbortSignal.timeout(30000)}); if (!response.ok) throw new Error(`Horizons HTTP ${response.status}`);
      const raw=(await response.json()).result;
      const table=parseIcrfTable(raw,name,command,batchTt);
      records.push(...table.map((fields,index)=>{
        const i=offset+index;
        const position=engine.experimentalApparentIcrfTdb(tdb[i],id).positionKm;
        if (position.length!==3 || !position.every(Number.isFinite) || !Math.hypot(...position)) throw new Error('invalid engine state');
        const differenceArcsec=separation(position,unit(fields.raDeg,fields.decDeg));
        if (id===1) moonMaxArcsec=Math.max(moonMaxArcsec,differenceArcsec);
        else otherMaxArcsec=Math.max(otherMaxArcsec,differenceArcsec);
        return {year:years[i],jdTdb:tdb[i],referenceRaDeg:fields.raDeg,referenceDecDeg:fields.decDeg,differenceArcsec};
      }));
      requests.push({url,raw});
    }
    rows.push({name,command,...(requests.length===1?requests[0]:{requests}),records});
  }
  if (await sha(kernel)!==expected) throw new Error('kernel changed');
  const passed=moonMaxArcsec<=limits.moonArcsec && otherMaxArcsec<=limits.otherArcsec;
  const summary={epochs:years.length,bodies:bodies.length,cases:years.length*bodies.length,moonMaxArcsec,otherMaxArcsec,limits,passed};
  fs.writeFileSync(output,`${JSON.stringify({schema:'virgostars-horizons-apparent-icrf/1',kernelSha256:expected,summary,rows},null,2)}\n`,{flag:'wx'});
  console.log(JSON.stringify(summary)); if (!passed) process.exitCode=1;
}
main().catch(error=>{console.error(error.message);process.exitCode=1;});
