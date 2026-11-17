// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-17): append caller-documented leads, never approvals.
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { findCandidates } from './iau_star_candidates.mjs';

const digest = (data) => createHash('sha256').update(data).digest('hex');
const fold = (name) => name.trim().toLocaleLowerCase('en-US');

export function attachResearchLeads({ queueBytes, researchBytes, iauBytes, hip2Sha256, hip2Path, inspectQuality }) {
  const queue = JSON.parse(queueBytes.toString('utf8'));
  const research = JSON.parse(researchBytes.toString('utf8'));
  if (queue.schema !== 'virgostars-star-review-queue/1' || queue.status !== 'REVIEW_REQUIRED_NOT_A_CATALOG'
      || !Array.isArray(queue.entries) || !queue.entries.length || !queue.sources
      || queue.sources.iauCatalogSha256 !== digest(iauBytes) || queue.sources.hip2Sha256 !== hip2Sha256
      || typeof inspectQuality !== 'function' || !/^[a-f0-9]{64}$/.test(hip2Sha256)) {
    throw new Error('invalid queue or source hash mismatch');
  }
  if (research.schema !== 'virgostars-star-research-leads/1' || research.status !== 'CALLER_ASSERTED_REVIEW_REQUIRED'
      || research.baseQueueSha256 !== digest(queueBytes) || !Array.isArray(research.leads) || !research.leads.length
      || research.leads.length > 2000) throw new Error('invalid research document or base queue hash');
  const entries = [];
  const byName = new Map();
  const decisionFields = ['identity', 'quality', 'magnitude', 'radialVelocity', 'aliases', 'sourceRights'];
  for (const row of queue.entries) {
    if (!row || typeof row.requestedName !== 'string' || !row.requestedName.trim()
        || byName.has(fold(row.requestedName)) || !Array.isArray(row.iauCandidates)
        || !Array.isArray(row.simbadCandidates) || !row.decisions
        || decisionFields.some((key) => !Object.hasOwn(row.decisions, key) || row.decisions[key] !== null)
        || Object.keys(row.decisions).length !== decisionFields.length) throw new Error('invalid or already decided queue row');
    const copy = { ...row, researchCandidates: [] };
    entries.push(copy);
    byName.set(fold(row.requestedName), copy);
  }
  if (!queue.counts || queue.counts.requested !== entries.length || queue.counts.decisionsCompleted !== 0
      || queue.counts.iauLead !== entries.filter((row) => row.iauCandidates.length).length
      || queue.counts.simbadLead !== entries.filter((row) => row.simbadCandidates.length).length
      || queue.counts.noLead !== entries.filter((row) => !row.iauCandidates.length && !row.simbadCandidates.length).length) {
    throw new Error('queue counts do not match unresolved rows');
  }
  const seen = new Set();
  const leads = research.leads.map((lead) => {
    if (!lead || typeof lead.requestedName !== 'string' || !lead.requestedName.trim()
        || typeof lead.standardName !== 'string' || !lead.standardName.trim()
        || !Number.isSafeInteger(lead.expectedHip) || lead.expectedHip <= 0 || lead.expectedHip > 0xffffffff
        || typeof lead.evidenceNote !== 'string' || !lead.evidenceNote.trim()
        || !Array.isArray(lead.evidenceUrls) || !lead.evidenceUrls.length || lead.evidenceUrls.length > 10) {
      throw new Error('invalid research lead');
    }
    for (const value of lead.evidenceUrls) {
      try { if (typeof value !== 'string' || new URL(value).protocol !== 'https:') throw new Error(); }
      catch { throw new Error('research source URLs must be HTTPS'); }
    }
    const row = byName.get(fold(lead.requestedName));
    if (!row || row.iauCandidates.length || row.simbadCandidates.length) throw new Error('research lead must target a name without an exact lead');
    const key = `${fold(lead.requestedName)}\0${fold(lead.standardName)}\0${lead.expectedHip}`;
    if (seen.has(key)) throw new Error('duplicate research lead');
    seen.add(key);
    const found = findCandidates(iauBytes.toString('utf8'), [lead.standardName], queue.sources.iauSourceUrl).results[0];
    if (found.status !== 'CANDIDATE_REVIEW_REQUIRED' || found.candidates[0].hip !== lead.expectedHip) {
      throw new Error('research lead does not match exactly one expected HIP in the IAU snapshot');
    }
    return { row, candidate: found.candidates[0], lead };
  });
  const hips = [...new Set(leads.map((entry) => entry.candidate.hip))];
  const quality = inspectQuality(hip2Path, hips);
  if (!Array.isArray(quality) || quality.length !== hips.length) throw new Error('HIP2 inspection incomplete');
  const qualityByHip = new Map();
  for (let i = 0; i < hips.length; i++) {
    const item = quality[i];
    if (!item || item.hip !== hips[i] || !Number.isInteger(item.solutionType) || !Number.isInteger(item.components)
        || typeof item.strictEligible !== 'boolean' || typeof item.traditionalEligible !== 'boolean') {
      throw new Error('HIP2 inspection returned inconsistent records');
    }
    qualityByHip.set(item.hip, { hip: item.hip, solutionType: item.solutionType, components: item.components,
      strictEligible: item.strictEligible, traditionalEligible: item.traditionalEligible });
  }
  for (const { row, candidate, lead } of leads) row.researchCandidates.push({ status: 'CALLER_ASSERTED_REVIEW_REQUIRED',
    standardName: candidate.iauName, hip: candidate.hip, component: candidate.component,
    hip2Quality: qualityByHip.get(candidate.hip), evidenceNote: lead.evidenceNote.trim(), evidenceUrls: [...lead.evidenceUrls] });
  const hipNames = new Map();
  for (const row of entries) for (const candidate of [...row.iauCandidates, ...row.simbadCandidates, ...row.researchCandidates]) {
    if (candidate.hip !== null && candidate.hip !== undefined) {
      const names = hipNames.get(candidate.hip) ?? new Set();
      names.add(row.requestedName);
      hipNames.set(candidate.hip, names);
    }
  }
  for (const row of entries) {
    const shared = new Set();
    for (const candidate of [...row.iauCandidates, ...row.simbadCandidates, ...row.researchCandidates]) {
      for (const name of hipNames.get(candidate.hip) ?? []) if (name !== row.requestedName) shared.add(name);
    }
    row.sharedHipWith = [...shared].sort();
  }
  // The original exact-match counts are preserved; the new number describes
  // recorded hypotheses, not resolved identities or an approved catalog.
  const withoutAnyLead = entries.filter((row) => !row.iauCandidates.length && !row.simbadCandidates.length && !row.researchCandidates.length).length;
  return { schema: 'virgostars-star-review-queue/2', status: 'REVIEW_REQUIRED_NOT_A_CATALOG',
    baseQueueSha256: digest(queueBytes), researchDocumentSha256: digest(researchBytes), sources: queue.sources,
    counts: { ...queue.counts, namesWithResearchLeads: entries.filter((row) => row.researchCandidates.length).length,
      noRecordedLead: withoutAnyLead }, entries };
}

