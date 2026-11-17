'use strict';
// SPDX-License-Identifier: CPAL-1.0. Compare fixed ICRF and of-date conventions.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine}=require('../index.js');
const {parseIcrfTable}=require('./horizons-icrf-parser.cjs');
const {parse:parseEcliptic}=require('./compare-horizons-ancient-ecliptic.cjs');
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
const rad=Math.PI/180;
function unit(ra,dec){return [Math.cos(dec*rad)*Math.cos(ra*rad),Math.cos(dec*rad)*Math.sin(ra*rad),Math.sin(dec*rad)];}
function separation(a,b){const cross=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]],dot=a.reduce((s,x,i)=>s+x*b[i],0);return Math.atan2(Math.hypot(...cross),dot)/rad*3600;}
async function main(){
  const [kernel,kernelHash,source,sourceHash,out,...extra]=process.argv.slice(2);
  if(!kernel||!source||!out||extra.length||![kernelHash,sourceHash].every(x=>/^[a-f0-9]{64}$/.test(x||''))||fs.existsSync(out)||new Set([kernel,source,out].map(x=>path.resolve(x))).size!==3)throw new Error('usage: <de441> <sha256> <ecliptic-report> <report-sha256> <new-report>');
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input hash mismatch');
  const prior=JSON.parse(fs.readFileSync(source));
  if(prior.schema!=='horizons-ancient-ecliptic-conventions/1'||prior.kernelSha256!==kernelHash||!Array.isArray(prior.rows)||!prior.rows.length||!prior.rows.every(r=>[r.year,r.jdTt,r.jdTdb,r.directMinusHorizonsLonArcsec,r.poleMinusHorizonsLonArcsec].every(Number.isFinite)))throw new Error('invalid source report');
  const tt=prior.rows.map(r=>r.jdTt);parseEcliptic(prior.raw,tt);
  const params=new URLSearchParams({format:'json',COMMAND:"'10'",OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",TLIST:`'${tt.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'45'",REF_SYSTEM:"'ICRF'",ANG_FORMAT:"'DEG'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
  const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});
  if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
  const raw=(await response.json()).result;if(!raw.includes('Atmos refraction: NO (AIRLESS)'))throw new Error('unexpected refraction');
  const refs=parseIcrfTable(raw,'Sun','10',tt),engine=new Engine(path.resolve(kernel));
  const rows=prior.rows.map((r,i)=>{const position=engine.experimentalApparentIcrfTdb(r.jdTdb,0).positionKm;
    if(position.length!==3||!position.every(Number.isFinite)||!Math.hypot(...position))throw new Error('invalid state');
    return {year:r.year,jdTt:r.jdTt,jdTdb:r.jdTdb,icrfSeparationArcsec:separation(position,unit(refs[i].raDeg,refs[i].decDeg)),directLongitudeDifferenceArcsec:r.directMinusHorizonsLonArcsec,poleLongitudeDifferenceArcsec:r.poleMinusHorizonsLonArcsec};});
  if(await sha(kernel)!==kernelHash||await sha(source)!==sourceHash)throw new Error('input changed');
  fs.writeFileSync(out,JSON.stringify({schema:'horizons-frame-isolation/1',status:'EXPLORATORY',kernelSha256:kernelHash,sourceSha256:sourceHash,notes:'Same geocentric Sun, fixed TT, airless apparent coordinates, DE441. ICRF angles from Horizons quantity 45 and ecliptic angles from quantity 31 use different frames; small ICRF differences do not prove which long-term orientation is physically correct.',url,raw,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(rows));
}
module.exports={unit,separation};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
