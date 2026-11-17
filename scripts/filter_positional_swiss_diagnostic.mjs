// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Prepare a local candidate subset; comparison does not establish identity.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const require = createRequire(import.meta.url);
const { prepareStarCatalog } = require('../bindings/node/star-catalog.js');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');

export function filterByDiagnostic(manifestBytes, reportBytes) {
  const manifest = JSON.parse(manifestBytes); const report = JSON.parse(reportBytes);
  prepareStarCatalog(manifest);
  const all = report.mode === 'all';
  if (report.mode !== undefined && !['all', 'positional'].includes(report.mode)) throw new Error('invalid diagnostic mode');
  const candidates = all ? manifest.stars : manifest.stars.filter(s => s.identitySource.startsWith('Swiss-NEC position <=1 arcsec'));
  if (report.schema !== 'virgostars-positional-swiss-diagnostic/1' || report.status !== 'REVIEW_REQUIRED'
      || report.manifestSha256 !== sha(manifestBytes) || report.hip2Sha256 !== manifest.sourceHashes?.hip2Sha256
      || report.swissCatalogSha256 !== manifest.sourceHashes?.swissCatalogSha256
      || report.stars !== candidates.length || !Array.isArray(report.epochs) || report.epochs.length !== 3
      || JSON.stringify(report.epochs) !== JSON.stringify([{ year: 1950, jd: 2433417 }, { year: 2000, jd: 2451545 }, { year: 2024, jd: 2460390 }])
      || !/^[a-f0-9]{64}$/.test(report.kernelSha256 || '')
      || report.thresholdArcsec !== 1 || report.refErrors !== 0 || report.engineErrors !== 0
      || report.comparisons !== candidates.length * 3 || !Array.isArray(report.outliers)
      || report.within !== report.comparisons - report.outliers.length) throw new Error('invalid diagnostic or source mismatch');
  const names = new Map(candidates.map(s => [s.hip, s.name])); const excluded = new Set(); const cases = new Set();
  for (const item of report.outliers) {
    const key = `${item.hip}:${item.year}`;
    if (names.get(item.hip) !== item.name || !report.epochs.some(epoch => epoch.year === item.year)
        || ![item.longitudeArcsec, item.latitudeArcsec].every(v => Number.isFinite(v) && v >= 0)
        || Math.max(item.longitudeArcsec, item.latitudeArcsec) <= report.thresholdArcsec || cases.has(key)) throw new Error('invalid outlier');
    cases.add(key); excluded.add(item.hip);
  }
  const stars = manifest.stars.filter(s => !excluded.has(s.hip)).map(s =>
    all || s.identitySource.startsWith('Swiss-NEC position <=1 arcsec')
      ? { ...s, qualityDecision: `${s.qualityDecision}; Swiss diagnostic <=1 arcsec on three sampled epochs, not identity approval` } : s);
  const result = { ...manifest, reviewer: `${manifest.reviewer}; sampled Swiss diagnostic, identity review pending`,
    sourceHashes: { ...manifest.sourceHashes, [all ? 'allStarSwissDiagnosticSha256' : 'swissDiagnosticSha256']: sha(reportBytes),
      swissDiagnosticKernelSha256: report.kernelSha256 }, stars };
  prepareStarCatalog(result);
  return { manifest: result, removed: [...excluded] };
}

function main() {
  const [manifestPath, reportPath, outputPath, ...rest] = process.argv.slice(2);
  if (!manifestPath || !reportPath || !outputPath || rest.length || new Set([manifestPath, reportPath, outputPath].map(p => resolve(p))).size !== 3) throw new Error('usage: <manifest> <diagnostic> <new-manifest>');
  if (existsSync(outputPath)) throw new Error('output already exists');
  const manifest = readFileSync(manifestPath); const report = readFileSync(reportPath);
  if (manifest.length > 8_000_000 || report.length > 8_000_000) throw new Error('input exceeds size limit');
  const result = filterByDiagnostic(manifest, report);
  if (sha(readFileSync(manifestPath)) !== sha(manifest) || sha(readFileSync(reportPath)) !== sha(report)) throw new Error('input changed');
  writeFileSync(outputPath, `${JSON.stringify(result.manifest, null, 2)}\n`, { flag: 'wx' });
  console.log(`${result.manifest.stars.length} catalog candidates; ${result.removed.length} positional outlier stars withheld.`);
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`Diagnostic filter failed: ${error.message}`); process.exitCode = 1; }
}
