'use strict';
// SPDX-License-Identifier: CPAL-1.0. Local research candidates, not approvals.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,prepareStarCatalog,julday,constants}=require('../index.js');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
const lon=(a,b)=>Math.abs((((a-b)%360)+540)%360-180)*3600;
async function sha(p){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(p))h.update(c);return h.digest('hex');}
async function main(){
  const [manifestPath,hipPath,kernelPath,kernelSha,swissPackage,swissPath,iauPath,outPath,reportPath,...extra]=process.argv.slice(2);
  const files=[manifestPath,hipPath,kernelPath,swissPath,iauPath,outPath,reportPath];
  if(!files.every(Boolean)||!swissPackage||!/^[a-f0-9]{64}$/.test(kernelSha||'')||extra.length
      ||new Set(files.map(p=>path.resolve(p))).size!==files.length||fs.existsSync(outPath)||fs.existsSync(reportPath))throw new Error('usage: <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <iau> <new-manifest> <new-report>');
  if(path.basename(swissPath).toLowerCase()!=='sefstars.txt'||path.resolve(path.dirname(swissPath))!==path.resolve(path.dirname(kernelPath)))throw new Error('expected Swiss catalog in ephemeris directory');
  const manifestBytes=fs.readFileSync(manifestPath),swissBytes=fs.readFileSync(swissPath),iauBytes=fs.readFileSync(iauPath);
  if([manifestBytes,swissBytes,iauBytes].some(b=>b.length>8e6))throw new Error('input too large');
  const manifest=JSON.parse(manifestBytes),catalog=prepareStarCatalog(manifest);
  if(await sha(kernelPath)!==kernelSha||await sha(hipPath)!==manifest.sourceHashes.hip2Sha256
    ||hash(swissBytes)!==manifest.sourceHashes.swissCatalogSha256||hash(iauBytes)!==manifest.sourceHashes.iauSha256)throw new Error('source hash mismatch');
  const {inventorySwissStars}=await import(require('node:url').pathToFileURL(path.resolve(__dirname,'../../../scripts/inventory_swiss_stars.mjs')));
  const {findCandidates}=await import(require('node:url').pathToFileURL(path.resolve(__dirname,'../../../scripts/iau_star_candidates.mjs')));
  const inv=inventorySwissStars(swissBytes.toString('utf8')),groups=new Map(),nameDesignations=new Map();
  for(const row of inv.entries){
    const key=fold(row.designation),rows=groups.get(key)??[];rows.push(row);groups.set(key,rows);
    if(row.name){const n=fold(row.name),set=nameDesignations.get(n)??new Set();set.add(key);nameDesignations.set(n,set);}
  }
  const used=new Map();for(const star of catalog.stars)for(const n of [star.name,...star.aliases])used.set(fold(n),star.hip);
  const engine=new Engine(path.resolve(kernelPath));engine.loadHipparcos(path.resolve(hipPath),catalog.hips,catalog.qualityPolicy);
  const swiss=require(path.resolve(swissPackage));swiss.swe_set_ephe_path(path.dirname(path.resolve(kernelPath)));swiss.swe_set_jpl_file(path.basename(kernelPath));
  const epochs=[[1950,5,15],[2000,1,1],[2024,3,20]].map(([y,m,d])=>julday(y,m,d,12,true));
  const refs=name=>epochs.map(jd=>{try{const r=swiss.swe_fixstar_ut(name,jd,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);
    return r&&!r.error&&(r.rflag&swiss.SEFLG_JPLEPH)&&Number.isFinite(r.longitude)&&Number.isFinite(r.latitude)&&typeof r.name==='string'?r:null;}catch{return null;}});
  const added=[],unresolved=[],stars=[];
  for(const star of manifest.stars){
    if(refs(star.name).every(Boolean)){stars.push(star);continue;}
    if(!star.name.startsWith(',')){unresolved.push({name:star.name,hip:star.hip,reason:'NON_DESIGNATION'});stars.push(star);continue;}
    const group=groups.get(fold(star.name.slice(1)))??[], candidates=[...new Map(group.filter(x=>x.name&&nameDesignations.get(fold(x.name)).size===1)
      .map(x=>[fold(x.name),x.name])).values()].filter(n=>!used.has(fold(n))||used.get(fold(n))===star.hip);
    const iau=candidates.length?findCandidates(iauBytes.toString('utf8'),candidates,'https://www.iau.org/').results:[];
    const positions=epochs.map(jd=>engine.fixstarUt(star.hip,star.radialVelocityKmS,jd,constants.SEFLG_SPEED).xx);
    const matches=candidates.filter((name,i)=>{
      const c=iau[i].candidates;if(c.length&&(c.length!==1||c[0].hip!==star.hip))return false;
      const r=refs(name);
      return r.every((x,j)=>x&&fold(x.name.split(',')[0])===fold(name)
        &&lon(positions[j][0],x.longitude)<=1&&Math.abs(positions[j][1]-x.latitude)*3600<=1);
    });
    if(matches.length!==1){unresolved.push({name:star.name,hip:star.hip,reason:'NO_UNIQUE_EXACT_NAME',matches});stars.push(star);continue;}
    const replacement=matches[0];used.delete(fold(star.name));used.set(fold(replacement),star.hip);
    const aliases=star.aliases.filter(x=>fold(x)!==fold(replacement));
    stars.push({...star,name:replacement,aliases,canonicalLookupSource:`Swiss exact name matched at 1950, 2000 and 2024; original designation unavailable; IAU conflict checked; review required`});
    added.push({hip:star.hip,previousName:star.name,name:replacement});
  }
  const output={...manifest,reviewer:`${manifest.reviewer}; canonical lookup repairs need review`,stars};prepareStarCatalog(output);
  if(await sha(kernelPath)!==kernelSha||await sha(hipPath)!==manifest.sourceHashes.hip2Sha256
    ||await sha(swissPath)!==hash(swissBytes)||await sha(iauPath)!==hash(iauBytes)||await sha(manifestPath)!==hash(manifestBytes))throw new Error('input changed');
  const report={schema:'virgostars-swiss-canonical-repairs/1',status:'REVIEW_REQUIRED',manifestSha256:hash(manifestBytes),kernelSha256:kernelSha,added,unresolved};
  fs.writeFileSync(outPath,JSON.stringify(output,null,2)+'\n',{flag:'wx'});fs.writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({repaired:added.length,unresolved:unresolved.length}));
}
main().catch(e=>{console.error(e.message);process.exitCode=1;});
