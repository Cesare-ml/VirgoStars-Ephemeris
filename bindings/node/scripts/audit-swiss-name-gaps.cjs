'use strict';
// SPDX-License-Identifier: CPAL-1.0. Local gap analysis; it approves no identities.
const fs=require('node:fs'); const path=require('node:path'); const crypto=require('node:crypto');
const {Engine,prepareStarCatalog,julday,constants}=require('../index.js');
const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const fold=value=>value.trim().toLocaleLowerCase('en-US');
const lonDiff=(a,b)=>Math.abs((((a-b)%360)+540)%360-180)*3600;
async function digest(file){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(file)) h.update(c);return h.digest('hex');}

function auditNameGaps(stars,names,positions,reference) {
  const used=new Set(stars.flatMap(s=>[s.name,...s.aliases]).map(fold)); const seen=new Set(); const results=[];
  for(const name of names){
    if(typeof name!=='string'||!name.trim()||seen.has(fold(name))) throw new Error('invalid or duplicate name');
    seen.add(fold(name)); if(used.has(fold(name))) continue;
    const refs=reference(name);
    if(!Array.isArray(refs)||refs.length!==3||refs.some(r=>!r||!Number.isFinite(r.longitude)||!Number.isFinite(r.latitude)||typeof r.name!=='string')){
      results.push({name,status:'REFERENCE_UNAVAILABLE'}); continue;
    }
    const returned=refs.map(r=>r.name.split(',')[0].trim());
    if(returned.some(value=>fold(value)!==fold(name))){results.push({name,status:'REFERENCE_PREFIX_OR_NAME_MISMATCH',returnedNames:[...new Set(returned)]});continue;}
    const matches=[];
    for(const star of stars){
      const values=positions.get(star.hip===undefined?`source:${star.sourceId}`:star.hip);
      if(!Array.isArray(values)||values.length!==3||values.some(v=>!Array.isArray(v)||v.length<2||!v.slice(0,2).every(Number.isFinite))) throw new Error('invalid engine position set');
      if(refs.every((r,i)=>lonDiff(values[i][0],r.longitude)<=1&&Math.abs(values[i][1]-r.latitude)*3600<=1))
        matches.push({name:star.name,...(star.hip===undefined?{sourceId:star.sourceId}:{hip:star.hip})});
    }
    results.push({name,status:matches.length===1?'EXISTING_POSITION_REVIEW':matches.length?'AMBIGUOUS_POSITION':'NO_SELECTED_POSITION',matches});
  }
  const counts={};for(const row of results)counts[row.status]=(counts[row.status]??0)+1;
  return {counts,results};
}

async function main(){
  const [manifestPath,hipPath,kernelPath,kernelSha,swissPackage,swissCatalog,output,...rest]=process.argv.slice(2);
  const paths=[manifestPath,hipPath,kernelPath,swissCatalog,output];
  if(!paths.every(Boolean)||!swissPackage||!/^[a-f0-9]{64}$/.test(kernelSha||'')||rest.length||new Set(paths.map(p=>path.resolve(p))).size!==paths.length) throw new Error('usage: <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <new-report>');
  if(fs.existsSync(output))throw new Error('output exists');
  if(path.basename(swissCatalog).toLowerCase()!=='sefstars.txt'||path.resolve(path.dirname(swissCatalog))!==path.resolve(path.dirname(kernelPath)))throw new Error('expected sefstars.txt in kernel directory');
  const bytes=fs.readFileSync(manifestPath),swissBytes=fs.readFileSync(swissCatalog);
  if(bytes.length>8_000_000||swissBytes.length>8_000_000)throw new Error('input too large');
  const manifest=JSON.parse(bytes),catalog=prepareStarCatalog(manifest);
  if(await digest(kernelPath)!==kernelSha||await digest(hipPath)!==manifest.sourceHashes?.hip2Sha256||hash(swissBytes)!==manifest.sourceHashes?.swissCatalogSha256)throw new Error('source hash mismatch');
  const {inventorySwissStars}=await import(require('node:url').pathToFileURL(path.resolve(__dirname,'../../../scripts/inventory_swiss_stars.mjs')).href);
  const inventory=inventorySwissStars(swissBytes.toString('utf8'));
  const engine=new Engine(path.resolve(kernelPath));engine.loadHipparcos(path.resolve(hipPath),catalog.hips,catalog.qualityPolicy);
  const swiss=require(path.resolve(swissPackage));swiss.swe_set_ephe_path(path.dirname(path.resolve(kernelPath)));swiss.swe_set_jpl_file(path.basename(kernelPath));
  const epochs=[[1950,5,15],[2000,1,1],[2024,3,20]].map(([y,m,d])=>({year:y,jd:julday(y,m,d,12,true)}));
  const positions=new Map(catalog.stars.map(star=>[star.hip===undefined?`source:${star.sourceId}`:star.hip,
    epochs.map(e=>(star.astrometry?engine.fixstarExplicitUt(star.astrometry,star.radialVelocityKmS,e.jd,constants.SEFLG_SPEED)
      :engine.fixstarUt(star.hip,star.radialVelocityKmS,e.jd,constants.SEFLG_SPEED)).xx)]));
  const ref=name=>epochs.map(e=>{try{const r=swiss.swe_fixstar_ut(name,e.jd,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);return r&&!r.error&&(r.rflag&swiss.SEFLG_JPLEPH)?r:null;}catch{return null;}});
  const result=auditNameGaps(catalog.stars,inventory.names.map(x=>x.value),positions,ref);
  if(await digest(kernelPath)!==kernelSha||await digest(hipPath)!==manifest.sourceHashes.hip2Sha256||await digest(swissCatalog)!==hash(swissBytes)||hash(fs.readFileSync(manifestPath))!==hash(bytes))throw new Error('source changed');
  const report={schema:'virgostars-swiss-name-gap-audit/1',status:'REVIEW_REQUIRED',manifestSha256:hash(bytes),hip2Sha256:manifest.sourceHashes.hip2Sha256,
    swissCatalogSha256:hash(swissBytes),kernelSha256:kernelSha,epochs,thresholdArcsec:1,...result};
  fs.writeFileSync(output,`${JSON.stringify(report,null,2)}\n`,{flag:'wx'});console.log(JSON.stringify(result.counts));
}
module.exports={auditNameGaps};
if(require.main===module)main().catch(error=>{console.error(`Name gap audit failed: ${error.message}`);process.exitCode=1;});
