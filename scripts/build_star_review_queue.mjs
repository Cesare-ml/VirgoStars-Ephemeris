// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Offline review workbench. No entry, source or scientific decision is approved.
// Modification (2026-11-17): assemble and verify existing IAU/SIMBAD leads.
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { findCandidates } from './iau_star_candidates.mjs';

const digest = (data) => createHash('sha256').update(data).digest('hex');
const fold = (s) => s.trim().toLocaleLowerCase('en-US');
const json = (data) => JSON.parse(data.toString('utf8'));
const qualityFields = ['hip', 'solutionType', 'components', 'strictEligible', 'traditionalEligible'];

// Intentionally narrow parser for the four-column SIMBAD response shape saved
// by this project. Other CSV quoting/layouts are rejected, never guessed.
function parseSimbad(text) {
  const lines = text.split(/\r?\n/);
  if (lines.shift() !== 'id,main_id,otype,hip_id') throw new Error('unexpected SIMBAD CSV header');
  return lines.filter((line) => line.trim()).map((line) => {
    const m = /^"([^"\r\n]+)","([^"\r\n]+)","([^"\r\n]+)",(?:"HIP (\d+)"|)$/.exec(line);
    if (!m) throw new Error('unsupported SIMBAD CSV row');
    const requestedName = m[1].startsWith('NAME ') ? m[1].slice(5) : m[1];
    const hip = m[4] === undefined ? null : Number(m[4]);
    if (!requestedName.trim() || (hip !== null && (!Number.isSafeInteger(hip) || hip <= 0 || hip > 0xffffffff))) {
      throw new Error('invalid SIMBAD candidate identity');
    }
    return { requestedName: requestedName.trim(), sourceId: m[1], mainId: m[2], objectType: m[3], hip };
  });
}

function checkQuality(selections, reportRows, inspect, hip2Path) {
  if (!Array.isArray(reportRows)) throw new Error('invalid quality rows');
  const hips = selections.map((entry) => entry.hip);
  if (new Set(hips).size !== hips.length) throw new Error('duplicate HIP candidates require manual resolution');
  if (reportRows.length !== selections.length) throw new Error('quality coverage mismatch');
  const byName = new Map();
  for (const row of reportRows) {
    if (!row || typeof row.requestedName !== 'string' || !row.requestedName.trim() || byName.has(fold(row.requestedName))) {
      throw new Error('invalid or duplicate quality name');
    }
    byName.set(fold(row.requestedName), row);
  }
  const actual = hips.length ? inspect(hip2Path, hips) : [];
  if (!Array.isArray(actual) || actual.length !== hips.length) throw new Error('HIP2 quality replay incomplete');
  const result = new Map();
  for (let i = 0; i < selections.length; i++) {
    const entry = selections[i];
    const claimed = byName.get(fold(entry.requestedName));
    const measured = actual[i];
    if (!claimed || !measured || qualityFields.some((field) => claimed[field] !== measured[field])
        || claimed.hip !== entry.hip || typeof claimed.strictEligible !== 'boolean'
        || typeof claimed.traditionalEligible !== 'boolean') {
      throw new Error('quality report differs from HIP2 replay');
    }
    result.set(fold(entry.requestedName), Object.fromEntries(qualityFields.map((field) => [field, claimed[field]])));
  }
  return result;
}

