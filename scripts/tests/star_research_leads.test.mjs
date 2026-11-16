// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Synthetic source evidence exercises mechanics; it is never catalog approval.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, rmdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { attachResearchLeads } from '../attach_star_research_leads.mjs';

const bytes = (value) => Buffer.from(typeof value === 'string' ? value : JSON.stringify(value));
const sha = (value) => createHash('sha256').update(value).digest('hex');
function row(name, hip) {
  const chars = Array(140).fill(' ');
  const put = (offset, text) => { for (let i = 0; i < text.length; i++) chars[offset + i] = text[i]; };
  put(0, name); put(65, 'A'); put(81, ' 1.20'); put(87, 'V'); put(90, String(hip).padStart(7)); put(126, '2020-01-02');
  return chars.join('');
}
function fixture() {
  const iauBytes = bytes(`#Name/ASCII Dec(J2000)\n${row('Standard One', 123)}\n${row('Standard Two', 456)}\n`);
  const hip2Sha256 = sha(bytes('fictional HIP2'));
  const decisions = { identity: null, quality: null, magnitude: null, radialVelocity: null, aliases: null, sourceRights: null };
  const base = { schema: 'virgostars-star-review-queue/1', status: 'REVIEW_REQUIRED_NOT_A_CATALOG',
    sources: { iauCatalogSha256: sha(iauBytes), hip2Sha256, iauSourceUrl: 'https://example.org/catalog' },
    counts: { requested: 3, iauLead: 1, simbadLead: 0, noLead: 2, decisionsCompleted: 0 },
    entries: [
      { requestedName: 'Original', iauCandidates: [{ hip: 123 }], simbadCandidates: [], decisions },
      { requestedName: 'First alias', iauCandidates: [], simbadCandidates: [], decisions },
      { requestedName: 'Second alias', iauCandidates: [], simbadCandidates: [], decisions },
    ] };
  const queueBytes = bytes(base);
  const research = { schema: 'virgostars-star-research-leads/1', status: 'CALLER_ASSERTED_REVIEW_REQUIRED', baseQueueSha256: sha(queueBytes),
    leads: [{ requestedName: 'First alias', standardName: 'Standard One', expectedHip: 123,
      evidenceNote: 'Fictional source claim', evidenceUrls: ['https://example.org/one'] },
    { requestedName: 'Second alias', standardName: 'Standard Two', expectedHip: 456,
      evidenceNote: 'Fictional second claim', evidenceUrls: ['https://example.org/two'] }] };
  return { queueBytes, researchBytes: bytes(research), iauBytes, hip2Sha256, hip2Path: 'fictional',
    inspectQuality: (_path, hips) => hips.map((hip) => ({ hip, solutionType: 5, components: 1, strictEligible: true, traditionalEligible: true })) };
}

test('research candidates retain source claims and flag shared HIPs without approving decisions', () => {
  const data = fixture();
  const report = attachResearchLeads(data);
  assert.equal(report.schema, 'virgostars-star-review-queue/2');
  assert.equal(report.counts.noLead, 2, 'original exact-match count stays intact');
  assert.equal(report.counts.namesWithResearchLeads, 2);
  assert.equal(report.counts.noRecordedLead, 0);
  assert.equal(report.entries[1].researchCandidates[0].status, 'CALLER_ASSERTED_REVIEW_REQUIRED');
  assert.equal(report.entries[1].researchCandidates[0].hip, 123);
  assert.deepEqual(report.entries[0].sharedHipWith, ['First alias']);
  assert.deepEqual(report.entries[1].sharedHipWith, ['Original']);
  assert.ok(report.entries.every((entry) => Object.values(entry.decisions).every((value) => value === null)));
  assert.deepEqual(attachResearchLeads(data), report);
  const extra = JSON.parse(data.researchBytes);
  extra.leads.push({ ...extra.leads[1], requestedName: 'First alias' });
  const ambiguous = attachResearchLeads({ ...data, researchBytes: bytes(extra) });
  assert.equal(ambiguous.entries[1].researchCandidates.length, 2, 'multiple hypotheses remain explicit');
  assert.deepEqual(ambiguous.entries[1].sharedHipWith, ['Original', 'Second alias']);
});

test('supplement rejects stale or untraceable claims and already decided entries', () => {
  const data = fixture();
  const mutate = (fn) => { const research = JSON.parse(data.researchBytes); fn(research); return bytes(research); };
  assert.throws(() => attachResearchLeads({ ...data, iauBytes: bytes('bad') }), /source hash mismatch/);
  assert.throws(() => attachResearchLeads({ ...data, researchBytes: mutate((r) => { r.baseQueueSha256 = '0'.repeat(64); }) }), /base queue hash/);
  assert.throws(() => attachResearchLeads({ ...data, researchBytes: mutate((r) => { r.leads[0].expectedHip = 456; }) }), /expected HIP/);
  assert.throws(() => attachResearchLeads({ ...data, researchBytes: mutate((r) => { r.leads[0].evidenceUrls = ['http://example.org/']; }) }), /HTTPS/);
  assert.throws(() => attachResearchLeads({ ...data, researchBytes: mutate((r) => { r.leads.push(r.leads[0]); }) }), /duplicate research lead/);
  assert.throws(() => attachResearchLeads({ ...data, researchBytes: mutate((r) => { r.leads[0].requestedName = 'Original'; }) }), /without an exact lead/);
  const base = JSON.parse(data.queueBytes);
  base.entries[0].decisions.identity = 'approved';
  const queueBytes = bytes(base);
  assert.throws(() => attachResearchLeads({ ...data, queueBytes, researchBytes: mutate((r) => { r.baseQueueSha256 = sha(queueBytes); }) }), /already decided/);
});

const { STAR_REVIEW_V1, STAR_REVIEW_V2, STAR_RESEARCH_LEADS, STAR_IAU_SNAPSHOT, HIP2_PATH } = process.env;
test('real research supplement is reproducible and refuses overwriting',
  { skip: STAR_REVIEW_V1 && STAR_REVIEW_V2 && STAR_RESEARCH_LEADS && STAR_IAU_SNAPSHOT && HIP2_PATH
    ? false : 'local review queue, sources and HIP2 required' }, () => {
    const directory = mkdtempSync(join(tmpdir(), 'virgostars-research-leads-'));
    const output = join(directory, 'queue.json');
    const script = fileURLToPath(new URL('../attach_star_research_leads.mjs', import.meta.url));
    const args = [script, STAR_REVIEW_V1, STAR_RESEARCH_LEADS, STAR_IAU_SNAPSHOT, HIP2_PATH, output];
    try {
      const first = spawnSync(process.execPath, args, { encoding: 'utf8' });
      assert.equal(first.status, 0, first.stderr);
      const saved = readFileSync(output);
      assert.deepEqual(saved, readFileSync(STAR_REVIEW_V2));
      const queue = JSON.parse(saved);
      assert.equal(queue.counts.namesWithResearchLeads, 7);
      assert.equal(queue.counts.noRecordedLead, 18);
      assert.ok(queue.entries.every((entry) => Object.values(entry.decisions).every((value) => value === null)));
      const second = spawnSync(process.execPath, args, { encoding: 'utf8' });
      assert.equal(second.status, 1);
      assert.deepEqual(readFileSync(output), saved);
    } finally {
      rmSync(output, { force: true });
      rmdirSync(directory);
    }
  });
