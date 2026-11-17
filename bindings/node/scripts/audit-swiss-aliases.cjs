'use strict';
// SPDX-License-Identifier: CPAL-1.0. Experimental Swiss lookup comparison only.
// Local caller-owned inputs; never redistributes reference data.
const fs = require('node:fs');
const crypto = require('node:crypto');
const path = require('node:path');
const { Engine, prepareStarCatalog, julday, constants } = require('../index.js');
async function digest(file) { const h = crypto.createHash('sha256'); for await (const chunk of fs.createReadStream(file)) h.update(chunk); return h.digest('hex'); }
const circular = (a, b) => Math.abs((((a-b)%360)+540)%360-180)*3600;
async function main() {
  const [manifestPath, hipPath, kernelPath, kernelSha, swissPackage, swissCatalog, outputPath, ...rest] = process.argv.slice(2);
  if (![manifestPath, hipPath, kernelPath, kernelSha, swissPackage, swissCatalog, outputPath].every(Boolean)
      || !/^[a-f0-9]{64}$/.test(kernelSha) || rest.length) throw new Error('usage: <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <new-report>');
  if (fs.existsSync(outputPath)) throw new Error('output exists');
  if (path.basename(swissCatalog).toLowerCase() !== 'sefstars.txt' || path.resolve(path.dirname(swissCatalog)) !== path.resolve(path.dirname(kernelPath))) throw new Error('sefstars.txt must be in Swiss ephemeris directory');
  const bytes = fs.readFileSync(manifestPath); const manifest = JSON.parse(bytes); const catalog = prepareStarCatalog(manifest);
  if (await digest(kernelPath) !== kernelSha || await digest(hipPath) !== manifest.sourceHashes?.hip2Sha256
      || await digest(swissCatalog) !== manifest.sourceHashes?.swissCatalogSha256) throw new Error('source hash mismatch');
  const engine = new Engine(path.resolve(kernelPath)); engine.loadHipparcos(path.resolve(hipPath), catalog.hips, catalog.qualityPolicy);
  const swiss = require(path.resolve(swissPackage)); swiss.swe_set_ephe_path(path.dirname(path.resolve(kernelPath))); swiss.swe_set_jpl_file(path.basename(kernelPath));
  const epochs = [[1950,5,15],[2000,1,1],[2024,3,20]].map(([y,m,d]) => ({ year:y, jd:julday(y,m,d,12,true) }));
  const thresholdArcsec = 1.0; let comparisons = 0; let within = 0; const failures = [];
  for (const star of catalog.stars) {
    if (!star.aliases.length) continue;
    const ours = epochs.map(e => (star.astrometry
      ? engine.fixstarExplicitUt(star.astrometry, star.radialVelocityKmS, e.jd, constants.SEFLG_SPEED)
      : engine.fixstarUt(star.hip, star.radialVelocityKmS, e.jd, constants.SEFLG_SPEED)).xx);
    for (const alias of star.aliases) for (let i = 0; i < epochs.length; i++) {
      let ref;
      try { ref = swiss.swe_fixstar_ut(alias, epochs[i].jd, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED); } catch { ref = null; }
      comparisons++;
      if (!ref || ref.error || !(ref.rflag & swiss.SEFLG_JPLEPH) || !Number.isFinite(ref.longitude) || !Number.isFinite(ref.latitude)) {
        failures.push({ name:star.name, hip:star.hip, alias, year:epochs[i].year, reason:'REFERENCE_UNAVAILABLE' }); continue;
      }
      const lon = circular(ours[i][0], ref.longitude); const lat = Math.abs(ours[i][1]-ref.latitude)*3600;
      if (lon > thresholdArcsec || lat > thresholdArcsec) failures.push({ name:star.name, hip:star.hip, alias, year:epochs[i].year, reason:'POSITION_MISMATCH', longitudeArcsec:lon, latitudeArcsec:lat, referenceName:ref.name });
      else within++;
    }
  }
  if (await digest(kernelPath) !== kernelSha || await digest(hipPath) !== manifest.sourceHashes.hip2Sha256 || await digest(swissCatalog) !== manifest.sourceHashes.swissCatalogSha256) throw new Error('source changed');
  const aliases = catalog.stars.reduce((n,s)=>n+s.aliases.length,0);
  const report = { schema:'virgostars-swiss-alias-audit/1', status:'REVIEW_REQUIRED', manifestSha256:crypto.createHash('sha256').update(bytes).digest('hex'),
    hip2Sha256:manifest.sourceHashes.hip2Sha256, swissCatalogSha256:manifest.sourceHashes.swissCatalogSha256, kernelSha256:kernelSha,
    epochs, thresholdArcsec, aliases, comparisons, within, failures };
  fs.writeFileSync(outputPath, `${JSON.stringify(report,null,2)}\n`, { flag:'wx' });
  console.log(JSON.stringify({aliases, comparisons, within, failedCases:failures.length, failedAliases:new Set(failures.map(f=>`${f.hip}:${f.alias.toLowerCase()}`)).size}));
}
main().catch(error => { console.error(error.message); process.exitCode=1; });
