// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-17): retain unverified designation hypotheses.
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
const fold = (s) => s.trim().toLocaleLowerCase('en-US');
const decisions = ['identity', 'quality', 'magnitude', 'radialVelocity', 'aliases', 'sourceRights'];
const fields = ['requestedName', 'designation', 'hip', 'evidenceNote', 'evidenceUrls'];

export function attachDesignationLeads({ queueBytes, researchBytes, hip2Sha256, hip2Path, inspectQuality }) {
  const queue = JSON.parse(queueBytes.toString('utf8'));
  const research = JSON.parse(researchBytes.toString('utf8'));
  if (queue.schema !== 'virgostars-star-review-queue/3' || queue.status !== 'REVIEW_REQUIRED_NOT_A_CATALOG'
      || !queue.sources || queue.sources.hip2Sha256 !== hip2Sha256 || !/^[a-f0-9]{64}$/.test(hip2Sha256)
      || !Array.isArray(queue.entries) || !queue.entries.length || !queue.counts || typeof inspectQuality !== 'function') {
    throw new Error('invalid queue or HIP2 source hash mismatch');
  }
  if (research.schema !== 'virgostars-star-designation-leads/1' || research.status !== 'CALLER_ASSERTED_REVIEW_REQUIRED'
      || research.baseQueueSha256 !== digest(queueBytes) || !Array.isArray(research.leads)
      || !research.leads.length || research.leads.length > 2000) throw new Error('invalid research document or base hash');
  const entries = [];
  const byName = new Map();
  for (const row of queue.entries) {
    if (!row || typeof row.requestedName !== 'string' || !row.requestedName.trim() || byName.has(fold(row.requestedName))
        || ['iauCandidates', 'simbadCandidates', 'researchCandidates', 'objectCandidates'].some((key) => !Array.isArray(row[key]))
        || !row.decisions || Object.keys(row.decisions).length !== decisions.length
        || decisions.some((key) => !Object.hasOwn(row.decisions, key) || row.decisions[key] !== null)) {
      throw new Error('invalid or already decided queue row');
    }
    const copy = { ...row, designationCandidates: [] };
    entries.push(copy);
    byName.set(fold(row.requestedName), copy);
  }
  const count = (predicate) => entries.filter(predicate).length;
  const noExact = (r) => !r.iauCandidates.length && !r.simbadCandidates.length;
  const noStellar = (r) => noExact(r) && !r.researchCandidates.length;
  const noAny = (r) => noStellar(r) && !r.objectCandidates.length;
  if (queue.counts.requested !== entries.length || queue.counts.decisionsCompleted !== 0
      || queue.counts.iauLead !== count((r) => r.iauCandidates.length)
      || queue.counts.simbadLead !== count((r) => r.simbadCandidates.length)
      || queue.counts.noLead !== count(noExact) || queue.counts.namesWithResearchLeads !== count((r) => r.researchCandidates.length)
      || queue.counts.noRecordedLeadBeforeObjects !== count(noStellar)
      || queue.counts.namesWithObjectLeads !== count((r) => r.objectCandidates.length)
      || queue.counts.objectHypotheses !== entries.reduce((n, r) => n + r.objectCandidates.length, 0)
      || queue.counts.noRecordedLead !== count(noAny)) throw new Error('base queue counts are inconsistent');
  const seen = new Set();
  const leads = research.leads.map((lead) => {
    if (!lead || Object.keys(lead).length !== fields.length || fields.some((key) => !Object.hasOwn(lead, key))
        || typeof lead.requestedName !== 'string' || !lead.requestedName.trim()
        || typeof lead.designation !== 'string' || !lead.designation.trim()
        || !Number.isSafeInteger(lead.hip) || lead.hip <= 0 || lead.hip > 0xffffffff
        || typeof lead.evidenceNote !== 'string' || !lead.evidenceNote.trim()
        || !Array.isArray(lead.evidenceUrls) || !lead.evidenceUrls.length || lead.evidenceUrls.length > 10) {
      throw new Error('invalid designation lead; coordinates and magnitudes are not accepted');
    }
    for (const url of lead.evidenceUrls) {
      try { if (typeof url !== 'string' || new URL(url).protocol !== 'https:') throw new Error(); }
      catch { throw new Error('designation source URLs must be HTTPS'); }
    }
    const row = byName.get(fold(lead.requestedName));
    if (!row) throw new Error('designation lead refers to an unknown name');
    const key = `${fold(lead.requestedName)}\0${fold(lead.designation)}\0${lead.hip}`;
    if (seen.has(key)) throw new Error('duplicate designation lead');
    seen.add(key);
    return { row, lead };
  });
  const hips = [...new Set(leads.map(({ lead }) => lead.hip))];
  const quality = inspectQuality(hip2Path, hips);
  if (!Array.isArray(quality) || quality.length !== hips.length) throw new Error('HIP2 inspection incomplete');
  const qualityByHip = new Map();
  for (let i = 0; i < hips.length; i++) {
    const item = quality[i];
    if (!item || item.hip !== hips[i] || !Number.isInteger(item.solutionType) || !Number.isInteger(item.components)
        || typeof item.strictEligible !== 'boolean' || typeof item.traditionalEligible !== 'boolean') throw new Error('HIP2 inspection returned inconsistent records');
    qualityByHip.set(item.hip, { hip: item.hip, solutionType: item.solutionType, components: item.components,
      strictEligible: item.strictEligible, traditionalEligible: item.traditionalEligible });
  }
  for (const { row, lead } of leads) row.designationCandidates.push({ status: 'CALLER_ASSERTED_IDENTITY_NOT_VERIFIED',
    designation: lead.designation.trim(), hip: lead.hip, hip2Quality: qualityByHip.get(lead.hip),
    evidenceNote: lead.evidenceNote.trim(), evidenceUrls: [...lead.evidenceUrls] });
  const hipNames = new Map();
  const starCandidates = (row) => [...row.iauCandidates, ...row.simbadCandidates, ...row.researchCandidates, ...row.designationCandidates];
  for (const row of entries) for (const candidate of starCandidates(row)) {
    if (candidate.hip !== null && candidate.hip !== undefined) {
      const names = hipNames.get(candidate.hip) ?? new Set();
      names.add(row.requestedName); hipNames.set(candidate.hip, names);
    }
  }
  for (const row of entries) {
    const shared = new Set();
    for (const candidate of starCandidates(row)) for (const name of hipNames.get(candidate.hip) ?? []) {
      if (name !== row.requestedName) shared.add(name);
    }
    row.sharedHipWith = [...shared].sort();
  }
  return { schema: 'virgostars-star-review-queue/4', status: 'REVIEW_REQUIRED_NOT_A_CATALOG',
    baseQueueSha256: digest(queueBytes), designationResearchDocumentSha256: digest(researchBytes), sources: queue.sources,
    counts: { ...queue.counts, noRecordedLeadBeforeDesignations: count(noAny),
      namesWithDesignationLeads: count((r) => r.designationCandidates.length), designationHypotheses: leads.length,
      noRecordedLead: count((r) => noAny(r) && !r.designationCandidates.length) }, entries };
}

