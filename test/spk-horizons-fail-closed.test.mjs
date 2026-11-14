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

import test from 'node:test';
import assert from 'node:assert/strict';
import { stateAtTdbSeconds, inspectSpk } from '../src/index.mjs';
import { parseHorizonsVector } from '../tools/horizons.mjs';

// Tiny original synthetic DAF/SPK fixture. It is NOT JPL reference data.
function spkFixture(segments, bigEndian = false) {
  const bytes = Buffer.alloc(4096);
  const i32 = (value, offset) => bigEndian
    ? bytes.writeInt32BE(value, offset) : bytes.writeInt32LE(value, offset);
  const f64 = (value, offset) => bigEndian
    ? bytes.writeDoubleBE(value, offset) : bytes.writeDoubleLE(value, offset);

  bytes.write('DAF/SPK ', 0, 'ascii');
  i32(2, 8);
  i32(6, 12);
  i32(2, 76);
  i32(2, 80);
  i32(600, 84);
  bytes.write(bigEndian ? 'BIG-IEEE' : 'LTL-IEEE', 88, 'ascii');
  f64(0, 1024);
  f64(0, 1032);
  f64(segments.length, 1040);

  let word = 385;
  for (const [index, spec] of segments.entries()) {
    if (index >= 25) throw new Error('Too many synthetic segments');
    const offset = 1024 + 24 + index * 40;
    f64(spec.startEt ?? -10, offset);
    f64(spec.endEt ?? 10, offset + 8);
    [spec.target, spec.center ?? 0, spec.frame ?? 1, spec.type ?? 2, word, word + 11]
      .forEach((value, j) => i32(value, offset + 16 + 4 * j));
    bytes.write(spec.name, 2048 + index * 40, 'ascii');
    [0, 10, spec.x, 0, 0, 0, 0, 0, -10, 20, 8, 1]
      .forEach((value, j) => f64(value, (word + j - 1) * 8));
    word += 12;
  }
  return bytes;
}

const oldSun = { target: 10, name: 'OLD-SUN', x: 10 };
const newSun = { target: 10, name: 'NEW-SUN', x: 20 };

test('last covering SPK segment wins for the same target, at inclusive boundaries', () => {
  for (const bigEndian of [false, true]) {
    const bytes = spkFixture([oldSun, newSun], bigEndian);
    assert.deepEqual(inspectSpk(bytes).segments.map(s => s.name), ['OLD-SUN', 'NEW-SUN']);
    for (const et of [-10, 0, 10]) {
      const state = stateAtTdbSeconds(bytes, 10, 0, et);
      assert.equal(state.positionKm[0], 20);
      assert.deepEqual(state.sourceSegments, ['NEW-SUN']);
    }
  }
});

test('non-covering later segment does not override earlier covering segment', () => {
  const bytes = spkFixture([oldSun, { ...newSun, startEt: 5, endEt: 10 }]);
  assert.equal(stateAtTdbSeconds(bytes, 10, 0, 0).positionKm[0], 10);
  assert.equal(stateAtTdbSeconds(bytes, 10, 0, 5).positionKm[0], 20);
});

test('unsupported high-priority frame fails closed rather than falling back', () => {
  for (const bigEndian of [false, true]) {
    const bytes = spkFixture([oldSun, { ...newSun, frame: 100 }], bigEndian);
    for (const et of [-10, 0, 10]) {
      assert.throws(
        () => stateAtTdbSeconds(bytes, 10, 0, et),
        error => error.code === 'UNSUPPORTED_SPK_FRAME'
      );
    }
  }
});

test('high-priority segment center determines chain, not earlier target center', () => {
  const bytes = spkFixture([
    { target: 3, name: 'EMB', x: 5 },
    oldSun,
    { ...newSun, center: 3 },
  ]);
  assert.equal(stateAtTdbSeconds(bytes, 10, 0, 0).positionKm[0], 25);
  assert.deepEqual(stateAtTdbSeconds(bytes, 10, 0, 0).sourceSegments, ['NEW-SUN', 'EMB']);
  assert.equal(stateAtTdbSeconds(bytes, 10, 3, 0).positionKm[0], 20);
});

test('unsupported high-priority SPK type fails closed', () => {
  const bytes = spkFixture([oldSun, { ...newSun, type: 99 }]);
  assert.throws(
    () => stateAtTdbSeconds(bytes, 10, 0, 0),
    error => error.code === 'UNSUPPORTED_SPK_TYPE'
  );
});

const vector = '2451545.0, A.D. 2000-Jan-01 12:00, 1.0, -2.0, 3.0, 0.1, -2.0E-1, +.3,';
const response = line => 'Header\n$$SOE\n' + line + '\n$$EOE\n';

test('strict Horizons parser accepts a complete scientific-notation row', () => {
  assert.deepEqual(
    parseHorizonsVector(response(vector), { expectedJdTdb: 2451545.0 }).velocityKmPerSec,
    [0.1, -0.2, 0.3]
  );
});

test('strict Horizons parser rejects any absent or malformed mandatory numeric field', () => {
  const fields = vector.slice(0, -1).split(',').map(x => x.trim());
  for (const position of [0, 2, 3, 4, 5, 6, 7]) {
    for (const bad of ['', ' ', 'NaN', 'Infinity', '0x10', 'oops']) {
      const copy = [...fields];
      copy[position] = bad;
      assert.throws(
        () => parseHorizonsVector(response(copy.join(',') + ',')),
        error => error.code === 'INVALID_HORIZONS',
        'position ' + position + ': ' + JSON.stringify(bad)
      );
    }
  }
  assert.throws(() => parseHorizonsVector(response(vector.replace('A.D. 2000-Jan-01 12:00', ''))),
    error => error.code === 'INVALID_HORIZONS');
  assert.throws(() => parseHorizonsVector(response(vector + 'extra,')),
    error => error.code === 'INVALID_HORIZONS');
});

test('strict Horizons parser rejects multiple rows and mismatched requested epoch', () => {
  assert.throws(
    () => parseHorizonsVector(response(vector + '\n' + vector)),
    error => error.code === 'INVALID_HORIZONS'
  );
  assert.throws(
    () => parseHorizonsVector(response(vector), { expectedJdTdb: 2451545.1 }),
    error => error.code === 'INVALID_HORIZONS'
  );
  assert.throws(
    () => parseHorizonsVector(response(vector), { expectedJdTdb: NaN }),
    error => error.code === 'INVALID_ARGUMENT'
  );
});
