'use strict';
// SPDX-License-Identifier: CPAL-1.0. Local catalog candidates; no external data bundled.
// IAU identity takes precedence over Swiss lookup; discrepancies are recorded per name.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {Engine,prepareStarCatalog,julday,constants,inspectHipparcosQuality}=require('../index.js');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
const lon=(a,b)=>Math.abs((((a-b)%360)+540)%360-180)*3600;
async function sha(p){const h=crypto.createHash('sha256');for await(const c of fs.createReadStream(p))h.update(c);return h.digest('hex');}

function classifyCompatibility(name,positions,refs){
  if(!Array.isArray(refs)||refs.length!==3||refs.some(r=>!r||!Number.isFinite(r.longitude)||!Number.isFinite(r.latitude)||typeof r.name!=='string'))
    return {name,code:'SWISS_LOOKUP_UNAVAILABLE',detail:'Swiss lookup unavailable for one or more diagnostic epochs.'};
  if(!Array.isArray(positions)||positions.length!==3||positions.some(v=>!Array.isArray(v)||!v.slice(0,2).every(Number.isFinite)))throw new Error('invalid engine positions');
  const maxLongitudeArcsec=Math.max(...refs.map((r,i)=>lon(positions[i][0],r.longitude)));
  const maxLatitudeArcsec=Math.max(...refs.map((r,i)=>Math.abs(positions[i][1]-r.latitude)*3600));
  const returned=[...new Set(refs.map(r=>r.name.split(',')[0].trim()))];
  if(Math.max(maxLongitudeArcsec,maxLatitudeArcsec)>1)
    return {name,code:'SWISS_POSITION_MISMATCH',detail:`Swiss lookup returned ${returned.join(' / ')}; position differs by up to ${Math.max(maxLongitudeArcsec,maxLatitudeArcsec).toFixed(3)} arcsec on 1950/2000/2024 samples.`};
  if(!name.startsWith(',') && returned.some(n=>fold(n)!==fold(name)))
    return {name,code:'SWISS_RETURNED_DIFFERENT_NAME',detail:`Swiss lookup returned ${returned.join(' / ')}; sampled positions agree within 1 arcsec.`};
  return null;
}

