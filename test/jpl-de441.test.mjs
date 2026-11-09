/*
 * EXHIBIT A. Common Public Attribution License Version 1.0.
 * 
 * The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
 * 
 * Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
 * 
 * The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
 * 
 * The Original Developer is the Initial Developer.
 * 
 * The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
 * 
 * Contributors retain applicable rights in their own contributions under the terms of this License.
 * 
 * [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
 * SPDX-License-Identifier: CPAL-1.0
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, unlinkSync, rmdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { openDe441 } from '../src/jpl-de441.mjs';

// One synthetic record with two Mercury granules and analytical polynomials.
// T0=1, T1=x, T2=2x²-1; the header padding fixes a 3000-byte record.
function fixture() {
  const dir = mkdtempSync(join(tmpdir(), 'de441-'));
  const path = join(dir, 'fixture.eph');
  const size = 3000;
  const b = Buffer.alloc(size * 3);
  b.write('JPL Planetary Ephemeris DE441');
  b.writeDoubleLE(1000, 2652); b.writeDoubleLE(1032, 2660); b.writeDoubleLE(32, 2668);
  b.writeDoubleLE(81, 2688); b.writeInt32LE(441, 2840);
  // Mercury uses words 3..20; libration pointer expands the record to 375 words.
  b.writeInt32LE(3, 2696); b.writeInt32LE(3, 2700); b.writeInt32LE(2, 2704);
  b.writeInt32LE(373, 2844); b.writeInt32LE(1, 2848); b.writeInt32LE(1, 2852);
  b.writeDoubleLE(1000, 2 * size); b.writeDoubleLE(1032, 2 * size + 8);
  for (let g = 0; g < 2; g++) for (let a = 0; a < 3; a++) {
    const offset = 2 * size + (2 + (g * 3 + a) * 3) * 8;
    b.writeDoubleLE(100 * g + 10 * a, offset);
    b.writeDoubleLE(2, offset + 8);
    b.writeDoubleLE(3, offset + 16);
  }
  writeFileSync(path, b);
  return { path, dir };
}

test('interpolates position and derivative and handles granule and final boundaries', () => {
  const f = fixture();
  try {
    const eph = openDe441(f.path);
    assert.deepEqual(eph.state('mercury', 1000), { position: [1, 11, 21], velocity: [-1.25, -1.25, -1.25] });
    assert.deepEqual(eph.state('mercury', 1008), { position: [-3, 7, 17], velocity: [0.25, 0.25, 0.25] });
    assert.deepEqual(eph.state('mercury', 1016), { position: [101, 111, 121], velocity: [-1.25, -1.25, -1.25] });
    assert.deepEqual(eph.state('mercury', 1032), { position: [105, 115, 125], velocity: [1.75, 1.75, 1.75] });
    assert.throws(() => eph.state('mercury', 1032.1), RangeError);
    assert.throws(() => eph.state('toString', 1010), RangeError);
    assert.throws(() => eph.state('sun', 1010), /not present/);
    eph.close(); eph.close();
    assert.throws(() => eph.state('mercury', 1010), /closed/);
  } finally { unlinkSync(f.path); rmdirSync(f.dir); }
});

test('rejects truncated files', () => {
  const f = fixture();
  try {
    writeFileSync(f.path, Buffer.alloc(100));
    assert.throws(() => openDe441(f.path), /Truncated/);
  } finally { unlinkSync(f.path); rmdirSync(f.dir); }
});

// Set DE441_PATH to exercise a real external file; no ephemeris is redistributed.
test('real DE441 states obey Earth-Moon barycenter relationships', { skip: !process.env.DE441_PATH }, () => {
  const eph = openDe441(process.env.DE441_PATH);
  try {
    for (const jd of [eph.coverage.start, 2451545, eph.coverage.end]) {
      const earth = eph.state('earth', jd), moon = eph.state('moon', jd), geo = eph.state('moonGeocentric', jd);
      for (let i = 0; i < 3; i++) {
        assert.ok(Math.abs(moon.position[i] - earth.position[i] - geo.position[i]) < 1e-6);
        assert.ok(Math.abs(moon.velocity[i] - earth.velocity[i] - geo.velocity[i]) < 1e-8);
      }
    }
  } finally { eph.close(); }
});
