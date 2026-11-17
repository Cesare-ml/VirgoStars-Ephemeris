// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; fictional unit records.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, mkdtempSync, rmSync, rmdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { findCandidates } from '../iau_star_candidates.mjs';
import { buildMvpStarManifest } from '../build_mvp_star_manifest.mjs';
const bytes = (v) => Buffer.from(JSON.stringify(v));
const sha = (v) => createHash('sha256').update(v).digest('hex');
function fixture() {
  const row = Array(140).fill(' '); const put = (offset, s) => [...s].forEach((v, i) => { row[offset + i] = v; });
  put(0, 'Fictional'); put(65, 'A'); put(81, ' 1.20'); put(87, 'V'); put(90, '    123'); put(126, '2020-01-02');
  const iauBytes = Buffer.from(`#Name/ASCII Dec(J2000)\n${row.join('')}\n`);
  const candidate = findCandidates(iauBytes.toString(), ['Fictional', 'Missing'], 'https://example.org/catalog');
  candidate.catalogSha256 = sha(iauBytes);
  const candidatesBytes = bytes(candidate);
  const hip2Sha256 = 'a'.repeat(64);
  const qualityBytes = bytes({ schema: 'virgostars-hip2-quality-audit/1', status: 'REVIEW_REQUIRED',
    candidateReportSha256: sha(candidatesBytes), hip2Sha256, results: [{ requestedName: 'Fictional', hip: 123, strictEligible: true }] });
  return { candidatesBytes, qualityBytes, iauBytes, hip2Sha256, hip2Path: 'mock',
    inspectQuality: (_, hips) => hips.map((hip) => ({ hip, strictEligible: true })) };
}

test('builds a source-bound strict MVP subset with explicit zero-RV assumption', () => {
  const data = fixture(); const manifest = buildMvpStarManifest(data);
  assert.equal(manifest.stars.length, 1);
  assert.equal(manifest.stars[0].name, 'Fictional');
  assert.equal(manifest.stars[0].radialVelocityKmS, null);
  assert.equal(manifest.sourceHashes.hip2Sha256, data.hip2Sha256);
  assert.match(manifest.stars[0].radialVelocityMissingReason, /assumes zero/);
  assert.match(manifest.reviewer, /review pending/);
  assert.deepEqual(buildMvpStarManifest(data), manifest);
});

test('refuses changed source, changed candidate, and failed HIP2 replay', () => {
  const data = fixture();
  assert.throws(() => buildMvpStarManifest({ ...data, hip2Sha256: 'b'.repeat(64) }), /hash mismatch/);
  assert.throws(() => buildMvpStarManifest({ ...data, iauBytes: Buffer.from('changed') }), /hash mismatch/);
  const altered = JSON.parse(data.candidatesBytes); altered.results[0].candidates[0].magnitude = 100;
  const candidatesBytes = bytes(altered);
  const quality = JSON.parse(data.qualityBytes); quality.candidateReportSha256 = sha(candidatesBytes);
  assert.throws(() => buildMvpStarManifest({ ...data, candidatesBytes, qualityBytes: bytes(quality) }), /does not reproduce/);
  assert.throws(() => buildMvpStarManifest({ ...data, inspectQuality: () => [{ hip: 123, strictEligible: false }] }), /quality replay/);
});

const { STAR_REVIEW_INPUT_DIR, HIP2_PATH, MVP_STAR_MANIFEST } = process.env;
test('local 40-star MVP manifest reproduces exactly and refuses overwrite',
  { skip: STAR_REVIEW_INPUT_DIR && HIP2_PATH && MVP_STAR_MANIFEST ? false : 'local catalog inputs required' }, () => {
    const dir = mkdtempSync(join(tmpdir(), 'virgostars-mvp-manifest-')); const output = join(dir, 'manifest.json');
    const script = fileURLToPath(new URL('../build_mvp_star_manifest.mjs', import.meta.url));
    const args = [script, join(STAR_REVIEW_INPUT_DIR, 'iau-star-candidates.json'), join(STAR_REVIEW_INPUT_DIR, 'iau-star-quality.json'),
      join(STAR_REVIEW_INPUT_DIR, 'IAU-CSN-2022-mirror.txt'), HIP2_PATH, output];
    try {
      const run = spawnSync(process.execPath, args, { encoding: 'utf8' }); assert.equal(run.status, 0, run.stderr);
      const saved = readFileSync(output); assert.deepEqual(saved, readFileSync(MVP_STAR_MANIFEST));
      assert.equal(JSON.parse(saved).stars.length, 40);
      const retry = spawnSync(process.execPath, args, { encoding: 'utf8' }); assert.equal(retry.status, 1);
      assert.deepEqual(readFileSync(output), saved);
    } finally { rmSync(output, { force: true }); rmdirSync(dir); }
  });