async function main(){
  const [manifestPath,hipPath,kernelPath,kernelSha,swissPackage,swissPath,iauPath,outPath,reportPath,...extra]=process.argv.slice(2);
  const files=[manifestPath,hipPath,kernelPath,swissPath,iauPath,outPath,reportPath];
  if(!files.every(Boolean)||!swissPackage||!/^[a-f0-9]{64}$/.test(kernelSha||'')||extra.length
      ||new Set(files.map(p=>path.resolve(p))).size!==files.length||fs.existsSync(outPath)||fs.existsSync(reportPath))throw new Error('usage: <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <iau> <new-manifest> <new-report>');
  if(path.basename(swissPath).toLowerCase()!=='sefstars.txt'||path.resolve(path.dirname(swissPath))!==path.resolve(path.dirname(kernelPath)))throw new Error('expected Swiss catalog alongside ephemeris');
  const mb=fs.readFileSync(manifestPath),sb=fs.readFileSync(swissPath),ib=fs.readFileSync(iauPath);
  if([mb,sb,ib].some(b=>b.length>8e6))throw new Error('input too large');
  const manifest=JSON.parse(mb),catalog=prepareStarCatalog(manifest);
  if(catalog.qualityPolicy!=='experimental'||await sha(kernelPath)!==kernelSha||await sha(hipPath)!==manifest.sourceHashes.hip2Sha256
      ||hash(sb)!==manifest.sourceHashes.swissCatalogSha256||hash(ib)!==manifest.sourceHashes.iauSha256)throw new Error('source hash or policy mismatch');
  const {inventorySwissStars}=await import(require('node:url').pathToFileURL(path.resolve(__dirname,'../../../scripts/inventory_swiss_stars.mjs')));
  const {findCandidates}=await import(require('node:url').pathToFileURL(path.resolve(__dirname,'../../../scripts/iau_star_candidates.mjs')));
  const {inspectAvailable}=await import(require('node:url').pathToFileURL(path.resolve(__dirname,'audit-star-quality.mjs')));
  const used=new Set(catalog.stars.flatMap(s=>[s.name,...s.aliases]).map(fold));
  const missing=inventorySwissStars(sb.toString('utf8')).names.map(x=>x.value).filter(name=>!used.has(fold(name)));
  const iau=missing.length?findCandidates(ib.toString('utf8'),missing,'https://www.iau.org/public/themes/naming_stars/').results:[];
  const candidates=iau.filter(r=>r.candidates.length===1&&r.candidates[0].hip!==null&&r.candidates[0].magnitudeBand==='V'&&Number.isFinite(r.candidates[0].magnitude));
  const hips=[...new Set(candidates.map(r=>r.candidates[0].hip))], quality=inspectAvailable(hipPath,hips,inspectHipparcosQuality);
  const q=new Map(quality.rows.map(x=>[x.hip,x]));
  if(q.size!==quality.rows.length||quality.rows.length+quality.missing.length!==hips.length
      ||hips.some(h=>!q.has(h)&&!quality.missing.includes(h))
      ||quality.rows.some(x=>!hips.includes(x.hip)||typeof x.experimentalEligible!=='boolean'||typeof x.traditionalEligible!=='boolean'||typeof x.strictEligible!=='boolean'))throw new Error('invalid quality audit');
  const eligible=candidates.filter(r=>q.get(r.candidates[0].hip)?.experimentalEligible);
  const engine=new Engine(path.resolve(kernelPath));engine.loadHipparcos(path.resolve(hipPath),[...new Set([...catalog.hips,...eligible.map(r=>r.candidates[0].hip)])],catalog.qualityPolicy);
  const swiss=require(path.resolve(swissPackage));swiss.swe_set_ephe_path(path.dirname(path.resolve(kernelPath)));swiss.swe_set_jpl_file(path.basename(kernelPath));
  const epochs=[[1950,5,15],[2000,1,1],[2024,3,20]].map(([y,m,d])=>julday(y,m,d,12,true));
  const added=[],withheld=[],stars=manifest.stars.map(s=>({...s,aliases:[...s.aliases],...(s.lookupWarnings?{lookupWarnings:[...s.lookupWarnings]}:{})}));
  const mutable=new Map(stars.map(s=>[s.hip,s]));
  for(const row of candidates){
    const candidate=row.candidates[0],qual=q.get(candidate.hip);
    if(!qual?.experimentalEligible){withheld.push({name:row.requestedName,hip:candidate.hip,reason:'HIP2_UNAVAILABLE_OR_INELIGIBLE'});continue;}
    let star=mutable.get(candidate.hip),created=false;
    if(!star){
      const tier=qual.strictEligible?'strict':qual.traditionalEligible?'traditional':'experimental';
      star={name:row.requestedName,aliases:[],hip:candidate.hip,qualityTier:tier,
        identitySource:`IAU CSN exact name and HIP; local snapshot SHA256 ${hash(ib)}; user selected physical identity; review required`,
        qualityDecision:`${tier} HIP2 filter; Sn ${qual.solutionType}, components ${qual.components}; only linear five-parameter propagation`,
        magnitude:candidate.magnitude,magnitudeSource:`IAU CSN V magnitude; local snapshot SHA256 ${hash(ib)}`,
        radialVelocityKmS:null,radialVelocityMissingReason:'HIP2 has no radial velocity; local candidate assumes zero km/s'};
      created=true;
    }
    let positions;
    try{positions=epochs.map(jd=>engine.fixstarUt(star.hip,star.radialVelocityKmS,jd,constants.SEFLG_SPEED).xx);}
    catch(e){withheld.push({name:row.requestedName,hip:candidate.hip,reason:'ENGINE_ERROR',code:e.code});continue;}
    const refs=epochs.map(jd=>{try{const r=swiss.swe_fixstar_ut(row.requestedName,jd,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);return r&&!r.error&&(r.rflag&swiss.SEFLG_JPLEPH)?r:null;}catch{return null;}});
    const warning=classifyCompatibility(row.requestedName,positions,refs);
    if(created){stars.push(star);mutable.set(star.hip,star);}
    else star.aliases.push(row.requestedName);
    if(warning){star.lookupWarnings??=[];star.lookupWarnings.push(warning);}
    used.add(fold(row.requestedName));
    added.push({name:row.requestedName,hip:star.hip,newStar:created,warning:warning?.code??null});
  }
  const output={...manifest,reviewer:`${manifest.reviewer}; IAU physical identity chosen over conflicting Swiss lookup; individual review pending`,
    sourceHashes:{...manifest.sourceHashes,iauPhysicalKernelSha256:kernelSha},stars};
  prepareStarCatalog(output);
  if(await sha(kernelPath)!==kernelSha||await sha(hipPath)!==manifest.sourceHashes.hip2Sha256||await sha(swissPath)!==hash(sb)||await sha(iauPath)!==hash(ib)||await sha(manifestPath)!==hash(mb))throw new Error('source changed');
  const report={schema:'virgostars-iau-physical-identities/1',status:'REVIEW_REQUIRED',manifestSha256:hash(mb),iauSha256:hash(ib),kernelSha256:kernelSha,added,withheld};
  fs.writeFileSync(outPath,JSON.stringify(output,null,2)+'\n',{flag:'wx'});fs.writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({namesAdded:added.length,newStars:added.filter(x=>x.newStar).length,warnings:added.filter(x=>x.warning).length,withheld:withheld.length}));
}
module.exports={classifyCompatibility};
if(require.main===module)main().catch(e=>{console.error(e.message);process.exitCode=1;});
