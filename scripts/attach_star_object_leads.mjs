// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-17): preserve explicitly unapproved extended-object leads.
// Modification (2026-11-17): append systems and asterisms to designation queues.
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const digest = (value) => createHash('sha256').update(value).digest('hex');
const fold = (value) => value.trim().toLocaleLowerCase('en-US');
const allowedKinds = new Set(['OPEN_CLUSTER', 'GLOBULAR_CLUSTER', 'NEBULA', 'OBJECT_GROUP', 'ASTERISM', 'STELLAR_SYSTEM']);
const requiredFields = ['requestedName', 'objectKind', 'designation', 'evidenceNote', 'evidenceUrls'];

export function attachObjectLeads({ queueBytes, researchBytes }) {
  const queue = JSON.parse(queueBytes.toString('utf8'));
  const research = JSON.parse(researchBytes.toString('utf8'));
  const withDesignations = queue.schema === 'virgostars-star-review-queue/4';
  if ((!withDesignations && queue.schema !== 'virgostars-star-review-queue/2') || queue.status !== 'REVIEW_REQUIRED_NOT_A_CATALOG'
      || !Array.isArray(queue.entries) || !queue.entries.length || !queue.counts) throw new Error('invalid base queue');
  if (research.schema !== 'virgostars-star-object-leads/1' || research.status !== 'CALLER_ASSERTED_REVIEW_REQUIRED'
      || research.baseQueueSha256 !== digest(queueBytes) || !Array.isArray(research.leads) || !research.leads.length
      || research.leads.length > 2000) throw new Error('invalid object research document or base hash');
  const entries = [];
  const byName = new Map();
  const decisions = ['identity', 'quality', 'magnitude', 'radialVelocity', 'aliases', 'sourceRights'];
  for (const row of queue.entries) {
    if (!row || typeof row.requestedName !== 'string' || !row.requestedName.trim() || byName.has(fold(row.requestedName))
        || !Array.isArray(row.iauCandidates) || !Array.isArray(row.simbadCandidates) || !Array.isArray(row.researchCandidates)
        || (withDesignations && (!Array.isArray(row.objectCandidates) || !Array.isArray(row.designationCandidates)))
        || !row.decisions || Object.keys(row.decisions).length !== decisions.length
        || decisions.some((key) => !Object.hasOwn(row.decisions, key) || row.decisions[key] !== null)) {
      throw new Error('invalid or already decided queue row');
    }
    const copy = { ...row, objectCandidates: withDesignations ? [...row.objectCandidates] : [] };
    entries.push(copy);
    byName.set(fold(row.requestedName), copy);
  }
  const count = (predicate) => entries.filter(predicate).length;
  const noStellar = (r) => !r.iauCandidates.length && !r.simbadCandidates.length && !r.researchCandidates.length;
  const noObjects = (r) => noStellar(r) && !r.objectCandidates.length;
  const noAny = (r) => noObjects(r) && (!withDesignations || !r.designationCandidates.length);
  const previousUnmatched = count(noAny);
  if (queue.counts.requested !== entries.length || queue.counts.decisionsCompleted !== 0
      || queue.counts.iauLead !== entries.filter((r) => r.iauCandidates.length).length
      || queue.counts.simbadLead !== entries.filter((r) => r.simbadCandidates.length).length
      || queue.counts.noLead !== entries.filter((r) => !r.iauCandidates.length && !r.simbadCandidates.length).length
      || queue.counts.namesWithResearchLeads !== entries.filter((r) => r.researchCandidates.length).length
      || queue.counts.noRecordedLead !== previousUnmatched
      || (withDesignations && (queue.counts.noRecordedLeadBeforeObjects !== count(noStellar)
        || queue.counts.namesWithObjectLeads !== count((r) => r.objectCandidates.length)
        || queue.counts.objectHypotheses !== entries.reduce((n, r) => n + r.objectCandidates.length, 0)
        || queue.counts.noRecordedLeadBeforeDesignations !== count(noObjects)
        || queue.counts.namesWithDesignationLeads !== count((r) => r.designationCandidates.length)
        || queue.counts.designationHypotheses !== entries.reduce((n, r) => n + r.designationCandidates.length, 0)))) {
    throw new Error('base queue counts are inconsistent');
  }
  const seen = new Set();
  if (withDesignations) for (const row of entries) for (const candidate of row.objectCandidates) {
    if (!candidate || typeof candidate.objectKind !== 'string' || typeof candidate.designation !== 'string') throw new Error('invalid existing object candidate');
    seen.add(`${fold(row.requestedName)}\0${candidate.objectKind}\0${fold(candidate.designation)}`);
  }
  for (const lead of research.leads) {
    if (!lead || Object.keys(lead).length !== requiredFields.length
        || requiredFields.some((field) => !Object.hasOwn(lead, field))
        || typeof lead.requestedName !== 'string' || !lead.requestedName.trim()
        || !allowedKinds.has(lead.objectKind) || typeof lead.designation !== 'string' || !lead.designation.trim()
        || typeof lead.evidenceNote !== 'string' || !lead.evidenceNote.trim()
        || !Array.isArray(lead.evidenceUrls) || !lead.evidenceUrls.length || lead.evidenceUrls.length > 10) {
      throw new Error('invalid object lead; HIP, coordinates and magnitudes are not accepted');
    }
    for (const value of lead.evidenceUrls) {
      try { if (typeof value !== 'string' || new URL(value).protocol !== 'https:') throw new Error(); }
      catch { throw new Error('object source URLs must be HTTPS'); }
    }
    const row = byName.get(fold(lead.requestedName));
    if (!row) throw new Error('object lead refers to an unknown requested name');
    const key = `${fold(lead.requestedName)}\0${lead.objectKind}\0${fold(lead.designation)}`;
    if (seen.has(key)) throw new Error('duplicate object lead');
    seen.add(key);
    row.objectCandidates.push({ status: 'CALLER_ASSERTED_REVIEW_REQUIRED', objectKind: lead.objectKind,
      designation: lead.designation.trim(), evidenceNote: lead.evidenceNote.trim(), evidenceUrls: [...lead.evidenceUrls],
      singleHipPath: lead.objectKind === 'STELLAR_SYSTEM' ? 'COMPONENT_REVIEW_REQUIRED' : 'NOT_APPLICABLE',
      positionPolicy: 'UNDECIDED', magnitudePolicy: 'UNDECIDED' });
  }
  const counts = withDesignations
    ? { ...queue.counts, noRecordedLeadBeforeAdditionalObjects: previousUnmatched,
      namesWithObjectLeads: count((r) => r.objectCandidates.length),
      objectHypotheses: entries.reduce((n, r) => n + r.objectCandidates.length, 0), noRecordedLead: count(noAny) }
    : { ...queue.counts, noRecordedLeadBeforeObjects: previousUnmatched,
      namesWithObjectLeads: count((r) => r.objectCandidates.length), objectHypotheses: research.leads.length, noRecordedLead: count(noAny) };
  return { schema: withDesignations ? 'virgostars-star-review-queue/5' : 'virgostars-star-review-queue/3', status: 'REVIEW_REQUIRED_NOT_A_CATALOG',
    baseQueueSha256: digest(queueBytes), objectResearchDocumentSha256: digest(researchBytes), sources: queue.sources,
    counts, entries };
}

