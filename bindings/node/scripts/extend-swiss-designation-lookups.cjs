'use strict';
// SPDX-License-Identifier: CPAL-1.0. Local Swiss lookup diagnostic, not identity approval.
const fs = require('node:fs');
const crypto = require('node:crypto');
const path = require('node:path');
const { Engine, prepareStarCatalog, julday, constants } = require('../index.js');
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const fold = value => value.trim().toLocaleLowerCase('en-US');
const deltaLon = (a,b) => Math.abs((((a-b)%360)+540)%360-180)*3600;
async function digest(file) { const h=crypto.createHash('sha256'); for await (const chunk of fs.createReadStream(file)) h.update(chunk); return h.digest('hex'); }

/** Match an exact Swiss designation to one unique candidate at every sampled epoch. */
function matchDesignations(stars, designations, positionByHip, referenceByDesignation) {
  const lookup = new Set(stars.flatMap(s=>[s.name,...s.aliases]).map(fold));
  const added = []; const failures = []; const seen = new Set();
  for (const designation of designations) {
    if (typeof designation!=='string' || !designation.trim() || seen.has(fold(designation))) throw new Error('invalid or duplicate designation');
    const alias=`,`+designation.trim(); const key=fold(alias); seen.add(fold(designation));
    if (lookup.has(key)) continue;
    const refs=referenceByDesignation(alias);
    if (!Array.isArray(refs) || refs.length!==3 || refs.some(r=>!r || !Number.isFinite(r.longitude) || !Number.isFinite(r.latitude))) {
      failures.push({alias,reason:'REFERENCE_UNAVAILABLE'}); continue;
    }
    const matches=[];
    for (const star of stars) {
      const points=positionByHip.get(star.hip);
      if (!Array.isArray(points) || points.length!==3 || points.some(p=>!Array.isArray(p) || p.length<2 || !p.slice(0,2).every(Number.isFinite))) throw new Error('invalid engine position set');
      const residuals=refs.map((ref,i)=>({longitudeArcsec:deltaLon(points[i][0],ref.longitude),latitudeArcsec:Math.abs(points[i][1]-ref.latitude)*3600}));
      if (residuals.every(r=>r.longitudeArcsec<=1 && r.latitudeArcsec<=1)) matches.push({star,residuals});
    }
    if (matches.length!==1) { failures.push({alias,reason:matches.length?'AMBIGUOUS_MATCH':'NO_MATCH',matchingStars:matches.length}); continue; }
    const {star,residuals}=matches[0];
    added.push({alias,canonicalName:star.name,hip:star.hip,
      maxLongitudeArcsec:Math.max(...residuals.map(r=>r.longitudeArcsec)),maxLatitudeArcsec:Math.max(...residuals.map(r=>r.latitudeArcsec))});
    lookup.add(key);
  }
  return {added,failures};
}

