'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Local exploratory cross-model comparison; no Swiss code or data is bundled.
const fs=require('node:fs'), path=require('node:path'), crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {validationYears}=require('./planet-validation-years.cjs');
const rad=Math.PI/180;
const tdbToTt=jd=>jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*rad)/86400;
const wrap=x=>((x%360)+540)%360-180;
const finite=(x,label)=>{if(typeof x!=='number'||!Number.isFinite(x))throw new Error(`invalid ${label}`);return x;};
async function sha(p){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(p))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,expected,swissPackage,output,grid,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!swissPackage||!output||extra.length) throw new Error('usage: <de441> <sha256> <swisseph> <new-report> [grid]');
  const years=validationYears(grid);
  if(fs.existsSync(output)||await sha(kernel)!==expected)throw new Error('output exists or kernel hash mismatch');
  const swiss=require(path.resolve(swissPackage)), engine=new Engine(path.resolve(kernel));
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  const halfStep=0.001,rows=[];
  const reference=(jd,id)=>{
    const r=swiss.swe_calc(tdbToTt(jd),id,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);
    if(!r||r.error||!(r.rflag&swiss.SEFLG_JPLEPH))throw new Error(`Swiss JPL reference unavailable for ${id} at ${jd}`);
    for(const k of ['longitude','latitude','distance','longitudeSpeed','latitudeSpeed','distanceSpeed'])finite(r[k],k);
    return r;
  };
  for(const year of years)for(let body=0;body<=9;body++){
    const jd=julday(year,7,1,12,true), ours=engine.experimentalLongTermApparentTdb(jd,body).xx;
    if(ours.length!==6||ours.some(x=>!Number.isFinite(x)))throw new Error('invalid engine result');
    const before=reference(jd-halfStep,body),ref=reference(jd,body),after=reference(jd+halfStep,body);
    const beforeEngine=engine.experimentalLongTermApparentTdb(jd-halfStep,body).xx;
    const afterEngine=engine.experimentalLongTermApparentTdb(jd+halfStep,body).xx;
    const actualSpan=(jd+halfStep)-(jd-halfStep);
    const fdLon=wrap(after.longitude-before.longitude)/actualSpan;
    const fdLat=(after.latitude-before.latitude)/actualSpan;
    rows.push({year,body,jdTdb:jd,jdTt:tdbToTt(jd),longitudeDifferenceArcsec:wrap(ours[0]-ref.longitude)*3600,
      latitudeDifferenceArcsec:(ours[1]-ref.latitude)*3600,distanceDifferenceAu:ours[2]-ref.distance,
      engineLongitudeRateVsSwissFiniteArcsecPerDay:(ours[3]-fdLon)*3600,
      engineLatitudeRateVsSwissFiniteArcsecPerDay:(ours[4]-fdLat)*3600,
      engineLongitudeRateVsOwnFiniteArcsecPerDay:(ours[3]-wrap(afterEngine[0]-beforeEngine[0])/actualSpan)*3600,
      engineLatitudeRateVsOwnFiniteArcsecPerDay:(ours[4]-(afterEngine[1]-beforeEngine[1])/actualSpan)*3600,
      swissLongitudeSpeedVsFiniteArcsecPerDay:(ref.longitudeSpeed-fdLon)*3600,
      swissLatitudeSpeedVsFiniteArcsecPerDay:(ref.latitudeSpeed-fdLat)*3600});
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const keys=['longitudeDifferenceArcsec','latitudeDifferenceArcsec','distanceDifferenceAu','engineLongitudeRateVsSwissFiniteArcsecPerDay',
    'engineLatitudeRateVsSwissFiniteArcsecPerDay','engineLongitudeRateVsOwnFiniteArcsecPerDay','engineLatitudeRateVsOwnFiniteArcsecPerDay',
    'swissLongitudeSpeedVsFiniteArcsecPerDay','swissLatitudeSpeedVsFiniteArcsecPerDay'];
  const maxima=Object.fromEntries(keys.map(key=>{const row=rows.reduce((a,b)=>Math.abs(b[key])>Math.abs(a[key])?b:a);return [key,{absolute:Math.abs(row[key]),year:row.year,body:row.body}];}));
  const summary={cases:rows.length,years:years.length,bodies:10,halfStepDays:halfStep,maxima};
  fs.writeFileSync(output,JSON.stringify({schema:'virgostars-swiss-long-range/1',status:'EXPLORATORY',kernelSha256:expected,
    notes:'TDB input vs approximate TT; Swiss JPL flag checked on all 630 calls; differing model conventions; no release tolerance established',summary,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
main().catch(e=>{console.error(e.message);process.exitCode=1;});
