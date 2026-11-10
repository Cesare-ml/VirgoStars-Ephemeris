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

// Optional, manually executed Swiss REFERENCE recorder (never part of Rust core/CI).
// Requires a separately licensed swisseph@0.5.17 Node module and local ephemeris.
// Nothing from the Swiss implementation or data files is committed here.
const { statSync } = require('node:fs');
const path = require('node:path');

function fail(message) { throw new Error(message); }
function run() {
  const ephePath = process.env.ASTRODIENST_EPHE_PATH;
  const dataId = process.env.SWISS_EPHE_DATA_ID;
  if (!ephePath || !statSync(path.resolve(ephePath)).isDirectory()) {
    fail('ASTRODIENST_EPHE_PATH must point to an existing Swiss data directory');
  }
  if (!dataId || !dataId.trim()) {
    fail('Set SWISS_EPHE_DATA_ID to a reproducible dataset manifest/hash identifier');
  }

  const swe = require('swisseph');
  const installedVersion = require('swisseph/package.json').version;
  if (installedVersion !== '0.5.17') fail(`Expected backend swisseph@0.5.17, got ${installedVersion}`);
  swe.swe_set_ephe_path(path.resolve(ephePath));

  const dates = [
    [2000, 1, 1, 12, 0],
    [2024, 2, 29, 23, 45],
    [2025, 3, 15, 12, 0],
    [2026, 11, 10, 18, 30],
  ];
  const flags = swe.SEFLG_SPEED | swe.SEFLG_SWIEPH;
  const cases = [];
  for (const [year, month, day, hour, minute] of dates) {
    const jdUt = swe.swe_julday(year, month, day, hour + minute / 60, swe.SE_GREG_CAL);
    if (!Number.isFinite(jdUt)) fail('Non-finite Swiss Julian date');
    for (let bodyId = swe.SE_SUN; bodyId <= swe.SE_SATURN; bodyId++) {
      const result = swe.swe_calc_ut(jdUt, bodyId, flags);
      const id = `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}T${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}Z-body-${bodyId}`;
      if (!result || result.error) fail(`${id}: Swiss error ${result?.error}`);
      if (!Number.isInteger(result.rflag) || (result.rflag & 7) !== swe.SEFLG_SWIEPH) {
        fail(`${id}: SWIEPH fallback or missing rflag: ${result.rflag}`);
      }
      const output = {};
      for (const key of [
        'longitude', 'latitude', 'distance',
        'longitudeSpeed', 'latitudeSpeed', 'distanceSpeed',
      ]) {
        if (typeof result[key] !== 'number' || !Number.isFinite(result[key])) {
          fail(`${id}: missing/non-finite ${key}`);
        }
        output[key] = result[key];
      }
      cases.push({
        id, function: 'swe_calc_ut',
        input: { jdUt, bodyId, flags },
        effectiveFlags: result.rflag,
        output,
      });
    }
  }
  process.stdout.write(JSON.stringify({
    schemaVersion: 1,
    source: {
      binding: 'swisseph@0.5.17',
      ephemeris: 'SWIEPH',
      dataId,
      generatedAtUtc: new Date().toISOString(),
      inputConvention: 'Gregorian UTC wall clock passed as UT JD (legacy backend)',
      positionConvention: 'apparent geocentric true ecliptic/equinox of date',
    },
    cases,
  }, null, 2) + '\n');
}
try { run(); } catch (error) { console.error(error.message); process.exitCode = 1; }
