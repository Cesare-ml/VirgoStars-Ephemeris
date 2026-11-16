// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Fictional cases, no coordinate, magnitude or identity approval.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, rmdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { attachObjectLeads } from '../attach_star_object_leads.mjs';

const bytes = (value) => Buffer.from(JSON.stringify(value));
const sha = (value) => createHash('sha256').update(value).digest('hex');
function fixture() {
  const decisions = { identity: null, quality: null, magnitude: null, radialVelocity: null, aliases: null, sourceRights: null };
  const queue = { schema: 'virgostars-star-review-queue/2', status: 'REVIEW_REQUIRED_NOT_A_CATALOG', sources: {},
    counts: { requested: 3, iauLead: 0, simbadLead: 1, noLead: 2, decisionsCompleted: 0, namesWithResearchLeads: 1, noRecordedLead: 1 },
    entries: [
      { requestedName: 'Alias', iauCandidates: [], simbadCandidates: [], researchCandidates: [{ hip: 123 }], decisions },
      { requestedName: 'Cluster', iauCandidates: [], simbadCandidates: [], researchCandidates: [], decisions },
      { requestedName: 'Known Cluster', iauCandidates: [], simbadCandidates: [{ hip: null }], researchCandidates: [], decisions },
    ] };
  const queueBytes = bytes(queue);
  const research = { schema: 'virgostars-star-object-leads/1', status: 'CALLER_ASSERTED_REVIEW_REQUIRED', baseQueueSha256: sha(queueBytes),
    leads: [
      { requestedName: 'Cluster', objectKind: 'OPEN_CLUSTER', designation: 'Example 1', evidenceNote: 'Fictional claim', evidenceUrls: ['https://example.org/one'] },
      { requestedName: 'Cluster', objectKind: 'NEBULA', designation: 'Example 2', evidenceNote: 'Alternative fictional claim', evidenceUrls: ['https://example.org/two'] },
      { requestedName: 'Known Cluster', objectKind: 'OPEN_CLUSTER', designation: 'Example 3', evidenceNote: 'Fictional existing lead', evidenceUrls: ['https://example.org/three'] },
    ] };
  return { queueBytes, researchBytes: bytes(research) };
}

test('extended and competing object hypotheses remain explicit and unapproved', () => {
  const data = fixture();
  const result = attachObjectLeads(data);
  assert.equal(result.schema, 'virgostars-star-review-queue/3');
  assert.equal(result.counts.noRecordedLeadBeforeObjects, 1);
  assert.equal(result.counts.namesWithObjectLeads, 2);
  assert.equal(result.counts.objectHypotheses, 3);
  assert.equal(result.counts.noRecordedLead, 0);
  assert.equal(result.entries[1].objectCandidates.length, 2);
  assert.equal(result.entries[1].objectCandidates[0].singleHipPath, 'NOT_APPLICABLE');
  assert.equal(result.entries[1].objectCandidates[0].positionPolicy, 'UNDECIDED');
  assert.ok(result.entries.every((entry) => Object.values(entry.decisions).every((value) => value === null)));
  assert.deepEqual(attachObjectLeads(data), result);
});

test('rejects stale, guessed, malformed and already decided object hypotheses', () => {
  const data = fixture();
  const mutate = (fn) => { const value = JSON.parse(data.researchBytes); fn(value); return bytes(value); };
  assert.throws(() => attachObjectLeads({ ...data, researchBytes: mutate((x) => { x.baseQueueSha256 = '0'.repeat(64); }) }), /base hash/);
  assert.throws(() => attachObjectLeads({ ...data, researchBytes: mutate((x) => { x.leads[0].hip = 123; }) }), /HIP, coordinates and magnitudes/);
  assert.throws(() => attachObjectLeads({ ...data, researchBytes: mutate((x) => { x.leads[0].longitude = 1; }) }), /HIP, coordinates and magnitudes/);
  assert.throws(() => attachObjectLeads({ ...data, researchBytes: mutate((x) => { x.leads[0].evidenceUrls = ['http://example.org']; }) }), /HTTPS/);
  assert.throws(() => attachObjectLeads({ ...data, researchBytes: mutate((x) => { x.leads.push(x.leads[0]); }) }), /duplicate object lead/);
  assert.throws(() => attachObjectLeads({ ...data, researchBytes: mutate((x) => { x.leads[0].requestedName = 'Unknown'; }) }), /unknown requested name/);
  const queue = JSON.parse(data.queueBytes);
  queue.entries[0].decisions.identity = 'approved';
  const queueBytes = bytes(queue);
  assert.throws(() => attachObjectLeads({ queueBytes, researchBytes: mutate((x) => { x.baseQueueSha256 = sha(queueBytes); }) }), /already decided/);
  const wrongCounts = JSON.parse(data.queueBytes);
  wrongCounts.counts.noLead = 0;
  const corruptedQueue = bytes(wrongCounts);
  assert.throws(() => attachObjectLeads({ queueBytes: corruptedQueue, researchBytes: mutate((x) => { x.baseQueueSha256 = sha(corruptedQueue); }) }), /counts are inconsistent/);
});

const { STAR_STELLAR_QUEUE, STAR_OBJECT_LEADS, STAR_MIXED_QUEUE } = process.env;
test('real extended-object review is reproducible and refuses overwrite',
  { skip: STAR_STELLAR_QUEUE && STAR_OBJECT_LEADS && STAR_MIXED_QUEUE ? false : 'local object review inputs required' }, () => {
    const directory = mkdtempSync(join(tmpdir(), 'virgostars-object-leads-'));
    const output = join(directory, 'queue.json');
    const script = fileURLToPath(new URL('../attach_star_object_leads.mjs', import.meta.url));
    const args = [script, STAR_STELLAR_QUEUE, STAR_OBJECT_LEADS, output];
    try {
      const first = spawnSync(process.execPath, args, { encoding: 'utf8' });
      assert.equal(first.status, 0, first.stderr);
      const saved = readFileSync(output);
      assert.deepEqual(saved, readFileSync(STAR_MIXED_QUEUE));
      const queue = JSON.parse(saved);
      assert.equal(queue.counts.namesWithObjectLeads, 5);
      assert.equal(queue.counts.objectHypotheses, 6);
      assert.equal(queue.counts.noRecordedLead, 13);
      assert.ok(queue.entries.every((entry) => Object.values(entry.decisions).every((value) => value === null)));
      const second = spawnSync(process.execPath, args, { encoding: 'utf8' });
      assert.equal(second.status, 1);
      assert.deepEqual(readFileSync(output), saved);
    } finally {
      rmSync(output, { force: true });
      rmdirSync(directory);
    }
  });
