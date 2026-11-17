// SPDX-License-Identifier: CPAL-1.0. Local diagnostic filtering, not approval.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const require = createRequire(import.meta.url);
const { prepareStarCatalog } = require('../bindings/node/star-catalog.js');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const fold = value => value.trim().toLocaleLowerCase('en-US');

export function filterAliases(manifestBytes, reportBytes) {
  const manifest = JSON.parse(manifestBytes); const report = JSON.parse(reportBytes);
  const catalog = prepareStarCatalog(manifest);
  const aliases = catalog.stars.reduce((n, star) => n + star.aliases.length, 0);
  const dates = [{year:1950,jd:2433417},{year:2000,jd:2451545},{year:2024,jd:2460390}];
  if (report.schema !== 'virgostars-swiss-alias-audit/1' || report.status !== 'REVIEW_REQUIRED'
      || report.manifestSha256 !== sha(manifestBytes) || report.hip2Sha256 !== manifest.sourceHashes?.hip2Sha256
      || report.swissCatalogSha256 !== manifest.sourceHashes?.swissCatalogSha256 || !/^[a-f0-9]{64}$/.test(report.kernelSha256 || '')
      || JSON.stringify(report.epochs) !== JSON.stringify(dates) || report.thresholdArcsec !== 1
      || report.aliases !== aliases || report.comparisons !== aliases*3 || !Array.isArray(report.failures)
      || report.within !== report.comparisons - report.failures.length) throw new Error('invalid alias diagnostic or source mismatch');
  const byHip = new Map(catalog.stars.map(star => [star.hip, star])); const excluded = new Set(); const cases = new Set();
  for (const row of report.failures) {
    const star = byHip.get(row.hip); const alias = typeof row.alias === 'string' ? fold(row.alias) : '';
    const key = `${row.hip}\0${alias}`; const caseKey = `${key}\0${row.year}`;
    if (!star || star.name !== row.name || !star.aliases.some(a => fold(a) === alias) || !dates.some(d => d.year === row.year)
        || cases.has(caseKey) || !['REFERENCE_UNAVAILABLE','POSITION_MISMATCH'].includes(row.reason)) throw new Error('invalid alias failure');
    if (row.reason === 'POSITION_MISMATCH' && (![row.longitudeArcsec,row.latitudeArcsec].every(v => Number.isFinite(v) && v >= 0)
        || Math.max(row.longitudeArcsec,row.latitudeArcsec) <= 1)) throw new Error('invalid alias residual');
    cases.add(caseKey); excluded.add(key);
  }
  const stars = manifest.stars.map(star => ({ ...star, aliases:star.aliases.filter(alias => !excluded.has(`${star.hip}\0${fold(alias)}`)) }));
  const output = { ...manifest, reviewer:`${manifest.reviewer}; sampled Swiss aliases, identity review pending`,
    sourceHashes:{ ...manifest.sourceHashes, aliasSwissDiagnosticSha256:sha(reportBytes), aliasSwissDiagnosticKernelSha256:report.kernelSha256 }, stars };
  prepareStarCatalog(output);
  return { manifest:output, removedAliases:excluded.size };
}
function main() {
  const [manifestPath, reportPath, outputPath, ...extra] = process.argv.slice(2);
  if (!manifestPath || !reportPath || !outputPath || extra.length || new Set([manifestPath,reportPath,outputPath].map(p=>resolve(p))).size !== 3) throw new Error('usage: <manifest> <alias-report> <new-manifest>');
  if (existsSync(outputPath)) throw new Error('output exists');
  const manifest = readFileSync(manifestPath); const report = readFileSync(reportPath);
  if (manifest.length>8_000_000 || report.length>8_000_000) throw new Error('input too large');
  const result = filterAliases(manifest, report);
  if (sha(readFileSync(manifestPath))!==sha(manifest) || sha(readFileSync(reportPath))!==sha(report)) throw new Error('input changed');
  writeFileSync(outputPath, `${JSON.stringify(result.manifest,null,2)}\n`, {flag:'wx'});
  console.log(`${result.manifest.stars.length} candidates; ${result.removedAliases} aliases withheld.`);
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`Alias diagnostic filter failed: ${error.message}`); process.exitCode=1; }
}