function main() {
  const [queuePath, researchPath, hip2Path, outputPath, ...extra] = process.argv.slice(2);
  if (![queuePath, researchPath, hip2Path, outputPath].every(Boolean) || extra.length) throw new Error('usage: node scripts/attach_star_designation_leads.mjs <queue-v3.json> <designation-leads.json> <hip2.dat> <new-output.json>');
  const paths = [queuePath, researchPath, hip2Path];
  if (paths.some((p) => resolve(p) === resolve(outputPath))) throw new Error('output must differ from inputs');
  const limits = [8_000_000, 4_000_000, 100_000_000];
  const input = paths.map((p, i) => {
    const stat = statSync(p);
    if (!stat.isFile() || stat.size > limits[i]) throw new Error('invalid or oversized input');
    const bytes = readFileSync(p);
    if (bytes.length > limits[i]) throw new Error('input exceeded size limit during read');
    return bytes;
  });
  const { inspectHipparcosQuality } = createRequire(import.meta.url)('../bindings/node/index.js');
  const result = attachDesignationLeads({ queueBytes: input[0], researchBytes: input[1], hip2Sha256: digest(input[2]), hip2Path, inspectQuality: inspectHipparcosQuality });
  for (let i = 0; i < paths.length; i++) if (digest(readFileSync(paths[i])) !== digest(input[i])) throw new Error('input changed during designation attachment');
  writeFileSync(outputPath, `${JSON.stringify(result, null, 2)}\n`, { flag: 'wx' });
  console.log(`${result.counts.namesWithDesignationLeads} names with designation leads; ${result.counts.noRecordedLead} without recorded leads; no decisions approved.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`Star designation attachment failed: ${error.message}`); process.exitCode = 1; }
}
