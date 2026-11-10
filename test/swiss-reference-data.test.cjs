// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { mkdtemp, rm, writeFile } = require('node:fs/promises');
const { tmpdir } = require('node:os');
const path = require('node:path');
const test = require('node:test');
const { swissDataManifest, manifestDigest } = require('../scripts/swiss-reference-data.cjs');

async function tempDir(t) {
  const dir = await mkdtemp(path.join(tmpdir(), 'virgostars-swiss-manifest-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  return dir;
}

test('manifest deterministically sorts and hashes external .se1 files', async t => {
  const dir = await tempDir(t);
  await writeFile(path.join(dir, 'sepl_18.se1'), 'planetary test bytes');
  await writeFile(path.join(dir, 'semo_18.se1'), 'moon test bytes');
  await writeFile(path.join(dir, 'ignored.txt'), 'not part of P0 ephemeris snapshot');
  const manifest = await swissDataManifest(dir);
  assert.equal(manifest.algorithm, 'sha256');
  assert.deepEqual(manifest.files.map(file => file.name), ['semo_18.se1', 'sepl_18.se1']);
  assert.deepEqual(manifest.files.map(file => file.bytes), [15, 20]);
  assert.equal(manifest.files[0].sha256,
    createHash('sha256').update('moon test bytes').digest('hex'));
  assert.equal(manifest.digest, manifestDigest(manifest.files));
  assert.equal(manifest.digest, (await swissDataManifest(dir)).digest);
  assert.ok(!JSON.stringify(manifest).includes(dir), 'absolute paths must never be exposed');
});

test('manifest fingerprint detects changed reference ephemeris bytes', async t => {
  const dir = await tempDir(t);
  const file = path.join(dir, 'sepl_18.se1');
  await writeFile(file, 'original');
  const before = await swissDataManifest(dir);
  await writeFile(file, 'modified');
  const after = await swissDataManifest(dir);
  assert.notEqual(before.files[0].sha256, after.files[0].sha256);
  assert.notEqual(before.digest, after.digest);
  assert.equal(before.files[0].bytes, after.files[0].bytes);
});

test('manifest refuses absent or non-directory .se1 input', async t => {
  const dir = await tempDir(t);
  await assert.rejects(swissDataManifest(dir), /No regular .se1 files/);
  await writeFile(path.join(dir, 'only.txt'), 'irrelevant');
  await assert.rejects(swissDataManifest(dir), /No regular .se1 files/);
  const file = path.join(dir, 'only.txt');
  await assert.rejects(swissDataManifest(file), /must be a directory/);
  await assert.rejects(swissDataManifest(''), /directory is required/);
});
