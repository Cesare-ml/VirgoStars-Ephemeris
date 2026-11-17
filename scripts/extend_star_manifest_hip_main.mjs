// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Build local research candidates from caller-provided Hipparcos I/239 and I/311.
import {createHash} from 'node:crypto';
import {existsSync,readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {findLeads} from './audit_hip2_swiss_unmatched.mjs';
import {inspectAvailable} from '../bindings/node/scripts/audit-star-quality.mjs';
import {inspectHipparcosQuality} from '../bindings/node/index.js';
import {prepareStarCatalog} from '../bindings/node/star-catalog.js';
const hash=b=>createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
const rad=Math.PI/180,asec=180/Math.PI*3600;
const unit=(ra,dec)=>[Math.cos(dec)*Math.cos(ra),Math.cos(dec)*Math.sin(ra),Math.sin(dec)];
function separation(x,y) {
  const c=[x[1]*y[2]-x[2]*y[1],x[2]*y[0]-x[0]*y[2],x[0]*y[1]-x[1]*y[0]];
  return Math.atan2(Math.hypot(...c),x[0]*y[0]+x[1]*y[1]+x[2]*y[2])*asec;
}
export function extendHipMain({manifest,swissText,hip2Text,hipMainText,hipMainSha256,hipPath,inspect=inspectHipparcosQuality,expectedCount=118218}) {
  const cat=prepareStarCatalog(manifest), hashes={swissCatalogSha256:hash(Buffer.from(swissText)),hip2Sha256:hash(Buffer.from(hip2Text))};
  if(cat.qualityPolicy!=='experimental'||manifest.sourceHashes.swissCatalogSha256!==hashes.swissCatalogSha256
    ||manifest.sourceHashes.hip2Sha256!==hashes.hip2Sha256||!/^[a-f0-9]{64}$/.test(hipMainSha256)
    ||hash(Buffer.from(hipMainText))!==hipMainSha256)throw new Error('policy or source hash mismatch');
  const leads=findLeads(manifest,swissText,hip2Text).leads.filter(x=>x.matches.length===1
    &&!x.matches[0].alreadySelected&&x.matches[0].separationArcsec<=0.05);
  const hips=[...new Set(leads.map(x=>x.matches[0].hip))],wanted=new Set(hips);
  const inspected=inspectAvailable(hipPath,hips,inspect),quality=new Map(inspected.rows.map(x=>[x.hip,x]));
  if(quality.size!==inspected.rows.length||inspected.rows.length+inspected.missing.length!==hips.length
    ||inspected.rows.some(x=>!wanted.has(x.hip)||typeof x.strictEligible!=='boolean'
      ||typeof x.traditionalEligible!=='boolean'||typeof x.experimentalEligible!=='boolean'
      ||(x.strictEligible&&!x.traditionalEligible)||(x.traditionalEligible&&!x.experimentalEligible))
    ||hips.some(h=>!quality.has(h)&&!inspected.missing.includes(h)))throw new Error('invalid quality inspection');
  const hip2=new Map();
  for(const line of hip2Text.split(/\r?\n/)){
    if(!line)continue;
    const id=Number(line.slice(0,6));if(!wanted.has(id))continue;
    const ra=Number(line.slice(15,28)),dec=Number(line.slice(29,42));
    if(!Number.isFinite(ra)||!Number.isFinite(dec)||hip2.has(id))throw new Error('invalid HIP2 position');
    hip2.set(id,unit(ra,dec));
  }
  const photometry=new Map(),seen=new Set();let count=0;
  for(const line of hipMainText.split(/\r?\n/)){
    if(!line)continue;
    if(line.length!==450||line[0]!=='H'||!/^[\x20-\x7e]+$/.test(line))throw new Error('invalid I/239 record');
    const rawId=line.slice(8,14).trim(),id=Number(rawId);
    if(!/^\d+$/.test(rawId)||id<=0||seen.has(id))throw new Error('invalid or repeated I/239 HIP');
    seen.add(id);count++;
    if(!wanted.has(id))continue;
    const raw=[line.slice(41,46),line.slice(51,63),line.slice(64,76)].map(x=>x.trim());
    if(raw.some(x=>!x))continue;
    const [magnitude,ra,dec]=raw.map(Number);
    if(![magnitude,ra,dec].every(Number.isFinite)||magnitude< -5||magnitude>30||ra<0||ra>=360||Math.abs(dec)>90)continue;
    photometry.set(id,{magnitude,vector:unit(ra*rad,dec*rad)});
  }
  if(count!==expectedCount)throw new Error('unexpected I/239 record count');
  const occupied=new Set(cat.stars.flatMap(s=>[s.name,...s.aliases]).map(fold)),grouped=new Map();
  for(const lead of leads){
    const id=lead.matches[0].hip,q=quality.get(id),p=photometry.get(id),pos=hip2.get(id);
    if(!q?.experimentalEligible||!p||!pos||separation(pos,p.vector)>1||Math.abs(p.magnitude-lead.swissMagnitude)>0.1)continue;
    const a=grouped.get(id)??[];a.push({lead,q,p});grouped.set(id,a);
  }
  const additions=[];
  for(const [hip,entries] of grouped){
    const aliases=[];
    for(const {lead} of entries){const value=','+lead.designation,key=fold(value);if(!occupied.has(key)){occupied.add(key);aliases.push(value);}}
    if(!aliases.length)continue;
    const {q,p}=entries[0],tier=q.strictEligible?'strict':q.traditionalEligible?'traditional':'experimental';
    additions.push({name:aliases[0],aliases:aliases.slice(1),hip,qualityTier:tier,
      identitySource:`Swiss-HIP2 J2000 <=0.05 arcsec; HIP2/HIP1 J1991.25 <=1 arcsec; Vmag <=0.1; I/239 SHA256 ${hipMainSha256}; identity review required`,
      qualityDecision:`Local candidate: ${tier} filter, Sn ${q.solutionType}, components ${q.components}; linear propagation ignores additional orbital and acceleration terms`,
      magnitude:p.magnitude,magnitudeSource:`Hipparcos I/239 Vmag; SHA256 ${hipMainSha256}`,
      radialVelocityKmS:null,radialVelocityMissingReason:'HIP2 has no radial velocity; local candidate assumes zero km/s'});
  }
  const output={...manifest,reviewer:`${manifest.reviewer}; I/239 positional/photometric matches require review`,
    sourceHashes:{...manifest.sourceHashes,hipMainSha256},stars:[...manifest.stars,...additions]};
  prepareStarCatalog(output);
  return {manifest:output,report:{schema:'virgostars-hip-main-candidates/1',status:'REVIEW_REQUIRED',hipMainSha256,
    positionalGroups:leads.length,selected:additions.map(s=>({hip:s.hip,name:s.name,qualityTier:s.qualityTier}))}};
}
function main(){
  const [manifestPath,swissPath,hip2Path,hipMainPath,hipMainSha256,outPath,reportPath,...extra]=process.argv.slice(2);
  const paths=[manifestPath,swissPath,hip2Path,hipMainPath,outPath,reportPath];
  if(!paths.every(Boolean)||extra.length||new Set(paths.map(p=>resolve(p))).size!==paths.length||existsSync(outPath)||existsSync(reportPath))throw new Error('usage: <manifest> <sefstars> <hip2> <hip-main> <hip-main-sha256> <new-manifest> <new-report>');
  const bytes=paths.slice(0,4).map(p=>readFileSync(p));
  if(bytes.some((b,i)=>b.length>(i>=2?1e8:8e6)))throw new Error('input too large');
  const result=extendHipMain({manifest:JSON.parse(bytes[0]),swissText:bytes[1].toString('utf8'),hip2Text:bytes[2].toString('utf8'),
    hipMainText:bytes[3].toString('utf8'),hipMainSha256,hipPath:hip2Path});
  if(paths.slice(0,4).some((p,i)=>hash(readFileSync(p))!==hash(bytes[i])))throw new Error('input changed');
  writeFileSync(outPath,JSON.stringify(result.manifest,null,2)+'\n',{flag:'wx'});
  writeFileSync(reportPath,JSON.stringify(result.report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({stars:result.manifest.stars.length,added:result.report.selected.length}));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)try{main();}catch(e){console.error(e.message);process.exitCode=1;}
