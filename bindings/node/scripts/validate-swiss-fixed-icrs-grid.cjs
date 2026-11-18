'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Caller supplies Swiss and DE441; nothing external is distributed here.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const {validationYears}=require('./planet-validation-years.cjs');
const {unit,separation}=require('./compare-horizons-frame-isolation.cjs');
const {direction}=require('./validate-horizons-planet-frame-grid.cjs');
function yearsFor(grid){
  if(grid==='baseline')return validationYears('13000bce-3999-step-250');
  if(grid==='interleaved')return Array.from({length:68},(_,i)=>-12875+i*250);
  throw new Error('unknown grid');
}
function tdbToTt(jd){
  if(!Number.isFinite(jd))throw new Error('invalid TDB');
  return jd-0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
}
function checkReference(r,flags,id,jd){
  if(!r||r.error||!Number.isInteger(r.rflag)||(r.rflag&flags)!==flags||![r.rectAscension,r.declination].every(Number.isFinite)||r.rectAscension<0||r.rectAscension>=360||Math.abs(r.declination)>90)throw new Error(`Swiss ICRS/JPL reference unavailable: ${id}, ${jd}: ${r?.error||'flags or coordinates'}`);
  return r;
}
function summarize(rows){
  if(!Array.isArray(rows)||!rows.length||!rows.every(r=>Number.isInteger(r.id)&&r.id>=0&&r.id<=9&&Number.isFinite(r.year)&&Number.isFinite(r.differenceArcsec)&&r.differenceArcsec>=0))throw new Error('invalid results');
  const max=list=>{if(!list.length)throw new Error('empty subgroup');const r=list.reduce((a,b)=>b.differenceArcsec>a.differenceArcsec?b:a);return {arcsec:r.differenceArcsec,year:r.year,month:r.month,id:r.id,solarElongationDeg:r.solarElongationDeg};};
  const byBody=Array.from({length:10},(_,id)=>{const r=rows.filter(x=>x.id===id);return {id,cases:r.length,max:max(r)};});
  return {cases:rows.length,byBody,moon:max(rows.filter(r=>r.id===1)),other:max(rows.filter(r=>r.id!==1)),preHorizons:max(rows.filter(r=>r.year< -9998)),overlap:max(rows.filter(r=>r.year>= -9997))};
}
function digest(file){const h=crypto.createHash('sha256'),fd=fs.openSync(file,'r'),buf=Buffer.allocUnsafe(1024*1024);try{for(;;){const n=fs.readSync(fd,buf,0,buf.length,null);if(!n)break;h.update(buf.subarray(0,n));}}finally{fs.closeSync(fd);}return h.digest('hex');}
function main(){
  const [kernel,kernelHash,swissDir,out,grid,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(kernelHash||'')||!swissDir||!out||!grid||extra.length||fs.existsSync(out))throw new Error('usage: KERNEL KERNEL_SHA SWISSEPH_PACKAGE NEW_REPORT baseline|interleaved');
  const years=yearsFor(grid),pkg=path.resolve(swissDir),inputs=[path.resolve(kernel),path.join(pkg,'package.json'),path.join(pkg,'lib/swisseph.js'),path.join(pkg,'build/Release/swisseph.node'),path.resolve(__dirname,'../native/virgostars_ephemeris.node')];
  if(new Set([...inputs,path.resolve(out)]).size!==inputs.length+1)throw new Error('output and inputs must differ');
  const hashes=inputs.map(digest);if(hashes[0]!==kernelHash)throw new Error('kernel hash mismatch');
  const swiss=require(pkg),version=swiss.swe_version();
  const needed=['SEFLG_JPLEPH','SEFLG_EQUATORIAL','SEFLG_J2000','SEFLG_ICRS'];
  if(!needed.every(k=>Number.isInteger(swiss[k])))throw new Error('required Swiss flags unavailable');
  const flags=needed.reduce((a,k)=>a|swiss[k],0),engine=new Engine(path.resolve(kernel));
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  const reference=(jd,id)=>checkReference(swiss.swe_calc(jd,id,flags),flags,id,jd);
  const rows=[];
  for(const year of years)for(const month of [1,4,7,10]){
    const jdTdb=julday(year,month,1,12,true),jdTt=tdbToTt(jdTdb),sun=reference(jdTt,0),sunDir=unit(sun.rectAscension,sun.declination);
    for(let id=0;id<10;id++){
      const ref=id===0?sun:reference(jdTt,id),state=engine.experimentalApparentIcrfTdb(jdTdb,id),referenceDir=unit(ref.rectAscension,ref.declination);
      rows.push({year,month,id,jdTdb,jdTt,referenceRaDeg:ref.rectAscension,referenceDecDeg:ref.declination,rflag:ref.rflag,
        solarElongationDeg:separation(referenceDir,sunDir)/3600,differenceArcsec:separation(direction(state.positionKm),referenceDir)});
    }
  }
  if(inputs.some((p,i)=>digest(p)!==hashes[i]))throw new Error('input changed');
  const report={schema:'swiss-fixed-icrs-grid/1',status:'EXPLORATORY',grid,kernelSha256:hashes[0],swissPackageSha256:hashes[1],swissWrapperSha256:hashes[2],swissNativeSha256:hashes[3],addonSha256:hashes[4],swissVersion:version,flags,
    notes:'Apparent geocentric ICRS/ICRF, TDB versus approximate TT. JPL, equatorial, J2000 and ICRS flags verified on every Swiss call. Both implementations share DE441. Sparse samples are not an independent orbital accuracy bound.',
    summary:summarize(rows),rows};
  fs.writeFileSync(out,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(report.summary));
}
module.exports={yearsFor,tdbToTt,checkReference,summarize};
if(require.main===module){try{main();}catch(e){console.error(e.message);process.exitCode=1;}}
