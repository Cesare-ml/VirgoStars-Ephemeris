'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Manual network diagnostic. Sends only public body IDs and TDB epochs to JPL.
// The output is external evidence, not bundled data or an apparent-position test.
const fs = require('node:fs');
const crypto = require('node:crypto');
const { Engine, julday } = require('../index.js');

const bodies = [
  [0, '10', 'Sun'], [1, '301', 'Moon'], [2, '199', 'Mercury'], [3, '299', 'Venus'],
  [4, '4', 'Mars Barycenter'], [5, '5', 'Jupiter Barycenter'], [6, '6', 'Saturn Barycenter'],
  [7, '7', 'Uranus Barycenter'], [8, '8', 'Neptune Barycenter'], [9, '9', 'Pluto Barycenter'],
];
const years = [0, 1000, 2000, 3000, 4000, 5000];
// Diagnostic thresholds fixed before querying, not release-acceptance criteria.
const positionLimitKm = 0.001;
const velocityLimitKmDay = 0.001;

async function sha256(path) {
  const h = crypto.createHash('sha256');
  for await (const chunk of fs.createReadStream(path)) h.update(chunk);
  return h.digest('hex');
}

async function main() {
  const [kernel, expectedSha, output, ...extra] = process.argv.slice(2);
  if (!kernel || !/^[a-f0-9]{64}$/.test(expectedSha || '') || !output || extra.length) {
    throw new Error('usage: node compare-horizons-geometric-range.cjs <kernel> <sha256> <new-output.json>');
  }
  if (fs.existsSync(output)) throw new Error('output already exists');
  if (await sha256(kernel) !== expectedSha) throw new Error('kernel hash mismatch');
  const engine = new Engine(kernel);
  const epochs = years.map(year => julday(year, 7, 1, 12, true));
  const results = [];
  for (const [id, command, name] of bodies) {
    const params = new URLSearchParams({ format: 'json', COMMAND: `'${command}'`, OBJ_DATA: "'NO'", MAKE_EPHEM: "'YES'",
      EPHEM_TYPE: "'VECTORS'", CENTER: "'500@399'", TLIST: `'${epochs.join(' ')}'`, TLIST_TYPE: "'JD'", TIME_TYPE: "'TDB'",
      REF_SYSTEM: "'ICRF'", REF_PLANE: "'FRAME'", VEC_CORR: "'NONE'", VEC_TABLE: "'2'",
      OUT_UNITS: "'KM-D'", CSV_FORMAT: "'YES'" });
    const url = `https://ssd.jpl.nasa.gov/api/horizons.api?${params}`;
    const response = await fetch(url, { signal: AbortSignal.timeout(30000) });
    if (!response.ok) throw new Error(`Horizons HTTP ${response.status} for ${name}`);
    const json = await response.json();
    const raw = json.result;
    if (typeof raw !== 'string' || !raw.includes(`Target body name: ${name} (${command})`)
        || !/Target body name:.*\{source: DE441\}/.test(raw)
        || !/Center body name: Earth \(399\).*\{source: DE441\}/.test(raw)
        || !raw.includes('Output units    : KM-D') || !raw.includes('Output type     : GEOMETRIC cartesian states')
        || !raw.includes('Reference frame : ICRF')) throw new Error(`unexpected Horizons metadata for ${name}`);
    const start = raw.indexOf('$$SOE'); const end = raw.indexOf('$$EOE');
    if (start < 0 || end <= start) throw new Error(`missing Horizons table for ${name}`);
    const rows = raw.slice(start + 5, end).trim().split('\n');
    if (rows.length !== epochs.length) throw new Error(`wrong row count for ${name}`);
    let maxPositionKm = 0; let maxVelocityKmDay = 0;
    for (let i = 0; i < rows.length; i++) {
      const fields = rows[i].split(',');
      const jd = Number(fields[0]); const vector = fields.slice(2, 8).map(Number);
      if (jd !== epochs[i] || vector.length !== 6 || fields.slice(2, 8).some(value => !value.trim())
          || !vector.every(Number.isFinite)) throw new Error(`bad Horizons row for ${name}`);
      const got = engine.geometricStateTdb(jd, id);
      for (let j = 0; j < 3; j++) {
        maxPositionKm = Math.max(maxPositionKm, Math.abs(got.positionKm[j] - vector[j]));
        maxVelocityKmDay = Math.max(maxVelocityKmDay, Math.abs(got.velocityKmPerDay[j] - vector[j + 3]));
      }
    }
    results.push({ name, command, maxPositionKm, maxVelocityKmDay, url, raw });
    console.log(`${name}: ${maxPositionKm} km, ${maxVelocityKmDay} km/day`);
  }
  if (await sha256(kernel) !== expectedSha) throw new Error('kernel changed during comparison');
  const passed = results.every(r => r.maxPositionKm <= positionLimitKm && r.maxVelocityKmDay <= velocityLimitKmDay);
  fs.writeFileSync(output, JSON.stringify({ schema: 'virgostars-horizons-geometric-range/1', kernelSha256: expectedSha,
    years, epochsTdb: epochs, positionLimitKm, velocityLimitKmDay, passed, results }, null, 2) + '\n', { flag: 'wx' });
  if (!passed) throw new Error('diagnostic thresholds exceeded');
}
main().catch(error => { console.error(error.message); process.exitCode = 1; });
