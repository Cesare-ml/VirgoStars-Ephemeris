// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Fictional inputs only; these tests never approve or distribute a catalog.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, rmdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { findCandidates } from '../iau_star_candidates.mjs';
import { buildReviewQueue } from '../build_star_review_queue.mjs';

const bytes = (x) => Buffer.from(typeof x === 'string' ? x : JSON.stringify(x));
const sha = (b) => createHash('sha256').update(b).digest('hex');
function iauRow(name, hip) {
  const record = Array(140).fill(' ');
  const put = (offset, text) => { for (let i = 0; i < text.length; i++) record[offset + i] = text[i]; };
  put(0, name); put(65, 'A'); put(81, ' 1.20'); put(87, 'V'); put(90, String(hip).padStart(7)); put(126, '2020-01-02');
  return record.join('');
}
function fixture() {
  const namesBytes = bytes(['Alpha', 'Beta', 'Gamma', 'Delta', 'Epsilon']);
  const iauBytes = bytes(`#Name/ASCII Dec(J2000)\n${iauRow('Alpha', 123)}\n${iauRow('Beta', '_')}\n`);
  const candidate = findCandidates(iauBytes.toString('utf8'), JSON.parse(namesBytes), 'https://example.org/fictional');
  candidate.catalogSha256 = sha(iauBytes);
  candidate.requestedNamesSha256 = sha(namesBytes);
  const candidateBytes = bytes(candidate);
  const hip2Sha256 = sha(bytes('fictional HIP2'));
  const a = { hip: 123, solutionType: 5, components: 1, strictEligible: true, traditionalEligible: true };
  const b = { hip: 456, solutionType: 7, components: 1, strictEligible: false, traditionalEligible: true };
  const quality = { schema: 'virgostars-hip2-quality-audit/1', status: 'REVIEW_REQUIRED',
    candidateReportSha256: sha(candidateBytes), hip2Sha256, results: [{ requestedName: 'Alpha', ...a }] };
  const simbadBytes = bytes('id,main_id,otype,hip_id\n"NAME Gamma","Fictional A","Star","HIP 456"\n"NAME Delta","Fictional cluster","OpC",\n');
  const second = { status: 'REVIEW_REQUIRED', simbadResponseSha256: sha(simbadBytes), hip2Sha256,
    results: [{ requestedName: 'Gamma', ...b }] };
  return { namesBytes, iauBytes, candidateBytes, qualityBytes: bytes(quality), hip2Sha256,
    hip2Path: 'fictional', inspectQuality: (_path, hips) => hips.map((hip) => ({ ...(hip === 123 ? a : b) })),
    simbadBytes, simbadQualityBytes: bytes(second) };
}

test('queue combines verified leads but leaves every product decision unresolved', () => {
  const data = fixture();
  const report = buildReviewQueue(data);
  assert.deepEqual(report.counts, { requested: 5, iauLead: 2, simbadLead: 2, noLead: 1, decisionsCompleted: 0 });
  assert.equal(report.status, 'REVIEW_REQUIRED_NOT_A_CATALOG');
  assert.equal(report.entries[0].iauCandidates[0].hip2Quality.strictEligible, true);
  assert.equal(report.entries[1].iauCandidates[0].hip, null);
  assert.equal(report.entries[2].simbadCandidates[0].hip, 456);
  assert.equal(report.entries[2].simbadCandidates[0].hip2Quality.traditionalEligible, true);
  assert.equal(report.entries[3].simbadCandidates[0].hip, null);
  assert.ok(report.entries.every((entry) => Object.values(entry.decisions).every((value) => value === null)));
  const withoutSimbad = buildReviewQueue({ ...data, simbadBytes: null, simbadQualityBytes: null });
  assert.equal(withoutSimbad.counts.noLead, 3);
  assert.equal(withoutSimbad.sources.simbad, null);
});

test('queue rejects stale sources, altered candidates, quality and inconsistent SIMBAD input', () => {
  const data = fixture();
  assert.throws(() => buildReviewQueue({ ...data, namesBytes: bytes(['Different']) }), /hash mismatch/);
  assert.throws(() => buildReviewQueue({ ...data, hip2Sha256: 'a'.repeat(64) }), /hash mismatch/);
  assert.throws(() => buildReviewQueue({ ...data, simbadQualityBytes: null }), /supplied together/);
  const quality = JSON.parse(data.qualityBytes);
  quality.results[0].strictEligible = false;
  assert.throws(() => buildReviewQueue({ ...data, qualityBytes: bytes(quality) }), /differs from HIP2 replay/);
  const editedCandidate = JSON.parse(data.candidateBytes);
  editedCandidate.results[0].candidates[0].component = 'Invented';
  const editedBytes = bytes(editedCandidate);
  const editedQuality = JSON.parse(data.qualityBytes);
  editedQuality.candidateReportSha256 = sha(editedBytes);
  assert.throws(() => buildReviewQueue({ ...data, candidateBytes: editedBytes, qualityBytes: bytes(editedQuality) }), /does not reproduce/);
  const badCsv = bytes('id,main_id,otype,hip_id\n"NAME Gamma","Bad format"\n');
  const second = JSON.parse(data.simbadQualityBytes);
  second.simbadResponseSha256 = sha(badCsv);
  assert.throws(() => buildReviewQueue({ ...data, simbadBytes: badCsv, simbadQualityBytes: bytes(second) }), /unsupported SIMBAD/);
});

const { STAR_REVIEW_INPUT_DIR, HIP2_PATH } = process.env;
test('real local inputs reproduce the review queue without overwriting output',
  { skip: STAR_REVIEW_INPUT_DIR && HIP2_PATH ? false : 'local review inputs and HIP2 required' }, () => {
    const directory = mkdtempSync(join(tmpdir(), 'virgostars-review-queue-'));
    const output = join(directory, 'queue.json');
    const secondOutput = join(directory, 'queue-second.json');
    const script = fileURLToPath(new URL('../build_star_review_queue.mjs', import.meta.url));
    const root = STAR_REVIEW_INPUT_DIR;
    const args = [script, join(root, 'backend-star-names.json'), join(root, 'IAU-CSN-2022-mirror.txt'),
      join(root, 'iau-star-candidates.json'), join(root, 'iau-star-quality.json'), HIP2_PATH, output,
      join(root, 'simbad-unmatched.csv'), join(root, 'simbad-unmatched-quality.json')];
    try {
      const first = spawnSync(process.execPath, args, { encoding: 'utf8' });
      assert.equal(first.status, 0, first.stderr);
      const saved = readFileSync(output);
      const report = JSON.parse(saved);
      assert.deepEqual(report.counts, { requested: 106, iauLead: 77, simbadLead: 4, noLead: 25, decisionsCompleted: 0 });
      const replayArgs = [...args];
      replayArgs[6] = secondOutput;
      const replay = spawnSync(process.execPath, replayArgs, { encoding: 'utf8' });
      assert.equal(replay.status, 0, replay.stderr);
      assert.deepEqual(readFileSync(secondOutput), saved);
      const second = spawnSync(process.execPath, args, { encoding: 'utf8' });
      assert.equal(second.status, 1);
      assert.deepEqual(readFileSync(output), saved);
    } finally {
      rmSync(output, { force: true });
      rmSync(secondOutput, { force: true });
      rmdirSync(directory);
    }
  });
