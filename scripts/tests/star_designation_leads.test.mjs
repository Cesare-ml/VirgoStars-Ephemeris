// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { attachDesignationLeads } from '../attach_star_designation_leads.mjs';

const bytes = (value) => Buffer.from(JSON.stringify(value));
const sha = (value) => createHash('sha256').update(value).digest('hex');
function fixture() {
  const decisions = { identity: null, quality: null, magnitude: null, radialVelocity: null, aliases: null, sourceRights: null };
  const queue = { schema: 'virgostars-star-review-queue/3', status: 'REVIEW_REQUIRED_NOT_A_CATALOG', sources: { hip2Sha256: 'a'.repeat(64) },
    counts: { requested: 3, iauLead: 1, simbadLead: 0, noLead: 2, decisionsCompleted: 0,
      namesWithResearchLeads: 0, noRecordedLeadBeforeObjects: 2, namesWithObjectLeads: 1, objectHypotheses: 1, noRecordedLead: 1 },
    entries: [
      { requestedName: 'Known', iauCandidates: [{ hip: 123 }], simbadCandidates: [], researchCandidates: [], objectCandidates: [], decisions },
      { requestedName: 'Uncertain', iauCandidates: [], simbadCandidates: [], researchCandidates: [], objectCandidates: [], decisions },
      { requestedName: 'Object', iauCandidates: [], simbadCandidates: [], researchCandidates: [], objectCandidates: [{}], decisions },
    ] };
  const queueBytes = bytes(queue);
  const research = { schema: 'virgostars-star-designation-leads/1', status: 'CALLER_ASSERTED_REVIEW_REQUIRED', baseQueueSha256: sha(queueBytes),
    leads: [
      { requestedName: 'Uncertain', designation: 'Example A', hip: 123, evidenceNote: 'Claim only', evidenceUrls: ['https://example.org/a'] },
      { requestedName: 'Uncertain', designation: 'Example B', hip: 456, evidenceNote: 'Alternative', evidenceUrls: ['https://example.org/b'] },
    ] };
  return { queueBytes, researchBytes: bytes(research), hip2Sha256: 'a'.repeat(64), hip2Path: 'mock',
    inspectQuality: (_, hips) => hips.map((hip) => ({ hip, solutionType: 5, components: 1, strictEligible: true, traditionalEligible: true })) };
}

test('preserves competing unverified identities and marks shared HIPs', () => {
  const data = fixture();
  const result = attachDesignationLeads(data);
  assert.equal(result.schema, 'virgostars-star-review-queue/4');
  assert.equal(result.counts.noRecordedLeadBeforeDesignations, 1);
  assert.equal(result.counts.noRecordedLead, 0);
  assert.equal(result.counts.namesWithDesignationLeads, 1);
  assert.equal(result.counts.designationHypotheses, 2);
  assert.equal(result.entries[1].designationCandidates[0].status, 'CALLER_ASSERTED_IDENTITY_NOT_VERIFIED');
  assert.deepEqual(result.entries[0].sharedHipWith, ['Uncertain']);
  assert.deepEqual(result.entries[1].sharedHipWith, ['Known']);
  assert.ok(result.entries.every((row) => Object.values(row.decisions).every((value) => value === null)));
  assert.deepEqual(attachDesignationLeads(data), result);
});

test('rejects stale, altered, decided and malformed inputs', () => {
  const data = fixture();
  const mutate = (fn) => { const value = JSON.parse(data.researchBytes); fn(value); return bytes(value); };
  assert.throws(() => attachDesignationLeads({ ...data, hip2Sha256: 'b'.repeat(64) }), /source hash/);
  assert.throws(() => attachDesignationLeads({ ...data, researchBytes: mutate((x) => { x.baseQueueSha256 = '0'.repeat(64); }) }), /base hash/);
  assert.throws(() => attachDesignationLeads({ ...data, researchBytes: mutate((x) => { x.leads[0].longitude = 1; }) }), /coordinates and magnitudes/);
  assert.throws(() => attachDesignationLeads({ ...data, researchBytes: mutate((x) => { x.leads[0].evidenceUrls = ['http://example.org']; }) }), /HTTPS/);
  assert.throws(() => attachDesignationLeads({ ...data, researchBytes: mutate((x) => { x.leads.push(x.leads[0]); }) }), /duplicate/);
  assert.throws(() => attachDesignationLeads({ ...data, researchBytes: mutate((x) => { x.leads[0].requestedName = 'Unknown'; }) }), /unknown/);
  assert.throws(() => attachDesignationLeads({ ...data, inspectQuality: () => [] }), /incomplete/);
  assert.throws(() => attachDesignationLeads({ ...data, inspectQuality: () => [{ hip: 456 }, { hip: 123 }] }), /inconsistent/);
  const queue = JSON.parse(data.queueBytes); queue.entries[0].decisions.identity = 'approved';
  const queueBytes = bytes(queue);
  assert.throws(() => attachDesignationLeads({ ...data, queueBytes, researchBytes: mutate((x) => { x.baseQueueSha256 = sha(queueBytes); }) }), /already decided/);
  queue.entries[0].decisions.identity = null; queue.counts.noLead = 0;
  const corrupt = bytes(queue);
  assert.throws(() => attachDesignationLeads({ ...data, queueBytes: corrupt, researchBytes: mutate((x) => { x.baseQueueSha256 = sha(corrupt); }) }), /counts are inconsistent/);
});

