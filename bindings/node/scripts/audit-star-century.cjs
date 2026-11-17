'use strict';
// SPDX-License-Identifier: CPAL-1.0. Local Swiss comparison, not identity proof.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,prepareStarCatalog,julday,constants}=require('../index.js');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const diff=(a,b)=>Math.abs((((a-b)%360)+540)%360-180)*3600;
const ttToTdb=jd=>jd+0.001657*Math.sin((357.5291092+35999.0502909*((jd-2451545)/36525))*Math.PI/180)/86400;
async function sha(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file))h.update(c);return h.digest('hex');}
async function main(){
  const [manifestPath,hipPath,kernelPath,kernelSha,swissPackage,swissPath,outPath,mode='canonical',...extra]=process.argv.slice(2);
  const files=[manifestPath,hipPath,kernelPath,swissPath,outPath];
  if(!files.every(Boolean)||!swissPackage||!/^[a-f0-9]{64}$/.test(kernelSha||'')||!['canonical','all-names'].includes(mode)||extra.length
      ||new Set(files.map(p=>path.resolve(p))).size!==files.length||fs.existsSync(outPath))throw new Error('usage: <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <new-report>');
  if(path.basename(swissPath).toLowerCase()!=='sefstars.txt'||path.resolve(path.dirname(swissPath))!==path.resolve(path.dirname(kernelPath)))throw new Error('expected catalog alongside ephemeris');
  const mb=fs.readFileSync(manifestPath),sb=fs.readFileSync(swissPath);
  if(mb.length>8e6||sb.length>8e6)throw new Error('input too large');
  const manifest=JSON.parse(mb),catalog=prepareStarCatalog(manifest);
  if(await sha(kernelPath)!==kernelSha||await sha(hipPath)!==manifest.sourceHashes.hip2Sha256||hash(sb)!==manifest.sourceHashes.swissCatalogSha256)throw new Error('source mismatch');
  const engine=new Engine(path.resolve(kernelPath));engine.loadHipparcos(path.resolve(hipPath),catalog.hips,catalog.qualityPolicy);
  const swiss=require(path.resolve(swissPackage));swiss.swe_set_ephe_path(path.dirname(path.resolve(kernelPath)));swiss.swe_set_jpl_file(path.basename(kernelPath));
  const epochs=[1900,1925,1950,1975,2000,2025,2050,2075,2099].map(year=>({year,jd:julday(year,5,15,12,true)}));
  const thresholdArcsec=1,failures=[];let compared=0,maxLongitudeArcsec=0,maxLatitudeArcsec=0;
  const targets=mode==='canonical'?catalog.stars:catalog.stars.flatMap(star=>[star.name,...star.aliases].map(name=>({...star,name})));
  for(const star of targets)for(const epoch of epochs){
    const declared=star.lookupWarnings?.find(w=>w.name.toLocaleLowerCase('en-US')===star.name.toLocaleLowerCase('en-US'));
    let ref,actual;
    try{ref=swiss.swe_fixstar(star.name,epoch.jd,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);}catch{}
    if(!ref||ref.error||!(ref.rflag&swiss.SEFLG_JPLEPH)||!Number.isFinite(ref.longitude)||!Number.isFinite(ref.latitude)){
      failures.push({hip:star.hip,sourceId:star.sourceId,name:star.name,year:epoch.year,reason:'REFERENCE_UNAVAILABLE',...(declared?{declaredWarning:declared.code}:{})});continue;
    }
    try{actual=(star.astrometry
      ? engine.fixstarExplicitTdb(star.astrometry,star.radialVelocityKmS,ttToTdb(epoch.jd),constants.SEFLG_SPEED)
      : engine.fixstarTdb(star.hip,star.radialVelocityKmS,ttToTdb(epoch.jd),constants.SEFLG_SPEED)).xx;}catch(error){
      failures.push({hip:star.hip,sourceId:star.sourceId,name:star.name,year:epoch.year,reason:'ENGINE_ERROR',code:error.code});continue;
    }
    if(!actual.every(Number.isFinite))throw new Error('non-finite engine output');
    const longitudeArcsec=diff(actual[0],ref.longitude),latitudeArcsec=Math.abs(actual[1]-ref.latitude)*3600;
    compared++;maxLongitudeArcsec=Math.max(maxLongitudeArcsec,longitudeArcsec);maxLatitudeArcsec=Math.max(maxLatitudeArcsec,latitudeArcsec);
    if(longitudeArcsec>thresholdArcsec||latitudeArcsec>thresholdArcsec)failures.push({hip:star.hip,sourceId:star.sourceId,name:star.name,year:epoch.year,reason:'POSITION_MISMATCH',longitudeArcsec,latitudeArcsec,
      ...(declared?.code==='SWISS_POSITION_MISMATCH'?{declaredWarning:declared.code}:{})});
  }
  if(await sha(kernelPath)!==kernelSha||await sha(hipPath)!==manifest.sourceHashes.hip2Sha256||await sha(swissPath)!==hash(sb)||await sha(manifestPath)!==hash(mb))throw new Error('input changed');
  const summary={stars:catalog.stars.length,lookupNames:targets.length,mode,epochs:epochs.length,attempted:targets.length*epochs.length,compared,thresholdArcsec,maxLongitudeArcsec,maxLatitudeArcsec,
    failedCases:failures.length,failedStars:new Set(failures.map(x=>x.hip??`source:${x.sourceId}`)).size,
    declaredCases:failures.filter(x=>x.declaredWarning).length,unexpectedCases:failures.filter(x=>!x.declaredWarning).length};
  fs.writeFileSync(outPath,JSON.stringify({schema:'virgostars-star-century-diagnostic/1',status:'REVIEW_REQUIRED',manifestSha256:hash(mb),
    hip2Sha256:manifest.sourceHashes.hip2Sha256,swissCatalogSha256:hash(sb),kernelSha256:kernelSha,
    notes:'Swiss TT input; approximate corresponding one-term TDB input to this engine',epochs,summary,failures},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(summary));
}
main().catch(e=>{console.error(e.message);process.exitCode=1;});
