'use strict';
// SPDX-License-Identifier: CPAL-1.0. Published parabola comparison only.
// Neither formula is an approved ancient-time model or uncertainty estimate.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,julday}=require('../index.js');
const years=[-12999,-10000,-5000,-1999,-1000,-719],shiftScenario=3600;
const wrap=x=>((x%360)+540)%360-180;
function parabolas(year){
  if(!Number.isFinite(year))throw new Error('finite astronomical decimal year required');
  const older=(year-1820)/100,newer=(year-1825)/100;
  return {espenakMeeus2006Seconds:-20+32*older*older,stephenson2016Seconds:-320+32.5*newer*newer};
}
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [kernel,expected,swissPackage,out,...extra]=process.argv.slice(2);
  if(!kernel||!/^[a-f0-9]{64}$/.test(expected||'')||!swissPackage||!out||extra.length
      ||path.resolve(kernel)===path.resolve(out)||fs.existsSync(out))throw new Error('usage: <de441> <sha256> <swisseph> <new-report>');
  if(await sha(kernel)!==expected)throw new Error('kernel hash mismatch');
  const swiss=require(path.resolve(swissPackage)),engine=new Engine(path.resolve(kernel));
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));swiss.swe_set_jpl_file(path.basename(kernel));
  const rows=[];
  for(const year of years){
    const jd=julday(year,7,1,12,true),jan=julday(year,1,1,0,true),next=julday(year+1,1,1,0,true);
    const decimalYear=year+(jd-jan)/(next-jan),oldNew=parabolas(decimalYear),swissDelta=swiss.swe_deltat(jd)?.delta*86400;
    if(!Number.isFinite(swissDelta))throw new Error(`invalid Swiss Delta T ${year}`);
    const models={swissSeconds:swissDelta,...oldNew},moon={};
    for(const [name,dt] of Object.entries(models)){
      const result=engine.experimentalLongTermDirectObliquityUt1(jd,1,dt,shiftScenario,`${name}; illustrative uncalibrated scenario`);
      if(!result.xx.every(Number.isFinite))throw new Error(`invalid engine result ${year}`);
      moon[name]=result.xx[0];
    }
    rows.push({year,jdUt1:jd,decimalYear,models,moonLongitudeDeg:moon,
      oldMinusNewSeconds:oldNew.espenakMeeus2006Seconds-oldNew.stephenson2016Seconds,
      oldMinusNewMoonArcmin:wrap(moon.espenakMeeus2006Seconds-moon.stephenson2016Seconds)*60,
      newerMinusSwissSeconds:oldNew.stephenson2016Seconds-swissDelta});
  }
  if(await sha(kernel)!==expected)throw new Error('kernel changed');
  const notes='Proleptic Gregorian UT1; NASA 2006 before -1999 and the 2016 parabola before its observational span are unvalidated extrapolations, not historical truth. The 2016 parabola is not its observational spline. Models may share observations; differences and 1h scenario are not uncertainty bounds. Direct-obliquity mixed frame, approximate TT/TDB.';
  fs.writeFileSync(out,JSON.stringify({schema:'virgostars-ancient-delta-models/1',status:'EXPLORATORY',kernelSha256:expected,notes,rows},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({cases:rows.length,first:rows[0],last:rows.at(-1)}));
}
module.exports={parabolas};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1});
