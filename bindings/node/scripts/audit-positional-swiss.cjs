'use strict';
// SPDX-License-Identifier: CPAL-1.0. Local comparison only; no Swiss code/data bundled.
// Comparison with Swiss is compatibility evidence, not independent scientific truth.
const fs = require('node:fs');
const crypto = require('node:crypto');
const path = require('node:path');
const { Engine, prepareStarCatalog, julday, constants } = require('../index.js');
async function digest(file) { const h = crypto.createHash('sha256'); for await (const chunk of fs.createReadStream(file)) h.update(chunk); return h.digest('hex'); }
const circular = (a, b) => Math.abs((((a - b) % 360) + 540) % 360 - 180);
async function main() {
  const [manifestPath, hipPath, kernelPath, expectedKernelSha, swissPackage, catalogPath, outputPath, mode = 'positional', ...extra] = process.argv.slice(2);
  if (![manifestPath, hipPath, kernelPath, expectedKernelSha, swissPackage, catalogPath, outputPath].every(Boolean)
      || !/^[a-f0-9]{64}$/.test(expectedKernelSha) || !['positional', 'all'].includes(mode) || extra.length) throw new Error('usage: <manifest> <hip2> <kernel> <kernel-sha256> <swisseph-package> <sefstars> <new-report> [positional|all]');
  if (fs.existsSync(outputPath)) throw new Error('output exists');
  const manifestBytes = fs.readFileSync(manifestPath); const manifest = JSON.parse(manifestBytes);
  const catalog = prepareStarCatalog(manifest);
  if (await digest(kernelPath) !== expectedKernelSha || await digest(hipPath) !== manifest.sourceHashes?.hip2Sha256
      || await digest(catalogPath) !== manifest.sourceHashes?.swissCatalogSha256) throw new Error('input digest mismatch');
  if (path.basename(catalogPath).toLowerCase() !== 'sefstars.txt'
      || path.resolve(path.dirname(catalogPath)) !== path.resolve(path.dirname(kernelPath))) throw new Error('sefstars.txt must be in Swiss ephemeris directory');
  const stars = mode === 'all' ? manifest.stars : manifest.stars.filter(s => s.identitySource.startsWith('Swiss-NEC position <=1 arcsec'));
  if (!stars.length) throw new Error('no positional candidates');
  const engine = new Engine(path.resolve(kernelPath));
  engine.loadHipparcos(path.resolve(hipPath), catalog.hips, catalog.qualityPolicy);
  const swiss = require(path.resolve(swissPackage));
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernelPath)));
  swiss.swe_set_jpl_file(path.basename(kernelPath));
  const epochs = [[1950, 5, 15], [2000, 1, 1], [2024, 3, 20]].map(([y, m, d]) => ({ year: y, jd: julday(y, m, d, 12, true) }));
  const thresholdArcsec = 1.0; // Predeclared diagnostic bound, not a release criterion.
  let comparisons = 0; let refErrors = 0; let engineErrors = 0; let within = 0;
  let maxLongitudeArcsec = 0; let maxLatitudeArcsec = 0; const outliers = [];
  for (const star of stars) for (const epoch of epochs) {
    let reference;
    try { reference = swiss.swe_fixstar_ut(star.name, epoch.jd, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED); }
    catch { refErrors++; continue; }
    if (!reference || reference.error || !(reference.rflag & swiss.SEFLG_JPLEPH)
      || !Number.isFinite(reference.longitude) || !Number.isFinite(reference.latitude)) { refErrors++; continue; }
    let result;
    try { result = engine.fixstarUt(star.hip, star.radialVelocityKmS, epoch.jd, constants.SEFLG_SPEED); }
    catch { engineErrors++; continue; }
    if (!result.xx.every(Number.isFinite)) { engineErrors++; continue; }
    const longitudeArcsec = circular(result.xx[0], reference.longitude) * 3600;
    const latitudeArcsec = Math.abs(result.xx[1] - reference.latitude) * 3600;
    comparisons++; maxLongitudeArcsec = Math.max(maxLongitudeArcsec, longitudeArcsec); maxLatitudeArcsec = Math.max(maxLatitudeArcsec, latitudeArcsec);
    if (longitudeArcsec <= thresholdArcsec && latitudeArcsec <= thresholdArcsec) within++;
    else outliers.push({ name: star.name, hip: star.hip, year: epoch.year, longitudeArcsec, latitudeArcsec });
  }
  if (await digest(kernelPath) !== expectedKernelSha || await digest(hipPath) !== manifest.sourceHashes.hip2Sha256
      || await digest(catalogPath) !== manifest.sourceHashes.swissCatalogSha256) throw new Error('source changed during comparison');
  const report = { schema: 'virgostars-positional-swiss-diagnostic/1', status: 'REVIEW_REQUIRED',
    manifestSha256: crypto.createHash('sha256').update(manifestBytes).digest('hex'), kernelSha256: expectedKernelSha,
    hip2Sha256: manifest.sourceHashes.hip2Sha256, swissCatalogSha256: manifest.sourceHashes.swissCatalogSha256,
    mode, stars: stars.length, epochs, thresholdArcsec, comparisons, refErrors, engineErrors, within, maxLongitudeArcsec, maxLatitudeArcsec, outliers };
  fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' });
  console.log(JSON.stringify({ stars: report.stars, comparisons, refErrors, engineErrors, within,
    outliers: outliers.length, maxLongitudeArcsec, maxLatitudeArcsec, thresholdArcsec }));
}
main().catch(error => { console.error(error.message); process.exitCode = 1; });