function main() {
  const [queuePath, researchPath, outputPath, ...extra] = process.argv.slice(2);
  if (!queuePath || !researchPath || !outputPath || extra.length) throw new Error('usage: node scripts/attach_star_object_leads.mjs <queue-v2-or-v4.json> <object-leads.json> <new-output.json>');
  const paths = [queuePath, researchPath];
  if (paths.some((p) => resolve(p) === resolve(outputPath))) throw new Error('output must differ from inputs');
  const limits = [8_000_000, 4_000_000];
  const input = paths.map((p, i) => {
    const stat = statSync(p);
    if (!stat.isFile() || stat.size > limits[i]) throw new Error('invalid or oversized input');
    const data = readFileSync(p);
    if (data.length > limits[i]) throw new Error('input exceeded size limit during read');
    return data;
  });
  const result = attachObjectLeads({ queueBytes: input[0], researchBytes: input[1] });
  for (let i = 0; i < paths.length; i++) if (digest(readFileSync(paths[i])) !== digest(input[i])) throw new Error('input changed during object attachment');
  writeFileSync(outputPath, `${JSON.stringify(result, null, 2)}\n`, { flag: 'wx' });
  console.log(`${result.counts.namesWithObjectLeads} names with object leads; ${result.counts.noRecordedLead} without recorded leads; no decisions approved.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`Star object attachment failed: ${error.message}`); process.exitCode = 1; }
}
