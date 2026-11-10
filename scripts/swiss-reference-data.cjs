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

// Independent test-data provenance helper; no Swiss code or kernel is loaded.
// Deliberately scans only regular *.se1 files in the configured directory.
// A streaming hash keeps memory bounded with large ephemeris files.
const { createHash } = require('node:crypto');
const { createReadStream } = require('node:fs');
const { readdir, stat } = require('node:fs/promises');
const path = require('node:path');

async function sha256File(filePath) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(filePath)) {
    hash.update(chunk);
  }
  return hash.digest('hex');
}

async function swissDataManifest(directory) {
  if (typeof directory !== 'string' || directory.trim() === '') {
    throw new Error('Swiss ephemeris directory is required');
  }
  const root = path.resolve(directory);
  const directoryStat = await stat(root);
  if (!directoryStat.isDirectory()) {
    throw new Error('Swiss ephemeris path must be a directory');
  }

  const names = (await readdir(root, { withFileTypes: true }))
    .filter(entry => entry.isFile() && /\.se1$/i.test(entry.name))
    .map(entry => entry.name)
    .sort();
  if (names.length === 0) {
    throw new Error('No regular .se1 files in Swiss ephemeris directory');
  }

  const files = [];
  for (const name of names) {
    const file = path.join(root, name);
    const initial = await stat(file);
    if (!initial.isFile()) throw new Error(`Not a regular Swiss data file: ${name}`);
    const sha256 = await sha256File(file);
    const final = await stat(file);
    if (initial.size !== final.size || initial.mtimeMs !== final.mtimeMs) {
      throw new Error(`Swiss ephemeris file changed during hashing: ${name}`);
    }
    files.push({ name, bytes: initial.size, sha256 });
  }

  // Canonical JSON layout (sorted names, fixed property order). The aggregate
  // SHA-256 identifies this recorded directory snapshot, not the exact subset
  // that Swiss loaded at runtime.
  const digest = createHash('sha256')
    .update(JSON.stringify(files), 'utf8')
    .digest('hex');
  return { algorithm: 'sha256', digest, files };
}

module.exports = { swissDataManifest };
