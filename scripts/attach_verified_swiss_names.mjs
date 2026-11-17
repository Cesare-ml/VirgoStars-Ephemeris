// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Local lookup candidates; exact three-epoch matches are not identity approval.
import {createHash} from 'node:crypto';
import {existsSync,readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {inventorySwissStars} from './inventory_swiss_stars.mjs';
import {findCandidates} from './iau_star_candidates.mjs';
import {prepareStarCatalog} from '../bindings/node/star-catalog.js';
const hash=b=>createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
const rad=Math.PI/180;
function sourceNear(a,b){
  if(a.equinox!=='ICRS'||b.equinox!=='ICRS')return false;
  const read=row=>{
    const f=row.fields,n=f.slice(3,14).map(Number),[h,m,s,d,dm,ds]=n;
    if(!n.every(Number.isFinite)||h<0||h>=24||m<0||m>=60||s<0||s>=60||Math.abs(d)>90||dm<0||dm>=60||ds<0||ds>=60)return null;
    const ra=15*(h+m/60+s/3600)*rad,dec=(f[6].startsWith('-')?-1:1)*(Math.abs(d)+dm/60+ds/3600)*rad;
    return {p:[Math.cos(dec)*Math.cos(ra),Math.cos(dec)*Math.sin(ra),Math.sin(dec)],pmRa:n[6],pmDec:n[7],mag:n[10]};
  };
  const x=read(a),y=read(b);if(!x||!y)return false;
  const cross=[x.p[1]*y.p[2]-x.p[2]*y.p[1],x.p[2]*y.p[0]-x.p[0]*y.p[2],x.p[0]*y.p[1]-x.p[1]*y.p[0]];
  const angle=Math.atan2(Math.hypot(...cross),x.p[0]*y.p[0]+x.p[1]*y.p[1]+x.p[2]*y.p[2])/rad*3600;
  return angle<=0.1&&Math.abs(x.pmRa-y.pmRa)<=5&&Math.abs(x.pmDec-y.pmDec)<=5&&Math.abs(x.mag-y.mag)<=0.1;
}

export function attachVerifiedNames({manifestBytes,swissBytes,iauBytes,gapBytes}) {
  const manifest=JSON.parse(manifestBytes),catalog=prepareStarCatalog(manifest),gap=JSON.parse(gapBytes);
  const swissHash=hash(swissBytes),iauHash=hash(iauBytes);
  if(manifest.sourceHashes?.swissCatalogSha256!==swissHash || manifest.sourceHashes?.iauSha256!==iauHash
    || gap.schema!=='virgostars-swiss-name-gap-audit/1'||gap.status!=='REVIEW_REQUIRED'
    || gap.manifestSha256!==hash(manifestBytes)||gap.swissCatalogSha256!==swissHash
    || gap.hip2Sha256!==manifest.sourceHashes.hip2Sha256 || gap.thresholdArcsec!==1
    || !Array.isArray(gap.epochs)||gap.epochs.map(x=>x.year).join(',')!=='1950,2000,2024'
    || !Array.isArray(gap.results)) throw new Error('source hash or diagnostic mismatch');
  const inventory=inventorySwissStars(swissBytes.toString('utf8')),lines=swissBytes.toString('utf8').split(/\r?\n/);
  const groups=new Map(),byName=new Map();
  for(const entry of inventory.entries){
    const fields=lines[entry.line-1].split('#',1)[0].split(',').map(s=>s.trim());
    const row={...entry,fields,signature:JSON.stringify(fields.slice(2,14))};
    const key=fold(entry.designation), g=groups.get(key)??[];g.push(row);groups.set(key,g);
    if(entry.name){const n=fold(entry.name),v=byName.get(n)??[];v.push(row);byName.set(n,v);}
  }
  const candidates=gap.results.filter(r=>r.status==='EXISTING_POSITION_REVIEW');
  if(candidates.some(r=>typeof r.name!=='string'||!Array.isArray(r.matches)||r.matches.length!==1)
    || new Set(candidates.map(r=>fold(r.name))).size!==candidates.length)throw new Error('invalid diagnostic candidates');
  const iau=candidates.length?findCandidates(iauBytes.toString('utf8'),candidates.map(r=>r.name),'https://www.iau.org/').results:[];
  const iauMap=new Map(iau.map(r=>[fold(r.requestedName),r]));
  const stars=new Map(catalog.stars.map(s=>[s.hip,s]));const additions=[],withheld=[];
  for(const lead of candidates){
    const star=stars.get(lead.matches[0].hip),rows=byName.get(fold(lead.name))??[];
    const designations=new Set(rows.map(r=>fold(r.designation)));
    const iauRow=iauMap.get(fold(lead.name));
    let reason=null;
    if(!Number.isSafeInteger(lead.matches[0].hip))reason='DIRECT_SOURCE_REVIEW_REQUIRED';
    else if(!star || star.name!==lead.matches[0].name)reason='DIAGNOSTIC_STAR_MISMATCH';
    else {
      const warned=new Set((star.lookupWarnings??[]).map(w=>fold(w.name)));
      const anchors=[star.name,...star.aliases].filter(n=>!n.startsWith(',')&&!warned.has(fold(n)))
        .flatMap(n=>byName.get(fold(n))??[]);
      const physicallyConsistent=rows.length>0 && rows.every(row=>anchors.some(anchor=>sourceNear(row,anchor)));
      if(iauRow.candidates.length && (iauRow.candidates.length!==1 || iauRow.candidates[0].hip!==star.hip))reason='IAU_CONFLICT_OR_AMBIGUITY';
      else if(designations.size!==1 && !physicallyConsistent)reason='MULTIPLE_DESIGNATIONS';
      if(!reason && designations.size===1){
      const designation=[...designations][0],group=groups.get(designation);
      const starNames=new Set([star.name,...star.aliases].map(fold));
      const candidateSignatures=new Set(rows.map(r=>r.signature));
      // A mixed designation group is allowed only when the new name's exact
      // source row agrees with an existing *named* lookup for this star.
      // A comma-only designation would otherwise match its own candidate row.
      const namedAnchors=group.filter(r=>r.name && starNames.has(fold(r.name)));
      if(!starNames.has(','+designation)&&!physicallyConsistent)reason='DESIGNATION_NOT_ATTACHED';
      else if(candidateSignatures.size!==1 || (new Set(group.map(r=>r.signature)).size!==1
        && !namedAnchors.some(r=>candidateSignatures.has(r.signature)))){
        if(!physicallyConsistent)reason='DIFFERENT_ASTROMETRY';
      }
      }
    }
    if(reason){withheld.push({name:lead.name,hip:star?.hip??null,reason});continue;}
    additions.push({alias:lead.name,hip:star.hip,canonicalName:star.name,designation:[...designations][0]});
  }
  const byHip=new Map();
  for(const a of additions){const list=byHip.get(a.hip)??[];list.push(a.alias);byHip.set(a.hip,list);}
  const output={...manifest,reviewer:`${manifest.reviewer}; Swiss exact-lookup name candidates require review`,
    sourceHashes:{...manifest.sourceHashes,nameGapAuditSha256:hash(gapBytes)},
    stars:manifest.stars.map(s=>byHip.has(s.hip)?{...s,aliases:[...s.aliases,...byHip.get(s.hip)],
      nameLookupSource:`Swiss exact lookup at 1950, 2000, 2024 and exact or close source astrometry to an existing named object; IAU catalog conflict checked SHA256 ${iauHash}; identity not approved`}:s)};
  prepareStarCatalog(output);
  return {manifest:output,report:{schema:'virgostars-swiss-exact-name-candidates/1',status:'REVIEW_REQUIRED',
    manifestSha256:hash(manifestBytes),gapSha256:hash(gapBytes),swissSha256:swissHash,iauSha256:iauHash,additions,withheld}};
}
function main(){
  const [manifestPath,swissPath,iauPath,gapPath,outPath,reportPath,...extra]=process.argv.slice(2);
  const paths=[manifestPath,swissPath,iauPath,gapPath,outPath,reportPath];
  if(!paths.every(Boolean)||extra.length||new Set(paths.map(p=>resolve(p))).size!==paths.length||existsSync(outPath)||existsSync(reportPath))throw new Error('usage: <manifest> <sefstars> <iau> <name-gap-report> <new-manifest> <new-report>');
  const inputs=paths.slice(0,4).map(p=>readFileSync(p));
  if(inputs.some(b=>b.length>8e6))throw new Error('input too large');
  const result=attachVerifiedNames({manifestBytes:inputs[0],swissBytes:inputs[1],iauBytes:inputs[2],gapBytes:inputs[3]});
  if(paths.slice(0,4).some((p,i)=>hash(readFileSync(p))!==hash(inputs[i])))throw new Error('input changed');
  writeFileSync(outPath,JSON.stringify(result.manifest,null,2)+'\n',{flag:'wx'});
  writeFileSync(reportPath,JSON.stringify(result.report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({added:result.report.additions.length,withheld:result.report.withheld.length}));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)try{main();}catch(e){console.error(e.message);process.exitCode=1;}
