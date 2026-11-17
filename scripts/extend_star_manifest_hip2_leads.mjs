// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Local research candidates. Mandatory subsequent Swiss diagnostic and identity review.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { findLeads } from './audit_hip2_swiss_unmatched.mjs';
import { parseCsv } from './extend_star_manifest_nec.mjs';
import { inspectAvailable } from '../bindings/node/scripts/audit-star-quality.mjs';
import { inspectHipparcosQuality } from '../bindings/node/index.js';
import { prepareStarCatalog } from '../bindings/node/star-catalog.js';
const sha=b=>createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
const rad=Math.PI/180;
function separationArcsec(a,b) {
  const u=([ra,dec])=>[Math.cos(dec*rad)*Math.cos(ra*rad),Math.cos(dec*rad)*Math.sin(ra*rad),Math.sin(dec*rad)];
  const x=u(a),y=u(b),cross=[x[1]*y[2]-x[2]*y[1],x[2]*y[0]-x[0]*y[2],x[0]*y[1]-x[1]*y[0]];
  return Math.atan2(Math.hypot(...cross),x[0]*y[0]+x[1]*y[1]+x[2]*y[2])/rad*3600;
}

export function extendHipLeads({manifest,swissText,hipText,necText,hipPath,inspect=inspectHipparcosQuality,selectionPolicy='traditional'}) {
  const catalog=prepareStarCatalog(manifest);
  if (!['traditional','experimental'].includes(catalog.qualityPolicy) || !['traditional','experimental'].includes(selectionPolicy)
      || (catalog.qualityPolicy==='experimental' && selectionPolicy!=='experimental')) throw new Error('explicit compatible base and selection policy required');
  const hashes={swissCatalogSha256:sha(Buffer.from(swissText)),hip2Sha256:sha(Buffer.from(hipText)),necSha256:sha(Buffer.from(necText))};
  for (const [key,value] of Object.entries(hashes)) if (manifest.sourceHashes?.[key]!==value) throw new Error('source hash mismatch');
  const rows=parseCsv(necText.replace(/^\ufeff/,'')); const header=rows.shift();
  if (!header || new Set(header).size!==header.length) throw new Error('bad NEC header');
  const cols=['HIP','Name','RA2000','DE2000','Vmag'].map(x=>header.indexOf(x));
  if (cols.some(x=>x<0)) throw new Error('missing NEC columns');
  const nec=new Map();
  for (const row of rows) {
    if (row.length!==header.length) throw new Error('bad NEC row');
    const match=/^HIP\s+(\d+)$/.exec(row[cols[0]].trim());
    if (!match || !row[cols[2]].trim() || !row[cols[3]].trim() || !row[cols[4]].trim()) continue;
    const [ra,dec,mag]=cols.slice(2).map(i=>Number(row[i]));
    if (![ra,dec,mag].every(Number.isFinite)||ra<0||ra>=360||Math.abs(dec)>90||mag< -5||mag>30) continue;
    const id=Number(match[1]); const entries=nec.get(id)??[];
    entries.push({name:row[cols[1]].trim(),ra,dec,mag}); nec.set(id,entries);
  }
  const result=findLeads(manifest,swissText,hipText);
  const potential=result.leads.filter(x=>x.matches.length===1 && !x.matches[0].alreadySelected && x.matches[0].separationArcsec<=0.05);
  const hips=[...new Set(potential.map(x=>x.matches[0].hip))];
  const inspected=inspectAvailable(hipPath,hips,inspect);
  const quality=new Map(inspected.rows.map(q=>[q.hip,q]));
  if (quality.size!==inspected.rows.length || inspected.rows.length+inspected.missing.length!==hips.length
      || inspected.rows.some(q=>!hips.includes(q.hip) || typeof q.strictEligible!=='boolean' || typeof q.traditionalEligible!=='boolean'
        || (selectionPolicy==='experimental' && (typeof q.experimentalEligible!=='boolean'
          || (q.strictEligible && !q.traditionalEligible) || (q.traditionalEligible && !q.experimentalEligible))))
      || hips.some(h=>!quality.has(h) && !inspected.missing.includes(h))) throw new Error('bad quality results');
  const existingNames=new Set(catalog.stars.flatMap(s=>[s.name,...s.aliases]).map(fold));
  const grouped=new Map();
  for (const lead of potential) {
    const id=lead.matches[0].hip, q=quality.get(id), entries=nec.get(id)??[];
    // Permit a multi-row HIP only if exactly one distinct astrometry/photometry
    // tuple agrees with the Swiss source. Preserve all matching NEC names.
    const compatible=entries.filter(e=>Math.abs(e.mag-lead.swissMagnitude)<=0.1
      && separationArcsec([lead.raDeg,lead.decDeg],[e.ra,e.dec])<=0.1);
    const tuples=new Map();
    for(const item of compatible) {
      const key=JSON.stringify([item.ra,item.dec,item.mag]), prev=tuples.get(key);
      if(prev)prev.names.add(fold(item.name));else tuples.set(key,{...item,names:new Set([fold(item.name)])});
    }
    const eligible=selectionPolicy==='experimental'?q?.experimentalEligible:q?.traditionalEligible;
    if (!eligible || tuples.size!==1) continue;
    const a=grouped.get(id)??[]; a.push({lead,nec:[...tuples.values()][0],q}); grouped.set(id,a);
  }
  const additions=[];
  for (const [hip,group] of grouped) {
    const names=[];
    for (const {lead,nec:record} of group) {
      const designation=','+lead.designation;
      if (!existingNames.has(fold(designation))) { names.push(designation); existingNames.add(fold(designation)); }
      // A proper name is included only when independently assigned to this HIP in NEC.
      for (const name of lead.names) if (record.names.has(fold(name)) && !existingNames.has(fold(name))) {
        names.push(name); existingNames.add(fold(name));
      }
    }
    if (!names.length) continue;
    const {nec:record,q}=group[0];
    const tier=q.strictEligible?'strict':q.traditionalEligible?'traditional':'experimental';
    additions.push({name:names[0],aliases:names.slice(1),hip,qualityTier:tier,
      identitySource:`Swiss-HIP2 J2000 position <=0.05 arcsec; unique NEC tuple within 0.1 arcsec and 0.1 mag; Swiss ${hashes.swissCatalogSha256}; NEC ${hashes.necSha256}; identity review required`,
      qualityDecision:`Local candidate: ${tier} HIP2 filter; Sn ${q.solutionType}, components ${q.components}; five-parameter linear propagation, extra orbital or acceleration terms not applied; no individual scientific review`,
      magnitude:record.mag,magnitudeSource:`NEC Vmag snapshot SHA256 ${hashes.necSha256}; provenance unverified`,
      radialVelocityKmS:null,radialVelocityMissingReason:'HIP2 has no radial velocity; local candidate assumes zero km/s'});
  }
  const output={...manifest,qualityPolicy:selectionPolicy,reviewer:`${manifest.reviewer}; HIP2 positional leads require review`,stars:[...manifest.stars,...additions]};
  prepareStarCatalog(output);
  return {manifest:output,report:{schema:'virgostars-hip2-nec-leads/1',status:'REVIEW_REQUIRED',sourceHashes:hashes,
    positionalLeads:potential.length,qualityEligible:hips.filter(h=>selectionPolicy==='experimental'?quality.get(h)?.experimentalEligible:quality.get(h)?.traditionalEligible).length,selected:additions.map(s=>({hip:s.hip,name:s.name,aliases:s.aliases,qualityTier:s.qualityTier}))}};
}
async function main() {
  const [manifestPath,swissPath,hipPath,necPath,outPath,reportPath,selectionPolicy='traditional',...extra]=process.argv.slice(2);
  const paths=[manifestPath,swissPath,hipPath,necPath,outPath,reportPath];
  if (!paths.every(Boolean)||extra.length||new Set(paths.map(p=>resolve(p))).size!==paths.length||existsSync(outPath)||existsSync(reportPath)) throw new Error('usage: <manifest> <sefstars> <hip2> <nec> <new-manifest> <new-report>');
  const input=paths.slice(0,4).map(p=>readFileSync(p));
  if (input.some((b,i)=>b.length>(i===2?1e8:8e6))) throw new Error('input too large');
  const r=extendHipLeads({manifest:JSON.parse(input[0]),swissText:input[1].toString('utf8'),hipText:input[2].toString('utf8'),necText:input[3].toString('utf8'),hipPath,selectionPolicy});
  if (paths.slice(0,4).some((p,i)=>sha(readFileSync(p))!==sha(input[i]))) throw new Error('source changed');
  writeFileSync(outPath,JSON.stringify(r.manifest,null,2)+'\n',{flag:'wx'});
  writeFileSync(reportPath,JSON.stringify(r.report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({added:r.report.selected.length,qualityEligible:r.report.qualityEligible}));
}
if (process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href) main().catch(e=>{console.error(e.message);process.exitCode=1;});
