'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Swiss supplies an illustrative model, not a validated historical uncertainty.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {Engine, julday} = require('../index.js');
const YEARS = [-12999, -10000, -5000, 0, 2000, 3999];
const SHIFT_SECONDS = 3600; // Scenario only: not an estimated Delta T error.
const wrap = value => ((value % 360) + 540) % 360 - 180;
async function sha(file) {
  const hash = crypto.createHash('sha256');
  for await (const chunk of fs.createReadStream(file)) hash.update(chunk);
  return hash.digest('hex');
}
async function main() {
  const [kernel, expected, swissPackage, output, ...extra] = process.argv.slice(2);
  if (!kernel || !/^[a-f0-9]{64}$/.test(expected || '') || !swissPackage || !output || extra.length
      || path.resolve(kernel) === path.resolve(output) || fs.existsSync(output))
    throw new Error('usage: <de441> <sha256> <swisseph> <new-report>');
  if (await sha(kernel) !== expected) throw new Error('kernel hash mismatch');
  const swiss = require(path.resolve(swissPackage));
  swiss.swe_set_ephe_path(path.dirname(path.resolve(kernel)));
  swiss.swe_set_jpl_file(path.basename(kernel));
  const engine = new Engine(path.resolve(kernel)), rows = [];
  for (const year of YEARS) {
    const jdUt1 = julday(year, 7, 1, 12, true);
    const deltaDays = swiss.swe_deltat(jdUt1)?.delta;
    if (!Number.isFinite(deltaDays)) throw new Error(`invalid Delta T for ${year}`);
    const deltaTSeconds = deltaDays * 86400;
    const source = 'local Swiss Delta T; illustrative scenario, not a validated uncertainty';
    const center = engine.experimentalLongTermApparentUt1(jdUt1, 1, deltaTSeconds, SHIFT_SECONDS, source);
    const plus = engine.experimentalLongTermApparentUt1(jdUt1, 1, deltaTSeconds + SHIFT_SECONDS, SHIFT_SECONDS, source);
    const ref = swiss.swe_calc_ut(jdUt1, 1, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED);
    if (!ref || ref.error || !(ref.rflag & swiss.SEFLG_JPLEPH) || !Number.isFinite(ref.longitude)
        || !center.xx.every(Number.isFinite) || !plus.xx.every(Number.isFinite))
      throw new Error(`invalid Moon calculation or JPL flag for ${year}`);
    rows.push({year, jdUt1, deltaTSeconds, jdTt: center.jdTt, jdTdb: center.jdTdb,
      moonLongitudeDeg: center.xx[0], swissMoonLongitudeDeg: ref.longitude,
      moonLongitudeDifferenceArcsec: wrap(center.xx[0] - ref.longitude) * 3600,
      plusOneHourMoonLongitudeShiftArcmin: wrap(plus.xx[0] - center.xx[0]) * 60});
  }
  if (await sha(kernel) !== expected) throw new Error('kernel changed');
  const report = {schema:'virgostars-ancient-ut1-sensitivity/1', status:'EXPLORATORY',
    kernelSha256:expected, years:YEARS, shiftSeconds:SHIFT_SECONDS,
    notes:'Proleptic Gregorian UT1; Swiss Delta T is a model illustration, not observed truth; +1h is a sensitivity scenario, not a confidence interval. Both use the same DE441; TT/TDB is approximate.', rows};
  fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', {flag:'wx'});
  console.log(JSON.stringify({cases:rows.length, minYear:YEARS[0], maxYear:YEARS.at(-1),
    firstDeltaTHours:rows[0].deltaTSeconds / 3600, firstPlusOneHourMoonShiftArcmin:rows[0].plusOneHourMoonLongitudeShiftArcmin}));
}
main().catch(error => { console.error(error.message); process.exitCode = 1; });
