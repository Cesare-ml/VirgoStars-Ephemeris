'use strict';
// SPDX-License-Identifier: CPAL-1.0. Different orientation conventions are not an error budget.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const years=[-9998,-7500,-5000,-2500,-1000,0,2000];
const wrap=x=>((x%360)+540)%360-180;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
function parse(raw,epochs){
  if(typeof raw!=='string'||!/^Target body name: Sun \(10\).*\{source: DE441\}/m.test(raw)||!/^Center body name: Earth \(399\).*\{source: DE441\}/m.test(raw)||!raw.includes('Atmos refraction: NO (AIRLESS)'))throw new Error('invalid target, center, ephemeris, or refraction');
  const header=raw.split(/\r?\n/).find(x=>x.includes('Date_________JDTT')&&x.includes('ObsEcLon')&&x.includes('ObsEcLat'));
  const start=raw.indexOf('$$SOE'),end=raw.indexOf('$$EOE');
  if(!header||start<0||end<=start||raw.indexOf('$$SOE',start+5)!==-1||raw.indexOf('$$EOE',end+5)!==-1)throw new Error('invalid Horizons table');
  const cols=header.split(',').map(x=>x.trim()),lon=cols.indexOf('ObsEcLon'),lat=cols.indexOf('ObsEcLat');
  const rows=raw.slice(start+5,end).trim().split(/\r?\n/);
  if(lon<1||lat<1||rows.length!==epochs.length)throw new Error('invalid Horizons columns or row count');
  return rows.map((line,i)=>{const f=line.split(',').map(x=>x.trim());
    if(![f[0],f[lon],f[lat]].every(x=>x!==''&&Number.isFinite(Number(x)))||Math.abs(Number(f[0])-epochs[i])>1e-8)throw new Error('invalid Horizons row');
    return [Number(f[lon]),Number(f[lat])];});
}
async function main(){
  const [kernel,expected,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!out||extra.length||fs.existsSync(out)||path.resolve(kernel)===path.resolve(out))throw new Error('usage: <de441> <sha256> <new-report>');
  if(await sha(kernel)!==expected)throw new Error('kernel hash mismatch');
  const epochs=years.map(y=>julday(y,7,1,12,true));
  const params=new URLSearchParams({format:'json',COMMAND:"'10'",OBJ_DATA:"'NO'",MAKE_EPHEM:"'YES'",EPHEM_TYPE:"'OBSERVER'",CENTER:"'500@399'",TLIST:`'${epochs.map(x=>x.toFixed(9)).join(' ')}'`,TLIST_TYPE:"'JD'",TIME_TYPE:"'TT'",CAL_FORMAT:"'JD'",QUANTITIES:"'31'",CSV_FORMAT:"'YES'",EXTRA_PREC:"'YES'",APPARENT:"'AIRLESS'"});
  const url=`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`,response=await fetch(url,{signal:AbortSignal.timeout(30000)});
  if(!response.ok)throw new Error(`Horizons HTTP ${response.status}`);
  const raw=(await response.json()).result,ref=parse(raw,epochs),engine=new Engine(path.resolve(kernel));
  const rows=years.map((year,i)=>{const tt=epochs[i],tdb=tt+0.001657*Math.sin((357.5291092+35999.0502909*((tt-2451545)/36525))*Math.PI/180)/86400;
    const pole=engine.experimentalLongTermApparentTdb(tdb,0).xx,direct=engine.experimentalLongTermDirectObliquityTdb(tdb,0).xx;
    if(![...pole,...direct].every(Number.isFinite))throw new Error('invalid engine result');
    return {year,jdTt:tt,jdTdb:tdb,horizonsLon:ref[i][0],horizonsLat:ref[i][1],poleMinusHorizonsLonArcsec:wrap(pole[0]-ref[i][0])*3600,poleMinusHorizonsLatArcsec:(pole[1]-ref[i][1])*3600,directMinusHorizonsLonArcsec:wrap(direct[0]-ref[i][0])*3600,directMinusHorizonsLatArcsec:(direct[1]-ref[i][1])*3600};});
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  fs.writeFileSync(out,JSON.stringify({schema:'horizons-ancient-ecliptic-conventions/1',status:'EXPLORATORY',kernelSha256:expected,notes:'Geocentric Sun apparent; Horizons quantity 31 IAU76/80, Owen long-term orientation, vs Vondrak mixed and pole frames; differing orientation and ephemeris models plus approximate TT/TDB: no inferred error bound.',url,raw,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(rows));
}
module.exports={parse};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
