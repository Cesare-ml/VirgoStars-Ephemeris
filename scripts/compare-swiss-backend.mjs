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

import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

// Offline-only comparison of independently exported JSON files.
// No Swiss library, external kernel or Rust binding is loaded by this tool.
const FIELDS = [
  'longitude', 'latitude', 'distance',
  'longitudeSpeed', 'latitudeSpeed', 'distanceSpeed',
];
const EPH_MASK = 1 | 2 | 4;
const SWIEPH = 2;
const SPEED = 256;
// These labels are an assertion made by each exporter, not proof of accuracy.
// They prevent a geometric/ICRF state from being silently compared as apparent.
const RESULT_CONTRACT = Object.freeze({
  center: 'geocentric',
  frame: 'true_ecliptic_of_date',
  corrections: 'apparent',
  inputTimeScale: 'UT',
  angleUnit: 'degree',
  distanceUnit: 'AU',
  rateUnit: 'per_day',
});

function ensure(ok, message) {
  if (!ok) throw new Error(message);
}

function finiteNumber(value) {
  return typeof value === 'number' && Number.isFinite(value);
}

export function angularDistanceDeg(left, right) {
  ensure(finiteNumber(left) && finiteNumber(right), 'non-finite angle');
  // JavaScript's remainder keeps the dividend's sign; normalize each angle
  // before taking the shortest signed separation (always <= 180 degrees).
  const normalize = value => ((value % 360) + 360) % 360;
  return Math.abs(((normalize(left) - normalize(right) + 540) % 360) - 180);
}

function requireResultContract(source, label) {
  for (const [field, expected] of Object.entries(RESULT_CONTRACT)) {
    ensure(source?.resultContract?.[field] === expected,
      `${label}: resultContract.${field} must be ${expected}`);
  }
}

function indexCases(document, label) {
  ensure(document?.schemaVersion === 1, `${label}: unsupported schemaVersion`);
  ensure(Array.isArray(document.cases) && document.cases.length > 0, `${label}: no cases`);
  const cases = new Map();
  for (const testCase of document.cases) {
    ensure(typeof testCase?.id === 'string' && testCase.id.length > 0,
      `${label}: missing case id`);
    ensure(!cases.has(testCase.id), `${label}: duplicate case id ${testCase.id}`);
    ensure(testCase.function === 'swe_calc_ut',
      `${label}: unsupported function for ${testCase.id}`);
    const input = testCase.input;
    ensure(finiteNumber(input?.jdUt) && Number.isInteger(input?.bodyId) &&
      Number.isInteger(input?.flags), `${label}: invalid input in ${testCase.id}`);
    ensure(input.flags === (SWIEPH | SPEED),
      `${label}: only P0 SWIEPH|SPEED ecliptic mode is supported in ${testCase.id}`);
    for (const field of FIELDS) {
      ensure(finiteNumber(testCase.output?.[field]),
        `${label}: missing/non-finite ${field} for ${testCase.id}`);
    }
    ensure(testCase.output.longitude >= 0 && testCase.output.longitude < 360,
      `${label}: longitude outside [0, 360) for ${testCase.id}`);
    ensure(Math.abs(testCase.output.latitude) <= 90 && testCase.output.distance >= 0,
      `${label}: invalid latitude/distance for ${testCase.id}`);
    cases.set(testCase.id, testCase);
  }
  return cases;
}

/** Compare two exported runs; tolerance units: degrees, degrees/day, AU, AU/day. */
export function compareRecordedCases(reference, candidate, tolerances) {
  ensure(reference?.source?.binding === 'swisseph@0.5.17',
    'reference must identify the backend Node binding swisseph@0.5.17');
  ensure(reference?.source?.ephemeris === 'SWIEPH' &&
    typeof reference?.source?.dataId === 'string' &&
    reference.source.dataId.length > 0,
  'reference must identify an explicit SWIEPH data set');
  ensure(candidate?.source?.engine === 'VirgoStars-Ephemeris',
    'candidate must identify the independently implemented engine');
  requireResultContract(reference?.source, 'reference');
  requireResultContract(candidate?.source, 'candidate');

  for (const field of FIELDS) {
    ensure(finiteNumber(tolerances?.[field]) && tolerances[field] >= 0,
      `missing or invalid tolerance for ${field}`);
  }

  const refs = indexCases(reference, 'reference');
  const actual = indexCases(candidate, 'candidate');
  ensure(refs.size === actual.size, 'case counts differ');

  const failures = [];
  const maxErrors = Object.fromEntries(
    FIELDS.map(field => [field, { delta: 0, id: null }]),
  );
  for (const [id, expected] of refs) {
    const got = actual.get(id);
    ensure(got, `candidate missing case ${id}`);
    ensure(Number.isInteger(expected.effectiveFlags) &&
      (expected.effectiveFlags & EPH_MASK) === SWIEPH,
    `reference ${id} did not verify SWIEPH effective flags (possible fallback)`);
    ensure((expected.effectiveFlags & SPEED) === (expected.input.flags & SPEED),
      `reference ${id} did not honor requested speed flag`);
    for (const key of ['jdUt', 'bodyId', 'flags']) {
      ensure(Object.is(expected.input[key], got.input[key]),
        `${id}: input ${key} does not match`);
    }
    for (const field of FIELDS) {
      const delta = field === 'longitude'
        ? angularDistanceDeg(expected.output[field], got.output[field])
        : Math.abs(expected.output[field] - got.output[field]);
      if (maxErrors[field].id === null || delta > maxErrors[field].delta) {
        maxErrors[field] = { delta, id };
      }
      if (delta > tolerances[field]) {
        failures.push({ id, field, delta, tolerance: tolerances[field] });
      }
    }
  }
  return { totalCases: refs.size, totalFields: refs.size * FIELDS.length, failures, maxErrors };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    ensure(process.argv.length === 5,
      'usage: node scripts/compare-swiss-backend.mjs swiss.json candidate.json tolerances.json');
    const [, , referencePath, candidatePath, tolerancePath] = process.argv;
    const load = (path) => JSON.parse(readFileSync(path, 'utf8'));
    const result = compareRecordedCases(
      load(referencePath), load(candidatePath), load(tolerancePath),
    );
    for (const [field, worst] of Object.entries(result.maxErrors)) {
      console.log(`MAX ${field}: ${worst.delta} (case ${worst.id})`);
    }
    if (result.failures.length) {
      for (const failure of result.failures) console.error(JSON.stringify(failure));
      console.error(`FAIL: ${result.failures.length}/${result.totalFields} fields differ`);
      process.exitCode = 1;
    } else {
      console.log(`PASS: ${result.totalCases} cases, ${result.totalFields} fields compared`);
    }
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