const { STAR_MIXED_QUEUE, STAR_DESIGNATION_LEADS, STAR_DESIGNATION_QUEUE, HIP2_PATH } = process.env;
test('real designation queue reproduces byte-for-byte and refuses overwrite',
  { skip: STAR_MIXED_QUEUE && STAR_DESIGNATION_LEADS && STAR_DESIGNATION_QUEUE && HIP2_PATH ? false : 'local review data required' }, () => {
    const dir = mkdtempSync(join(tmpdir(), 'virgostars-designations-'));
    const output = join(dir, 'queue.json');
    const script = fileURLToPath(new URL('../attach_star_designation_leads.mjs', import.meta.url));
    const args = [script, STAR_MIXED_QUEUE, STAR_DESIGNATION_LEADS, HIP2_PATH, output];
    try {
      const first = spawnSync(process.execPath, args, { encoding: 'utf8' });
      assert.equal(first.status, 0, first.stderr);
      const saved = readFileSync(output);
      assert.deepEqual(saved, readFileSync(STAR_DESIGNATION_QUEUE));
      const queue = JSON.parse(saved);
      assert.equal(queue.counts.namesWithDesignationLeads, 10);
      assert.equal(queue.counts.designationHypotheses, 15);
      assert.equal(queue.counts.noRecordedLead, 3);
      const row = (name) => queue.entries.find((entry) => entry.requestedName === name);
      assert.equal(row('Birdun').designationCandidates.length, 2);
      assert.equal(row('Muhlifain').designationCandidates.length, 3);
      assert.ok(row('Manus').sharedHipWith.includes('El Nath'));
      assert.ok(row('Muhlifain').designationCandidates.every((candidate) => !candidate.hip2Quality.traditionalEligible));
      assert.ok(queue.entries.every((row) => Object.values(row.decisions).every((value) => value === null)));
      const second = spawnSync(process.execPath, args, { encoding: 'utf8' });
      assert.equal(second.status, 1);
      assert.deepEqual(readFileSync(output), saved);
    } finally { rmSync(output, { force: true }); rmSync(dir, { recursive: true, force: true }); }
  });

const { STAR_DESIGNATION_LEADS_V2, STAR_DESIGNATIONS_V6 } = process.env;
test('expanded real designation research preserves the prior claims and reproduces the queue',
  { skip: STAR_MIXED_QUEUE && STAR_DESIGNATION_LEADS && STAR_DESIGNATION_LEADS_V2 && STAR_DESIGNATIONS_V6 && HIP2_PATH
    ? false : 'local updated designation inputs required' }, () => {
    const oldLeads = JSON.parse(readFileSync(STAR_DESIGNATION_LEADS));
    const newLeads = JSON.parse(readFileSync(STAR_DESIGNATION_LEADS_V2));
    assert.deepEqual(newLeads.leads.slice(0, oldLeads.leads.length), oldLeads.leads);
    const dir = mkdtempSync(join(tmpdir(), 'virgostars-designations-v2-'));
    const output = join(dir, 'queue.json');
    const script = fileURLToPath(new URL('../attach_star_designation_leads.mjs', import.meta.url));
    try {
      const run = spawnSync(process.execPath, [script, STAR_MIXED_QUEUE, STAR_DESIGNATION_LEADS_V2, HIP2_PATH, output], { encoding: 'utf8' });
      assert.equal(run.status, 0, run.stderr);
      const saved = readFileSync(output);
      assert.deepEqual(saved, readFileSync(STAR_DESIGNATIONS_V6));
      const queue = JSON.parse(saved);
      assert.equal(queue.counts.namesWithDesignationLeads, 12);
      assert.equal(queue.counts.designationHypotheses, 18);
      assert.equal(queue.counts.noRecordedLead, 1);
      assert.ok(queue.entries.every((entry) => Object.values(entry.decisions).every((value) => value === null)));
    } finally { rmSync(output, { force: true }); rmSync(dir, { recursive: true, force: true }); }
  });