export function buildReviewQueue({ namesBytes, iauBytes, candidateBytes, qualityBytes, hip2Sha256,
  hip2Path, inspectQuality, simbadBytes = null, simbadQualityBytes = null }) {
  if (typeof inspectQuality !== 'function' || !/^[a-f0-9]{64}$/.test(hip2Sha256)) throw new Error('HIP2 inspection and SHA-256 are required');
  if ((simbadBytes === null) !== (simbadQualityBytes === null)) throw new Error('SIMBAD response and quality must be supplied together');
  const names = json(namesBytes);
  const candidate = json(candidateBytes);
  const quality = json(qualityBytes);
  if (candidate.schema !== 'virgostars-iau-candidates/1' || candidate.status !== 'REVIEW_REQUIRED'
      || quality.schema !== 'virgostars-hip2-quality-audit/1' || quality.status !== 'REVIEW_REQUIRED'
      || candidate.catalogSha256 !== digest(iauBytes) || candidate.requestedNamesSha256 !== digest(namesBytes)
      || quality.candidateReportSha256 !== digest(candidateBytes) || quality.hip2Sha256 !== hip2Sha256) {
    throw new Error('IAU source or report hash mismatch');
  }
  const { catalogSha256, requestedNamesSha256, ...candidateBody } = candidate;
  if (!isDeepStrictEqual(findCandidates(iauBytes.toString('utf8'), names, candidate.sourceUrl), candidateBody)) {
    throw new Error('candidate report does not reproduce from source and requested names');
  }
  const iauSelections = candidate.results.flatMap((row) =>
    row.status === 'CANDIDATE_REVIEW_REQUIRED' && row.candidates[0].hip !== null
      ? [{ requestedName: row.requestedName, hip: row.candidates[0].hip }] : []);
  const iauQuality = checkQuality(iauSelections, quality.results, inspectQuality, hip2Path);
  const entries = candidate.results.map((row) => ({ requestedName: row.requestedName,
    iauStatus: row.status, iauCandidates: row.candidates.map((item) => ({ ...item,
      hip2Quality: row.candidates.length === 1 ? iauQuality.get(fold(row.requestedName)) ?? null : null })),
    simbadCandidates: [], decisions: { identity: null, quality: null, magnitude: null,
      radialVelocity: null, aliases: null, sourceRights: null } }));
  const index = new Map(entries.map((entry) => [fold(entry.requestedName), entry]));
  let simbadSource = null;
  if (simbadBytes !== null) {
    const second = json(simbadQualityBytes);
    if (second.status !== 'REVIEW_REQUIRED' || second.simbadResponseSha256 !== digest(simbadBytes)
        || second.hip2Sha256 !== hip2Sha256) throw new Error('SIMBAD source or quality hash mismatch');
    const leads = parseSimbad(simbadBytes.toString('utf8'));
    const seen = new Set();
    for (const lead of leads) {
      const key = fold(lead.requestedName);
      const row = index.get(key);
      if (!row || row.iauStatus !== 'UNMATCHED' || seen.has(key)) throw new Error('unexpected or duplicate SIMBAD name');
      seen.add(key);
    }
    const secondary = leads.filter((lead) => lead.hip !== null);
    const secondaryQuality = checkQuality(secondary, second.results, inspectQuality, hip2Path);
    for (const lead of leads) {
      index.get(fold(lead.requestedName)).simbadCandidates.push({ ...lead,
        hip2Quality: secondaryQuality.get(fold(lead.requestedName)) ?? null });
    }
    simbadSource = { responseSha256: digest(simbadBytes), qualitySha256: digest(simbadQualityBytes) };
  }
  const hipNames = new Map();
  for (const row of entries) for (const lead of [...row.iauCandidates, ...row.simbadCandidates]) {
    if (lead.hip !== null) {
      const namesForHip = hipNames.get(lead.hip) ?? new Set();
      namesForHip.add(row.requestedName);
      hipNames.set(lead.hip, namesForHip);
    }
  }
  for (const row of entries) {
    const shared = new Set();
    for (const lead of [...row.iauCandidates, ...row.simbadCandidates]) {
      for (const other of hipNames.get(lead.hip) ?? []) if (other !== row.requestedName) shared.add(other);
    }
    row.sharedHipWith = [...shared].sort();
  }
  return { schema: 'virgostars-star-review-queue/1', status: 'REVIEW_REQUIRED_NOT_A_CATALOG',
    sources: { iauCatalogSha256: digest(iauBytes), requestedNamesSha256: digest(namesBytes),
      candidateReportSha256: digest(candidateBytes), iauQualitySha256: digest(qualityBytes),
      hip2Sha256, iauSourceUrl: candidate.sourceUrl, simbad: simbadSource },
    counts: { requested: entries.length, iauLead: entries.filter((r) => r.iauCandidates.length).length,
      simbadLead: entries.filter((r) => r.simbadCandidates.length).length,
      noLead: entries.filter((r) => !r.iauCandidates.length && !r.simbadCandidates.length).length,
      decisionsCompleted: 0 }, entries };
}

function main() {
  const [namesPath, iauPath, candidatePath, qualityPath, hip2Path, outputPath, ...optional] = process.argv.slice(2);
  if (![namesPath, iauPath, candidatePath, qualityPath, hip2Path, outputPath].every(Boolean)
      || (optional.length !== 0 && optional.length !== 2)) {
    throw new Error('usage: node scripts/build_star_review_queue.mjs <names.json> <iau.txt> <candidates.json> <quality.json> <hip2.dat> <new-output.json> [simbad.csv simbad-quality.json]');
  }
  const paths = [namesPath, iauPath, candidatePath, qualityPath, hip2Path, ...optional];
  if (paths.some((p) => resolve(p) === resolve(outputPath))) throw new Error('output must differ from inputs');
  const limits = [1_000_000, 4_000_000, 4_000_000, 4_000_000, 100_000_000, 4_000_000, 4_000_000];
  const input = paths.map((p, i) => {
    const stat = statSync(p);
    if (!stat.isFile() || stat.size > limits[i]) throw new Error('invalid or oversized input');
    const data = readFileSync(p);
    if (data.length > limits[i]) throw new Error('input exceeded size limit during read');
    return data;
  });
  const require = createRequire(import.meta.url);
  const { inspectHipparcosQuality } = require('../bindings/node/index.js');
  const queue = buildReviewQueue({ namesBytes: input[0], iauBytes: input[1], candidateBytes: input[2],
    qualityBytes: input[3], hip2Sha256: digest(input[4]), hip2Path, inspectQuality: inspectHipparcosQuality,
    simbadBytes: input[5] ?? null, simbadQualityBytes: input[6] ?? null });
  for (let i = 0; i < paths.length; i++) if (digest(readFileSync(paths[i])) !== digest(input[i])) throw new Error('input changed during review-queue construction');
  writeFileSync(outputPath, `${JSON.stringify(queue, null, 2)}\n`, { flag: 'wx' });
  console.log(`${queue.counts.requested} requests, ${queue.counts.iauLead} IAU leads, ${queue.counts.simbadLead} SIMBAD leads, ${queue.counts.noLead} without leads; all require review.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`Star review queue failed: ${error.message}`); process.exitCode = 1; }
}
