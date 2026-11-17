// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Generate local REVIEW LEADS only. Positional proximity does not prove identity.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { inventorySwissStars } from './inventory_swiss_stars.mjs';
import { prepareStarCatalog } from '../bindings/node/star-catalog.js';
const hash=b=>createHash('sha256').update(b).digest('hex');
const fold=s=>s.trim().toLocaleLowerCase('en-US');
const arcsec=180/Math.PI*3600;
const angle=(a,b)=>Math.atan2(Math.hypot(a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]),a[0]*b[0]+a[1]*b[1]+a[2]*b[2])*arcsec;
const unit=(ra,dec)=>[Math.cos(dec)*Math.cos(ra),Math.cos(dec)*Math.sin(ra),Math.sin(dec)];

export function findLeads(manifest,swissText,hipText) {
  prepareStarCatalog(manifest);
  const inventory=inventorySwissStars(swissText);
  const lines=swissText.split(/\r?\n/);
  const usedNames=new Set(manifest.stars.flatMap(s=>[s.name,...s.aliases]).map(fold));
  const usedHips=new Set(manifest.stars.map(s=>s.hip));
  const hip=[];
  for (const line of hipText.split(/\r?\n/)) {
    if (!line) continue;
    if (line.length!==276 || !/^[\x20-\x7e]+$/.test(line)) throw new Error('invalid HIP2 record');
    const v=[[0,6],[15,28],[29,42],[51,59],[60,68]].map(([a,b])=>line.slice(a,b).trim());
    if (v.some(x=>!x) || v.some(x=>!Number.isFinite(Number(x)))) continue;
    const [id,ra,dec,pmRa,pmDec]=v.map(Number);
    if (!Number.isSafeInteger(id) || id<=0 || ra<0 || ra>=2*Math.PI || Math.abs(dec)>Math.PI/2 || Math.abs(Math.cos(dec))<1e-8) continue;
    // HIP2 is ICRS at epoch 1991.25; first-order propagation to J2000.
    const ra2000=ra+pmRa*8.75/(1000*arcsec*Math.cos(dec));
    const dec2000=dec+pmDec*8.75/(1000*arcsec);
    hip.push({hip:id,vector:unit(ra2000,dec2000)});
  }
  const groups=new Map();
  for (const e of inventory.entries) { const k=fold(e.designation), g=groups.get(k)??[]; g.push(e); groups.set(k,g); }
  const leads=[];
  for (const group of groups.values()) {
    if (usedNames.has(','+fold(group[0].designation)) || group.some(e=>e.name&&usedNames.has(fold(e.name))) || group.some(e=>e.equinox!=='ICRS')) continue;
    const fields=group.map(e=>lines[e.line-1].split('#',1)[0].split(',').map(s=>s.trim()));
    if (new Set(fields.map(f=>JSON.stringify(f.slice(2,14)))).size!==1) continue;
    const f=fields[0], v=f.slice(3,9).map(Number);
    const [h,m,s,d,dm,ds]=v;
    if (!v.every(Number.isFinite)||h<0||h>=24||m<0||m>=60||s<0||s>=60||Math.abs(d)>90||dm<0||dm>=60||ds<0||ds>=60) continue;
    const ra=(h+m/60+s/3600)*15*Math.PI/180, dec=(f[6].startsWith('-')?-1:1)*(Math.abs(d)+dm/60+ds/3600)*Math.PI/180;
    const ref=unit(ra,dec); const matches=[];
    for (const item of hip) {
      // Cheap bound before the precise great-circle separation.
      if (Math.abs(item.vector[2]-ref[2])>1/arcsec) continue;
      const separationArcsec=angle(item.vector,ref);
      if (separationArcsec<=1) matches.push({hip:item.hip,separationArcsec,alreadySelected:usedHips.has(item.hip)});
    }
    leads.push({designation:group[0].designation,names:group.map(e=>e.name).filter(Boolean),
      raDeg:ra*180/Math.PI,decDeg:dec*180/Math.PI,swissMagnitude:Number(f[13]),matches});
  }
  return {schema:'virgostars-hip2-unmatched-leads/1',status:'REVIEW_REQUIRED',notes:'ICRS J2000 positional leads only; no magnitude, identity or quality approval',leads};
}

const [manifestPath,swissPath,hipPath,outputPath,...extra]=process.argv.slice(2);
if (process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href) {
  try {
    if (![manifestPath,swissPath,hipPath,outputPath].every(Boolean)||extra.length||new Set([manifestPath,swissPath,hipPath,outputPath].map(p=>resolve(p))).size!==4||existsSync(outputPath)) throw new Error('usage: <manifest> <sefstars> <hip2> <new-report>');
    const input=[manifestPath,swissPath,hipPath].map(p=>readFileSync(p));
    if (input[0].length>8e6||input[1].length>8e6||input[2].length>1e8) throw new Error('input too large');
    const manifest=JSON.parse(input[0]);
    if (manifest.sourceHashes?.swissCatalogSha256!==hash(input[1])||manifest.sourceHashes?.hip2Sha256!==hash(input[2])) throw new Error('source hash mismatch');
    const report=findLeads(manifest,input[1].toString('utf8'),input[2].toString('utf8'));
    report.sourceHashes={manifestSha256:hash(input[0]),swissSha256:hash(input[1]),hip2Sha256:hash(input[2])};
    if ([manifestPath,swissPath,hipPath].some((p,i)=>hash(readFileSync(p))!==hash(input[i]))) throw new Error('source changed');
    writeFileSync(outputPath,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
    console.log(JSON.stringify({groups:report.leads.length,oneMatch:report.leads.filter(x=>x.matches.length===1).length,
      uniqueNew:report.leads.filter(x=>x.matches.length===1&&!x.matches[0].alreadySelected).length}));
  } catch (e) { console.error(e.message); process.exitCode=1; }
}
