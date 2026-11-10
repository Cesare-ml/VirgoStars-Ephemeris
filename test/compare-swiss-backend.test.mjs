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
import swissData from '../scripts/swiss-reference-data.cjs';

const { manifestDigest } = swissData;

const fields = {
  longitude: 359.999, latitude: 0.2, distance: 1.1,
  longitudeSpeed: -0.4, latitudeSpeed: 0.002, distanceSpeed: -0.003,
};
const tolerances = {
  longitude: 0.003, latitude: 0.001, distance: 0.0001,
  longitudeSpeed: 0.0001, latitudeSpeed: 0.0001, distanceSpeed: 0.0001,
};
const resultContract = {
  center: 'geocentric',
  frame: 'true_ecliptic_of_date',
  corrections: 'apparent',
  inputTimeScale: 'UT',
  angleUnit: 'degree',
  distanceUnit: 'AU',
  rateUnit: 'per_day',
};
const input = { jdUt: 2461355.5, bodyId: 4, flags: 258 };
const syntheticFiles = [{ name: 'sepl_18.se1', bytes: 4, sha256: 'a'.repeat(64) }];
const digest = manifestDigest(syntheticFiles);
const reference = {
  schemaVersion: 1,
  source: {
    binding: 'swisseph@0.5.17', ephemeris: 'SWIEPH', dataId: digest,
    dataManifest: { algorithm: 'sha256', digest, files: syntheticFiles },
    resultContract,
  },
  cases: [{
    id: 'mars-wrap-synthetic', function: 'swe_calc_ut', input,
    effectiveFlags: 258, output: fields,
  }],
};
const candidate = {
  schemaVersion: 1,
  source: { engine: 'VirgoStars-Ephemeris', resultContract },
  cases: [{
    id: 'mars-wrap-synthetic', function: 'swe_calc_ut', input,
    output: { ...fields, longitude: 0.001 },
  }],
};

test('shortest circular longitude distance at 0/360', () => {
  assert.ok(Math.abs(angularDistanceDeg(359.999, 0.001) - 0.002) < 1e-10);
  assert.equal(angularDistanceDeg(0, 360), 0);
  assert.equal(angularDistanceDeg(0, -720), 0);
  assert.equal(angularDistanceDeg(-720, 0), 0);
  assert.equal(angularDistanceDeg(5, 725), 0);
  assert.equal(angularDistanceDeg(0, 540), 180);
  assert.equal(angularDistanceDeg(0, -181), 179);
});

test('identical inputs and synthetic six-field comparison pass', () => {
  const report = compareRecordedCases(reference, candidate, tolerances);
  assert.equal(report.totalCases, 1);
  assert.equal(report.totalFields, 6);
  assert.deepEqual(report.failures, []);
  assert.equal(report.maxErrors.longitude.id, 'mars-wrap-synthetic');
  assert.ok(Math.abs(report.maxErrors.longitude.delta - 0.002) < 1e-10);
  assert.equal(report.maxErrors.latitude.delta, 0);
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

test('a mislabeled geometric or J2000 candidate cannot masquerade as apparent-of-date', () => {
  const altered = structuredClone(candidate);
  altered.source.resultContract.corrections = 'geometric';
  assert.throws(() => compareRecordedCases(reference, altered, tolerances),
    /candidate: resultContract.corrections must be apparent/);
  altered.source.resultContract.corrections = 'apparent';
  altered.source.resultContract.frame = 'ICRF_J2000';
  assert.throws(() => compareRecordedCases(reference, altered, tolerances),
    /candidate: resultContract.frame must be true_ecliptic_of_date/);
  altered.source.resultContract.frame = 'true_ecliptic_of_date';
  altered.source.resultContract.distanceUnit = 'km';
  assert.throws(() => compareRecordedCases(reference, altered, tolerances),
    /candidate: resultContract.distanceUnit must be AU/);
});

test('source metadata must be asserted by Swiss as well as by the candidate', () => {
  const altered = structuredClone(reference);
  delete altered.source.resultContract;
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /reference: resultContract.center must be geocentric/);
});

test('only the observed backend P0 SWIEPH and SPEED flags are compared', () => {
  const altered = structuredClone(reference);
  altered.cases[0].input.flags = 2;
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /only P0 SWIEPH\|SPEED ecliptic mode is supported/);
  altered.cases[0].input.flags = 258 | 2048; // EQUATORIAL: separate field contract needed
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /only P0 SWIEPH\|SPEED ecliptic mode is supported/);
});

test('Swiss source must confirm speed as well as SWIEPH, not just return six fields', () => {
  const altered = structuredClone(reference);
  altered.cases[0].effectiveFlags = 2; // SWIEPH, but SEFLG_SPEED was dropped
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /did not honor requested speed flag/);
});

test('angular output is required to be normalized and distance nonnegative', () => {
  const altered = structuredClone(candidate);
  altered.cases[0].output.longitude = 360;
  assert.throws(() => compareRecordedCases(reference, altered, tolerances),
    /longitude outside/);
  altered.cases[0].output.longitude = 0.001;
  altered.cases[0].output.distance = -1;
  assert.throws(() => compareRecordedCases(reference, altered, tolerances),
    /invalid latitude\/distance/);
});

test('maximum errors identify the worst sample across multiple bodies/dates', () => {
  const ref = structuredClone(reference);
  const got = structuredClone(candidate);
  const extraRef = structuredClone(ref.cases[0]);
  const extraGot = structuredClone(got.cases[0]);
  extraRef.id = 'another-date-body-4';
  extraGot.id = 'another-date-body-4';
  extraRef.input.jdUt += 1;
  extraGot.input.jdUt += 1;
  extraRef.output.longitude = 45;
  extraGot.output.longitude = 45.05;
  ref.cases.push(extraRef);
  got.cases.push(extraGot);
  const report = compareRecordedCases(ref, got, tolerances);
  assert.equal(report.totalCases, 2);
  assert.equal(report.totalFields, 12);
  assert.equal(report.failures.length, 1);
  assert.equal(report.failures[0].id, 'another-date-body-4');
  assert.equal(report.maxErrors.longitude.id, 'another-date-body-4');
  assert.ok(Math.abs(report.maxErrors.longitude.delta - 0.05) < 1e-9);
});

test('Swiss reference manifest is required and validated against its digest', () => {
  const altered = structuredClone(reference);
  delete altered.source.dataManifest;
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /missing SHA-256 dataset manifest/);

  altered.source.dataManifest = structuredClone(reference.source.dataManifest);
  altered.source.dataManifest.files[0].sha256 = 'b'.repeat(64);
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /manifest digest and dataId disagree/);

  altered.source.dataManifest = structuredClone(reference.source.dataManifest);
  altered.source.dataId = 'c'.repeat(64);
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /manifest digest and dataId disagree/);
});

test('Swiss reference manifest rejects unsafe filenames and duplicate entries', () => {
  const altered = structuredClone(reference);
  altered.source.dataManifest.files[0].name = '../sepl_18.se1';
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /invalid Swiss .se1 manifest filename/);

  altered.source.dataManifest = structuredClone(reference.source.dataManifest);
  altered.source.dataManifest.files.push(structuredClone(altered.source.dataManifest.files[0]));
  assert.throws(() => compareRecordedCases(altered, candidate, tolerances),
    /manifest filenames must be unique and sorted/);
});