async function main() {
  const [manifestPath,hipPath,kernelPath,kernelSha,swissPackage,swissCatalog,outputPath,reportPath,...extra]=process.argv.slice(2);
  const files=[manifestPath,hipPath,kernelPath,swissCatalog,outputPath,reportPath];
  if (!files.every(Boolean) || !swissPackage || !/^[a-f0-9]{64}$/.test(kernelSha||'') || extra.length
      || new Set(files.map(p=>path.resolve(p))).size!==files.length) throw new Error('usage: <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <new-manifest> <new-report>');
  if (fs.existsSync(outputPath) || fs.existsSync(reportPath)) throw new Error('output exists');
  if (path.basename(swissCatalog).toLowerCase()!=='sefstars.txt' || path.resolve(path.dirname(swissCatalog))!==path.resolve(path.dirname(kernelPath))) throw new Error('expected sefstars.txt in kernel directory');
  const bytes=fs.readFileSync(manifestPath); if (bytes.length>8_000_000) throw new Error('manifest too large');
  const manifest=JSON.parse(bytes); const catalog=prepareStarCatalog(manifest); const swissBytes=fs.readFileSync(swissCatalog);
  if (swissBytes.length>8_000_000 || await digest(kernelPath)!==kernelSha || await digest(hipPath)!==manifest.sourceHashes?.hip2Sha256
      || hash(swissBytes)!==manifest.sourceHashes?.swissCatalogSha256) throw new Error('source hash mismatch');
  const { inventorySwissStars }=await import(pathToFileUrl(path.resolve(__dirname,'../../../scripts/inventory_swiss_stars.mjs')));
  const inventory=inventorySwissStars(swissBytes.toString('utf8'));
  const engine=new Engine(path.resolve(kernelPath)); engine.loadHipparcos(path.resolve(hipPath),catalog.hips,catalog.qualityPolicy);
  const swiss=require(path.resolve(swissPackage)); swiss.swe_set_ephe_path(path.dirname(path.resolve(kernelPath))); swiss.swe_set_jpl_file(path.basename(kernelPath));
  const epochs=[[1950,5,15],[2000,1,1],[2024,3,20]].map(([y,m,d])=>({year:y,jd:julday(y,m,d,12,true)}));
  const positions=new Map(catalog.stars.map(star=>[star.hip,epochs.map(e=>engine.fixstarUt(star.hip,star.radialVelocityKmS,e.jd,constants.SEFLG_SPEED).xx)]));
  const ref=alias=>epochs.map(e=>{ try { const r=swiss.swe_fixstar_ut(alias,e.jd,swiss.SEFLG_JPLEPH|swiss.SEFLG_SPEED);
    return r && !r.error && (r.rflag&swiss.SEFLG_JPLEPH) ? r : null; } catch { return null; } });
  const result=matchDesignations(catalog.stars,inventory.designations.map(v=>v.value),positions,ref);
  const byHip=new Map(); for(const row of result.added) { const a=byHip.get(row.hip)??[]; a.push(row.alias); byHip.set(row.hip,a); }
  const stars=manifest.stars.map(star=>byHip.has(star.hip) ? {...star,aliases:[...star.aliases,...byHip.get(star.hip)],
    lookupAliasSource:`Swiss designation lookup compared against HIP2 positions on 1950, 2000, 2024; Swiss SHA256 ${hash(swissBytes)}; review required`} : star);
  const output={...manifest,reviewer:`${manifest.reviewer}; Swiss lookup matches require review`,
    sourceHashes:{...manifest.sourceHashes,designationLookupBaseSha256:hash(bytes),designationLookupKernelSha256:kernelSha},stars};
  prepareStarCatalog(output);
  if (await digest(kernelPath)!==kernelSha || await digest(hipPath)!==manifest.sourceHashes.hip2Sha256 || await digest(swissCatalog)!==hash(swissBytes)
      || hash(fs.readFileSync(manifestPath))!==hash(bytes)) throw new Error('source changed');
  const report={schema:'virgostars-swiss-designation-lookups/1',status:'REVIEW_REQUIRED',baseManifestSha256:hash(bytes),
    swissCatalogSha256:hash(swissBytes),hip2Sha256:manifest.sourceHashes.hip2Sha256,kernelSha256:kernelSha,epochs,thresholdArcsec:1,...result};
  fs.writeFileSync(outputPath,`${JSON.stringify(output,null,2)}\n`,{flag:'wx'});
  fs.writeFileSync(reportPath,`${JSON.stringify(report,null,2)}\n`,{flag:'wx'});
  console.log(JSON.stringify({stars:stars.length,addedAliases:result.added.length,unmatched:result.failures.length,
    unavailable:result.failures.filter(r=>r.reason==='REFERENCE_UNAVAILABLE').length,ambiguous:result.failures.filter(r=>r.reason==='AMBIGUOUS_MATCH').length}));
}
function pathToFileUrl(p) { return require('node:url').pathToFileURL(p).href; }
module.exports={matchDesignations};
if (require.main===module) main().catch(error=>{console.error(`Designation lookup extension failed: ${error.message}`);process.exitCode=1;});
