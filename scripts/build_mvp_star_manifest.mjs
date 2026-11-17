// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-17): prepare the locally tested strict subset.
// External inputs remain external; this is not scientific or legal certification.
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { findCandidates } from './iau_star_candidates.mjs';

const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');

export function buildMvpStarManifest({ candidatesBytes, qualityBytes, iauBytes, hip2Sha256, hip2Path, inspectQuality }) {
  const candidates = JSON.parse(candidatesBytes.toString('utf8'));
  const quality = JSON.parse(qualityBytes.toString('utf8'));
  if (candidates.schema !== 'virgostars-iau-candidates/1' || candidates.status !== 'REVIEW_REQUIRED'
      || !Array.isArray(candidates.results) || !candidates.results.length
      || candidates.catalogSha256 !== sha(iauBytes) || !/^https:\/\//.test(candidates.sourceUrl)
      || quality.schema !== 'virgostars-hip2-quality-audit/1' || quality.status !== 'REVIEW_REQUIRED'
      || !Array.isArray(quality.results) || quality.candidateReportSha256 !== sha(candidatesBytes)
      || quality.hip2Sha256 !== hip2Sha256 || !/^[a-f0-9]{64}$/.test(hip2Sha256)
      || typeof inspectQuality !== 'function') throw new Error('invalid reports or source hash mismatch');
  const replay = findCandidates(iauBytes.toString('utf8'), candidates.results.map((r) => r.requestedName), candidates.sourceUrl);
  if (JSON.stringify(replay.results) !== JSON.stringify(candidates.results)
      || replay.requestedCount !== candidates.requestedCount || replay.exactNameCandidates !== candidates.exactNameCandidates
      || replay.unmatched !== candidates.unmatched || replay.catalogRecordCount !== candidates.catalogRecordCount) {
    throw new Error('candidate report does not reproduce from IAU snapshot');
  }
  const selected = quality.results.filter((item) => item.strictEligible === true);
  const byName = new Map(selected.map((item) => [item.requestedName, item.hip]));
  if (!selected.length || byName.size !== selected.length) throw new Error('invalid strict quality selection');
  const stars = candidates.results.flatMap((row) => {
    if (!byName.has(row.requestedName)) return [];
    if (row.status !== 'CANDIDATE_REVIEW_REQUIRED' || row.candidates.length !== 1) throw new Error('ambiguous selected name');
    const c = row.candidates[0];
    if (c.hip !== byName.get(row.requestedName) || c.magnitudeBand !== 'V' || !Number.isFinite(c.magnitude)) {
      throw new Error('selected HIP or V magnitude mismatch');
    }
    return [{ name: row.requestedName, aliases: [], hip: c.hip,
      identitySource: `Exact historical IAU-CSN name; ${candidates.sourceUrl}; component ${c.component}; snapshot SHA256 ${sha(iauBytes)}`,
      qualityDecision: 'MVP local candidate: strict HIP2 filter, not individual scientific review',
      magnitude: c.magnitude, magnitudeSource: `Historical IAU-CSN snapshot, V band; SHA256 ${sha(iauBytes)}`,
      radialVelocityKmS: null, radialVelocityMissingReason: 'HIP2 has no radial velocity; MVP explicitly assumes zero km/s' }];
  });
  if (stars.length !== selected.length || new Set(stars.map((star) => star.hip)).size !== stars.length) {
    throw new Error('selected names or HIP identifiers are missing or duplicated');
  }
  const checked = inspectQuality(hip2Path, stars.map((star) => star.hip));
  if (!Array.isArray(checked) || checked.length !== stars.length
      || checked.some((row, i) => row.hip !== stars[i].hip || row.strictEligible !== true)) {
    throw new Error('HIP2 quality replay failed');
  }
  return { schema: 'virgostars-star-manifest/1', reviewer: 'VirgoStars MVP local preparation; individual review pending',
    astrometrySource: `Hipparcos new reduction I/311, HIP2 SHA256 ${hip2Sha256}`,
    dataUseDecision: 'Local MVP evaluation requested by project owner; redistribution not determined',
    sourceHashes: { hip2Sha256, iauSha256: sha(iauBytes), candidateReportSha256: sha(candidatesBytes), qualityReportSha256: sha(qualityBytes) },
    qualityPolicy: 'strict', stars };
}

function main() {
  const [candidatesPath, qualityPath, iauPath, hip2Path, outputPath, ...extra] = process.argv.slice(2);
  if (![candidatesPath, qualityPath, iauPath, hip2Path, outputPath].every(Boolean) || extra.length) {
    throw new Error('usage: node scripts/build_mvp_star_manifest.mjs <candidates.json> <quality.json> <iau.txt> <hip2.dat> <new-manifest.json>');
  }
  const paths = [candidatesPath, qualityPath, iauPath, hip2Path];
  if (paths.some((p) => resolve(p) === resolve(outputPath))) throw new Error('output must differ from inputs');
  const limits = [8_000_000, 8_000_000, 4_000_000, 100_000_000];
  const inputs = paths.map((p, i) => {
    const stat = statSync(p);
    if (!stat.isFile() || stat.size > limits[i]) throw new Error('invalid or oversized input');
    const bytes = readFileSync(p);
    if (bytes.length > limits[i]) throw new Error('input exceeded size limit');
    return bytes;
  });
  const { inspectHipparcosQuality, prepareStarCatalog } = createRequire(import.meta.url)('../bindings/node/index.js');
  const manifest = buildMvpStarManifest({ candidatesBytes: inputs[0], qualityBytes: inputs[1], iauBytes: inputs[2],
    hip2Sha256: sha(inputs[3]), hip2Path, inspectQuality: inspectHipparcosQuality });
  prepareStarCatalog(manifest);
  for (let i = 0; i < paths.length; i++) if (sha(readFileSync(paths[i])) !== sha(inputs[i])) throw new Error('input changed during preparation');
  writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, { flag: 'wx' });
  console.log(`${manifest.stars.length} stars prepared for local MVP; zero radial velocity assumed; review remains open.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`MVP star manifest failed: ${error.message}`); process.exitCode = 1; }
}
