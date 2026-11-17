'use strict';
// SPDX-License-Identifier: CPAL-1.0. Physical-identity-first local alias research.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,prepareStarCatalog,julday,constants}=require('../index.js');
const {classifyCompatibility}=require('./attach-iau-physical-identities.cjs');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
const diff=(a,b)=>Math.abs((((a-b)%360)+540)%360-180)*3600;
async function sha(p){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(p))h.update(c);return h.digest('hex');}
async function main(){
  const [manifestPath,hipPath,kernelPath,kernelSha,swissPackage,swissPath,iauPath,outPath,reportPath,...extra]=process.argv.slice(2);
  const files=[manifestPath,hipPath,kernelPath,swissPath,iauPath,outPath,reportPath];
  if(!files.every(Boolean)||!swissPackage||!/^[a-f0-9]{64}$/.test(kernelSha||'')||extra.length
    ||new Set(files.map(p=>path.resolve(p))).size!==files.length||fs.existsSync(outPath)||fs.existsSync(reportPath))throw new Error('usage: <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <iau> <new-manifest> <new-report>');
  if(path.basename(swissPath).toLowerCase()!=='sefstars.txt'||path.resolve(path.dirname(swissPath))!==path.resolve(path.dirname(kernelPath)))throw new Error('expected catalog alongside ephemeris');
  const mb=fs.readFileSync(manifestPath),sb=fs.readFileSync(swissPath),ib=fs.readFileSync(iauPath);
  if([mb,sb,ib].some(b=>b.length>8e6))throw new Error('input too large');
  const manifest=JSON.parse(mb),catalog=prepareStarCatalog(manifest);
  if(await sha(kernelPath)!==kernelSha||await sha(hipPath)!==manifest.sourceHashes.hip2Sha256
    ||hash(sb)!==manifest.sourceHashes.swissCatalogSha256||hash(ib)!==manifest.sourceHashes.iauSha256)throw new Error('source mismatch');
  const {inventorySwissStars}=await import(require('node:url').pathToFileURL(path.resolve(__dirname,'../../../scripts/inventory_swiss_stars.mjs')));
  const {findCandidates}=await import(require('node:url').pathToFileURL(path.resolve(__dirname,'../../../scripts/iau_star_candidates.mjs')));
  const inventory=inventorySwissStars(sb.toString('utf8')),lines=sb.toString('utf8').split(/\r?\n/),byName=new Map(),byDesignation=new Map();
  for(const entry of inventory.entries){
    const fields=lines[entry.line-1].split('#',1)[0].split(',').map(s=>s.trim()),row={...entry,signature:JSON.stringify(fields.slice(2,14))};
    const d=fold(entry.designation),group=byDesignation.get(d)??[];group.push(row);byDesignation.set(d,group);
    if(entry.name){const n=fold(entry.name),rows=byName.get(n)??[];rows.push(row);byName.set(n,rows);}
  }
  const allNames=catalog.stars.flatMap(s=>[s.name,...s.aliases]),used=new Set(allNames.map(fold));
  const iauNames=[...new Map([...allNames,...inventory.names.map(x=>x.value)].map(n=>[fold(n),n])).values()];
  if(iauNames.length>2000)throw new Error('too many IAU lookup names');
  const iau=new Map(findCandidates(ib.toString('utf8'),iauNames,'https://www.iau.org/').results.map(r=>[fold(r.requestedName),r.candidates]));
  const engine=new Engine(path.resolve(kernelPath));engine.loadHipparcos(path.resolve(hipPath),catalog.hips,catalog.qualityPolicy);
  const swiss=require(path.resolve(swissPackage));swiss.swe_set_ephe_path(path.dirname(path.resolve(kernelPath)));swiss.swe_set_jpl_file(path.basename(kernelPath));
  const epochs=[[1950,5,15],[2000,1,1],[2024,3,20]].map(([y,m,d])=>julday(y,m,d,12,true));
  const refs=name=>epochs.map(jd=>{try{const r=swiss.swe_fixstar_ut(name,jd,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);return r&&!r.error&&(r.rflag&swiss.SEFLG_JPLEPH)?r:null;}catch{return null;}});
  const stars=manifest.stars.map(s=>({...s,aliases:[...s.aliases],...(s.lookupWarnings?{lookupWarnings:[...s.lookupWarnings]}:{})})),added=[],withheld=[];
  for(const star of stars){
    if(star.hip===undefined)continue;
    const anchors=[star.name,...star.aliases].filter(name=>{const c=iau.get(fold(name));return c?.length===1&&c[0].hip===star.hip;});
    if(!anchors.length)continue;
    let positions;try{positions=epochs.map(jd=>engine.fixstarUt(star.hip,star.radialVelocityKmS,jd,constants.SEFLG_SPEED).xx);}catch{continue;}
    for(const anchor of anchors){
      const reference=refs(anchor);
      if(reference.some((r,i)=>!r||!Number.isFinite(r.longitude)||!Number.isFinite(r.latitude)
        ||diff(positions[i][0],r.longitude)>5||Math.abs(positions[i][1]-r.latitude)*3600>5))continue;
      for(const source of byName.get(fold(anchor))??[]){
        const group=byDesignation.get(fold(source.designation))??[];
        const designation=','+source.designation;
        if(!used.has(fold(designation)) && new Set(group.map(r=>r.signature)).size===1){
          const designationRefs=refs(designation);
          if(designationRefs.every((r,i)=>r&&Number.isFinite(r.longitude)&&Number.isFinite(r.latitude)
            &&diff(positions[i][0],r.longitude)<=5&&Math.abs(positions[i][1]-r.latitude)*3600<=5)){
            const warning=classifyCompatibility(designation,positions,designationRefs);
            star.aliases.push(designation);if(warning){star.lookupWarnings??=[];star.lookupWarnings.push(warning);}
            used.add(fold(designation));added.push({name:designation,hip:star.hip,anchor,warning:warning?.code??null});
          }
        }
        for(const candidate of group){
          if(!candidate.name||candidate.signature!==source.signature||used.has(fold(candidate.name)))continue;
          const rows=byName.get(fold(candidate.name))??[],des=new Set(rows.map(r=>fold(r.designation)));
          const official=iau.get(fold(candidate.name))??[];
          if(des.size!==1||rows.some(r=>r.signature!==source.signature)
            ||(official.length&&(official.length!==1||official[0].hip!==star.hip))){withheld.push({name:candidate.name,hip:star.hip,reason:'AMBIGUOUS_OR_IAU_CONFLICT'});continue;}
          const warning=classifyCompatibility(candidate.name,positions,refs(candidate.name));
          star.aliases.push(candidate.name);if(warning){star.lookupWarnings??=[];star.lookupWarnings.push(warning);}
          used.add(fold(candidate.name));added.push({name:candidate.name,hip:star.hip,anchor,warning:warning?.code??null});
        }
      }
    }
  }
  const output={...manifest,reviewer:`${manifest.reviewer}; source-row aliases anchored to IAU names require review`,stars};prepareStarCatalog(output);
  if(await sha(kernelPath)!==kernelSha||await sha(hipPath)!==manifest.sourceHashes.hip2Sha256||await sha(swissPath)!==hash(sb)||await sha(iauPath)!==hash(ib)||await sha(manifestPath)!==hash(mb))throw new Error('source changed');
  const report={schema:'virgostars-iau-anchor-aliases/1',status:'REVIEW_REQUIRED',manifestSha256:hash(mb),iauSha256:hash(ib),kernelSha256:kernelSha,added,withheld};
  fs.writeFileSync(outPath,JSON.stringify(output,null,2)+'\n',{flag:'wx'});fs.writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({added:added.length,withheld:withheld.length,warnings:added.filter(x=>x.warning).length}));
}
main().catch(e=>{console.error(e.message);process.exitCode=1;});