function main() {
  const [queuePath, researchPath, iauPath, hip2Path, outputPath, ...extra] = process.argv.slice(2);
  if (![queuePath, researchPath, iauPath, hip2Path, outputPath].every(Boolean) || extra.length) {
    throw new Error('usage: node scripts/attach_star_research_leads.mjs <queue.json> <research.json> <iau.txt> <hip2.dat> <new-output.json>');
  }
  const paths = [queuePath, researchPath, iauPath, hip2Path];
  if (paths.some((p) => resolve(p) === resolve(outputPath))) throw new Error('output must differ from inputs');
  const limits = [8_000_000, 4_000_000, 4_000_000, 100_000_000];
  const input = paths.map((p, i) => {
    const stat = statSync(p);
    if (!stat.isFile() || stat.size > limits[i]) throw new Error('invalid or oversized input');
    const data = readFileSync(p);
    if (data.length > limits[i]) throw new Error('input exceeded size limit during read');
    return data;
  });
  const require = createRequire(import.meta.url);
  const { inspectHipparcosQuality } = require('../bindings/node/index.js');
  const result = attachResearchLeads({ queueBytes: input[0], researchBytes: input[1], iauBytes: input[2],
    hip2Sha256: digest(input[3]), hip2Path, inspectQuality: inspectHipparcosQuality });
  for (let i = 0; i < paths.length; i++) if (digest(readFileSync(paths[i])) !== digest(input[i])) throw new Error('input changed during research attachment');
  writeFileSync(outputPath, `${JSON.stringify(result, null, 2)}\n`, { flag: 'wx' });
  console.log(`${result.counts.namesWithResearchLeads} names with research leads; ${result.counts.noRecordedLead} without recorded leads; no decisions approved.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`Star research attachment failed: ${error.message}`); process.exitCode = 1; }
}
