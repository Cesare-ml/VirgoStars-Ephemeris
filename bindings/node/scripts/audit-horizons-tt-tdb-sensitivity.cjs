'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// A model sensitivity experiment, not a physical ancient time error bound.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');
const { Engine } = require('../index.js');
const { epochGrid, urlFor } = require('./validate-horizons-planet-holdout.cjs');
const { ttToTdb, direction } = require('./validate-horizons-planet-frame-grid.cjs');
const { parseIcrfTable } = require('./horizons-icrf-parser.cjs');
const { unit, separation } = require('./compare-horizons-frame-isolation.cjs');
const bodies = [[0, '10', 'Sun'], [1, '301', 'Moon'], [2, '199', 'Mercury'], [3, '299', 'Venus'], [4, '4', 'Mars Barycenter'], [5, '5', 'Jupiter Barycenter'], [6, '6', 'Saturn Barycenter'], [7, '7', 'Uranus Barycenter'], [8, '8', 'Neptune Barycenter'], [9, '9', 'Pluto Barycenter']];

function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function summarize(rows) {
  if (!Array.isArray(rows) || !rows.length || !rows.every(r => Number.isInteger(r.id) && r.id >= 0 && r.id <= 9 && Number.isInteger(r.year) && [r.offsetDifferenceMs, r.roundedDifferenceMs, r.timeImpactArcsec, r.oldResidualArcsec, r.erfaResidualArcsec].every(Number.isFinite) && [r.timeImpactArcsec, r.oldResidualArcsec, r.erfaResidualArcsec].every(x => x >= 0))) throw new Error('invalid rows');
  const max = (group, key, absolute = false) => {
    if (!group.length) throw new Error('empty group');
    const r = group.reduce((a, b) => (absolute ? Math.abs(b[key]) : b[key]) > (absolute ? Math.abs(a[key]) : a[key]) ? b : a);
    return { value: r[key], year: r.year, month: r.month, id: r.id };
  };
  const group = list => ({ cases: list.length, maxImpact: max(list, 'timeImpactArcsec'), maxOldResidual: max(list, 'oldResidualArcsec'), maxErfaResidual: max(list, 'erfaResidualArcsec'), maxResidualChange: max(list.map(r => ({ ...r, residualChangeArcsec: r.erfaResidualArcsec - r.oldResidualArcsec })), 'residualChangeArcsec', true) });
  return { cases: rows.length, maxOffsetDifferenceMs: max(rows, 'offsetDifferenceMs', true), maxRoundedDifferenceMs: max(rows, 'roundedDifferenceMs', true), moon: group(rows.filter(r => r.id === 1)), other: group(rows.filter(r => r.id !== 1)) };
}
function main() {
  const [kernel, kernelHash, source, sourceHash, python, out, ...extra] = process.argv.slice(2);
  if (!kernel || !source || !python || !out || extra.length || ![kernelHash, sourceHash].every(x => /^[a-f0-9]{64}$/.test(x || '')) || fs.existsSync(out)) throw new Error('usage: KERNEL KERNEL_SHA HORIZONS_REPORT REPORT_SHA PYTHON NEW_REPORT');
  const addon = path.resolve(__dirname, '../native/virgostars_ephemeris.node'), helper = path.resolve(__dirname, '../../../scripts/compare_erfa_tdb.py');
  const inputs = [kernel, source, python, addon, helper].map(p => path.resolve(p)), output = path.resolve(out);
  if (new Set([...inputs, output]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== kernelHash || hashes[1] !== sourceHash) throw new Error('input hash mismatch');
  const previous = JSON.parse(fs.readFileSync(inputs[1], 'utf8'));
  if (previous.schema !== 'horizons-planet-holdout/1' || previous.kernelSha256 !== kernelHash || !Array.isArray(previous.responses) || previous.responses.length !== 10 || !Array.isArray(previous.rows) || previous.rows.length !== 300) throw new Error('invalid Horizons report');
  const grid = epochGrid(), epochs = grid.map(r => r.jdTt);
  const erfa = JSON.parse(execFileSync(inputs[2], [helper], { input: JSON.stringify({ epochs }), encoding: 'utf8', timeout: 30000, maxBuffer: 1048576 }));
  const refHashes = ['erfaInitSha256', 'erfaCoreSha256', 'erfaBinarySha256', 'numpyInitSha256', 'numpyBinarySha256'];
  if (erfa.pyerfaVersion !== '2.0.1.5' || !refHashes.every(k => /^[a-f0-9]{64}$/.test(erfa[k] || '')) || !Array.isArray(erfa.rows) || erfa.rows.length !== epochs.length || !erfa.rows.every((r, i) => Array.isArray(r) && r.length === 2 && r[0] === epochs[i] && Number.isFinite(r[1]) && Math.abs(r[1]) < 1)) throw new Error('invalid ERFA response');
  const engine = new Engine(inputs[0]), rows = [];
  for (const [id, command, name] of bodies) {
    const matches = previous.responses.filter(r => r.id === command && r.name === name && r.url === urlFor(command, epochs));
    if (matches.length !== 1 || typeof matches[0].raw !== 'string' || !matches[0].raw.includes('Atmos refraction: NO (AIRLESS)')) throw new Error('invalid raw Horizons response');
    const reference = parseIcrfTable(matches[0].raw, name, command, epochs);
    for (let i = 0; i < epochs.length; i++) {
      const ref = reference[i];
      if (ref.raDeg < 0 || ref.raDeg >= 360 || Math.abs(ref.decDeg) > 90) throw new Error('invalid reference direction');
      const oldTdb = ttToTdb(epochs[i]), erfaTdb = epochs[i] + erfa.rows[i][1] / 86400;
      const old = direction(engine.experimentalApparentIcrfTdb(oldTdb, id).positionKm), alternative = direction(engine.experimentalApparentIcrfTdb(erfaTdb, id).positionKm), target = unit(ref.raDeg, ref.decDeg);
      const original = previous.rows.find(r => r.id === id && r.jdTt === epochs[i]);
      const oldResidualArcsec = separation(old, target);
      if (!original || original.jdTdb !== oldTdb || !Number.isFinite(original.icrfArcsec) || Math.abs(original.icrfArcsec - oldResidualArcsec) > 1e-9) throw new Error('original comparison mismatch');
      const simpleSeconds = 0.001657 * Math.sin((357.5291092 + 35999.0502909 * ((epochs[i] - 2451545) / 36525)) * Math.PI / 180);
      rows.push({ ...grid[i], id, body: name, simpleOffsetSeconds: simpleSeconds, erfaOffsetSeconds: erfa.rows[i][1], offsetDifferenceMs: (erfa.rows[i][1] - simpleSeconds) * 1000, roundedDifferenceMs: (erfaTdb - oldTdb) * 86400000,
        timeImpactArcsec: separation(old, alternative), oldResidualArcsec, erfaResidualArcsec: separation(alternative, target) });
    }
  }
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'horizons-tt-tdb-sensitivity/1', status: 'EXPLORATORY', kernelSha256: hashes[0], sourceSha256: hashes[1], pythonSha256: hashes[2], addonSha256: hashes[3], helperSha256: hashes[4],
    reference: { pyerfaVersion: erfa.pyerfaVersion, erfaVersion: erfa.erfaVersion, sofaVersion: erfa.sofaVersion, numpyVersion: erfa.numpyVersion, pythonVersion: erfa.pythonVersion, ...Object.fromEntries(refHashes.map(k => [k, erfa[k]])) },
    notes: 'Same geocentric apparent ICRF directions and DE441, with two TT-to-TDB models. ERFA is extrapolated in ancient epochs and not verified as Horizons own time model. Single-f64 JD rounding matters. Sensitivity, not historical physical error or accuracy bound.', summary: summarize(rows), rows };
  fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(report.summary));
}
module.exports = { summarize };
if (require.main === module) { try { main(); } catch (e) { console.error(e.message); process.exitCode = 1; } }
