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

import assert from 'node:assert/strict';
import test from 'node:test';
import { angularDistanceDeg, compareRecordedCases } from '../scripts/compare-swiss-backend.mjs';

const fields = {
  longitude: 359.999, latitude: 0.2, distance: 1.1,
  longitudeSpeed: -0.4, latitudeSpeed: 0.002, distanceSpeed: -0.003,
};
const tolerances = {
  longitude: 0.003, latitude: 0.001, distance: 0.0001,
  longitudeSpeed: 0.0001, latitudeSpeed: 0.0001, distanceSpeed: 0.0001,
};
const input = { jdUt: 2461355.5, bodyId: 4, flags: 258 };
const reference = {
  schemaVersion: 1,
  source: { binding: 'swisseph@0.5.17', ephemeris: 'SWIEPH', dataId: 'synthetic-only' },
  cases: [{
    id: 'mars-wrap-synthetic', function: 'swe_calc_ut', input,
    effectiveFlags: 258, output: fields,
  }],
};
const candidate = {
  schemaVersion: 1,
  source: { engine: 'VirgoStars-Ephemeris' },
  cases: [{
    id: 'mars-wrap-synthetic', function: 'swe_calc_ut', input,
    output: { ...fields, longitude: 0.001 },
  }],
};

test('shortest circular longitude distance at 0/360', () => {
  assert.ok(Math.abs(angularDistanceDeg(359.999, 0.001) - 0.002) < 1e-10);
  assert.equal(angularDistanceDeg(0, 360), 0);
});

test('identical inputs and synthetic six-field comparison pass', () => {
  const report = compareRecordedCases(reference, candidate, tolerances);
  assert.equal(report.totalCases, 1);
  assert.equal(report.totalFields, 6);
  assert.deepEqual(report.failures, []);
});

test('retrograde speed sign differences fail', () => {
  const altered = structuredClone(candidate);
  altered.cases[0].output.longitudeSpeed = 0.4;
  const report = compareRecordedCases(reference, altered, tolerances);
  assert.deepEqual(report.failures.map(({ field }) => field), ['longitudeSpeed']);
});

test('input/body mismatch is rejected before comparing outputs', () => {
  const altered = structuredClone(candidate);
  altered.cases[0].input.bodyId = 1;
  assert.throws(() => compareRecordedCases(reference, altered, tolerances),
    /input bodyId does not match/);
});

test('missing effective Swiss flags or a silent Moshier fallback fails', () => {
  const altered = structuredClone(reference);
  altered.cases[0].effectiveFlags = 260; // SEFLG_MOSEPH | SEFLG_SPEED
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /possible fallback/);
  delete altered.cases[0].effectiveFlags;
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /possible fallback/);
});

test('missing fields and duplicate case IDs fail', () => {
  const altered = structuredClone(candidate);
  delete altered.cases[0].output.latitudeSpeed;
  assert.throws(() => compareRecordedCases(reference, altered, tolerances),
    /missing\/non-finite latitudeSpeed/);
  altered.cases[0] = structuredClone(candidate.cases[0]);
  altered.cases.push(structuredClone(candidate.cases[0]));
  assert.throws(() => compareRecordedCases(reference, altered, tolerances),
    /duplicate case id/);
});
