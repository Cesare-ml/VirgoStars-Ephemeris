'use strict';
// SPDX-License-Identifier: CPAL-1.0. Recheck warnings after catalog data changes.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,prepareStarCatalog,julday,constants}=require('../index.js');
const {classifyCompatibility}=require('./attach-iau-physical-identities.cjs');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
async function sha(p){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(p))h.update(c);return h.digest('hex');}
async function main(){
  const [manifestPath,iauReportPath,hipPath,kernelPath,kernelSha,swissPackage,swissPath,outPath,reportPath,...extra]=process.argv.slice(2);
  const files=[manifestPath,iauReportPath,hipPath,kernelPath,swissPath,outPath,reportPath];
  if(!files.every(Boolean)||!swissPackage||!/^[a-f0-9]{64}$/.test(kernelSha||'')||extra.length
    ||new Set(files.map(p=>path.resolve(p))).size!==files.length||fs.existsSync(outPath)||fs.existsSync(reportPath))throw new Error('usage: <manifest> <iau-report> <hip2> <de441> <sha256> <swisseph> <sefstars> <new-manifest> <new-report>');
  const mb=fs.readFileSync(manifestPath),ib=fs.readFileSync(iauReportPath),sb=fs.readFileSync(swissPath);
  if([mb,ib,sb].some(b=>b.length>8e6))throw new Error('input too large');
  const manifest=JSON.parse(mb),input=JSON.parse(ib),catalog=prepareStarCatalog(manifest);
  if(input.schema!=='virgostars-iau-physical-identities/1'||input.status!=='REVIEW_REQUIRED'||!Array.isArray(input.added)
    ||input.iauSha256!==manifest.sourceHashes.iauSha256||input.kernelSha256!==kernelSha
    ||await sha(kernelPath)!==kernelSha||await sha(hipPath)!==manifest.sourceHashes.hip2Sha256
    ||hash(sb)!==manifest.sourceHashes.swissCatalogSha256)throw new Error('source mismatch');
  if(path.basename(swissPath).toLowerCase()!=='sefstars.txt'||path.resolve(path.dirname(swissPath))!==path.resolve(path.dirname(kernelPath)))throw new Error('expected Swiss catalog alongside ephemeris');
  const stars=manifest.stars.map(s=>({...s,aliases:[...s.aliases],...(s.lookupWarnings?{lookupWarnings:[...s.lookupWarnings]}:{})}));
  const byHip=new Map(stars.map(s=>[s.hip,s])),seen=new Set();
  const engine=new Engine(path.resolve(kernelPath));engine.loadHipparcos(path.resolve(hipPath),catalog.hips,catalog.qualityPolicy);
  const swiss=require(path.resolve(swissPackage));swiss.swe_set_ephe_path(path.dirname(path.resolve(kernelPath)));swiss.swe_set_jpl_file(path.basename(kernelPath));
  const epochs=[[1950,5,15],[2000,1,1],[2024,3,20]].map(([y,m,d])=>julday(y,m,d,12,true));
  const warnings=[];
  for(const row of input.added){
    if(typeof row.name!=='string'||!Number.isSafeInteger(row.hip)||seen.has(fold(row.name)))throw new Error('invalid IAU report');
    seen.add(fold(row.name));const star=byHip.get(row.hip);
    if(!star||![star.name,...star.aliases].some(x=>fold(x)===fold(row.name)))throw new Error('report name not in manifest');
    const positions=epochs.map(jd=>engine.fixstarUt(star.hip,star.radialVelocityKmS,jd,constants.SEFLG_SPEED).xx);
    const refs=epochs.map(jd=>{try{const r=swiss.swe_fixstar_ut(row.name,jd,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);return r&&!r.error&&(r.rflag&swiss.SEFLG_JPLEPH)?r:null;}catch{return null;}});
    const warning=classifyCompatibility(row.name,positions,refs);
    star.lookupWarnings=(star.lookupWarnings??[]).filter(x=>fold(x.name)!==fold(row.name));
    if(warning){star.lookupWarnings.push(warning);warnings.push({hip:star.hip,...warning});}
    if(!star.lookupWarnings.length)delete star.lookupWarnings;
  }
  const output={...manifest,sourceHashes:{...manifest.sourceHashes,iauWarningRefreshSha256:hash(ib)},stars};prepareStarCatalog(output);
  if(await sha(kernelPath)!==kernelSha||await sha(hipPath)!==manifest.sourceHashes.hip2Sha256||await sha(swissPath)!==hash(sb)||await sha(iauReportPath)!==hash(ib)||await sha(manifestPath)!==hash(mb))throw new Error('source changed');
  fs.writeFileSync(outPath,JSON.stringify(output,null,2)+'\n',{flag:'wx'});
  fs.writeFileSync(reportPath,JSON.stringify({schema:'virgostars-swiss-warning-refresh/1',status:'REVIEW_REQUIRED',manifestSha256:hash(mb),iauReportSha256:hash(ib),kernelSha256:kernelSha,warnings},null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({checked:input.added.length,warnings:warnings.length}));
}
main().catch(e=>{console.error(e.message);process.exitCode=1;});
