'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Shared DE441 and extrapolated models: sampled sensitivity, not an accuracy bound.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');
const { Engine } = require('../index.js');
const { validateSource } = require('./audit-nutation-direction-impact.cjs');
const { ttToTdb, direction } = require('./validate-horizons-planet-frame-grid.cjs');
const { unit, separation } = require('./compare-horizons-frame-isolation.cjs');

function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function summarize(rows) {
  const fields = ['offsetDifferenceMs', 'roundedDifferenceMs', 'roundTripDifferenceMs', 'icrfImpactArcsec', 'eclipticImpactArcsec', 'simpleResidualArcsec', 'erfaResidualArcsec'];
  if (!Array.isArray(rows) || !rows.length || !rows.every(r => Number.isInteger(r.id) && r.id >= 0 && r.id < 10 && Number.isInteger(r.year) && fields.every(k => Number.isFinite(r[k])) && fields.slice(3).every(k => r[k] >= 0))) throw new Error('invalid rows');
  const max = (list, key, abs = false) => {
    if (!list.length) throw new Error('empty group');
    const r = list.reduce((a, b) => (abs ? Math.abs(b[key]) : b[key]) > (abs ? Math.abs(a[key]) : a[key]) ? b : a);
    return { value: r[key], year: r.year, month: r.month, id: r.id };
  };
  const group = list => ({ cases: list.length, offsetDifferenceMs: max(list, 'offsetDifferenceMs', true), roundedDifferenceMs: max(list, 'roundedDifferenceMs', true), roundTripDifferenceMs: max(list, 'roundTripDifferenceMs', true), icrfImpactArcsec: max(list, 'icrfImpactArcsec'), eclipticImpactArcsec: max(list, 'eclipticImpactArcsec'), moonIcrfImpactArcsec: max(list.filter(r => r.id === 1), 'icrfImpactArcsec'), moonSimpleResidualArcsec: max(list.filter(r => r.id === 1), 'simpleResidualArcsec'), moonErfaResidualArcsec: max(list.filter(r => r.id === 1), 'erfaResidualArcsec') });
  return { all: group(rows), preHorizons: group(rows.filter(r => r.year < -9998)) };
}
function main() {
  const [kernel, kernelHash, source, sourceHash, python, out, ...extra] = process.argv.slice(2);
  if (!kernel || !source || !python || !out || extra.length || ![kernelHash, sourceHash].every(x => /^[a-f0-9]{64}$/.test(x || '')) || fs.existsSync(out)) throw new Error('usage: KERNEL KERNEL_SHA SWISS_GRID REPORT_SHA PYTHON NEW_REPORT');
  const addon = path.resolve(__dirname, '../native/virgostars_ephemeris.node'), helper = path.resolve(__dirname, '../../../scripts/compare_erfa_tdb.py');
  const inputs = [kernel, source, python, addon, helper].map(p => path.resolve(p)), output = path.resolve(out);
  if (new Set([...inputs, output]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== kernelHash || hashes[1] !== sourceHash) throw new Error('input hash mismatch');
  const sourceData = JSON.parse(fs.readFileSync(inputs[1], 'utf8'));
  validateSource(sourceData);
  const forbidden = 16 | 8 | 512 | 1024 | 32768 | 65536 | 16384 | 4096 | 8192;
  if (sourceData.kernelSha256 !== kernelHash || sourceData.flags !== 133153 || !sourceData.rows.every(r => (r.rflag & forbidden) === 0)) throw new Error('invalid source provenance or flags');
  const epochs = [...new Set(sourceData.rows.map(r => r.jdTt))];
  if (epochs.length !== sourceData.rows.length / 10) throw new Error('unexpected epoch count');
  const erfa = JSON.parse(execFileSync(inputs[2], [helper], { input: JSON.stringify({ epochs }), encoding: 'utf8', timeout: 30000, maxBuffer: 1048576 }));
  const refHashes = ['erfaInitSha256', 'erfaCoreSha256', 'erfaBinarySha256', 'numpyInitSha256', 'numpyBinarySha256'];
  if (erfa.pyerfaVersion !== '2.0.1.5' || !refHashes.every(k => /^[a-f0-9]{64}$/.test(erfa[k] || '')) || !Array.isArray(erfa.rows) || erfa.rows.length !== epochs.length || !erfa.rows.every((r, i) => Array.isArray(r) && r.length === 2 && r[0] === epochs[i] && Number.isFinite(r[1]) && Math.abs(r[1]) < 1)) throw new Error('invalid ERFA response');
  const byEpoch = new Map(erfa.rows), engine = new Engine(inputs[0]), rows = [];
  for (const r of sourceData.rows) {
    const tt = r.jdTt, simpleTdb = ttToTdb(tt), erfaSeconds = byEpoch.get(tt), erfaTdb = tt + erfaSeconds / 86400;
    const original = direction(engine.experimentalApparentIcrfTdb(r.jdTdb, r.id).positionKm);
    const simple = direction(engine.experimentalApparentIcrfTdb(simpleTdb, r.id).positionKm);
    const alternative = direction(engine.experimentalApparentIcrfTdb(erfaTdb, r.id).positionKm);
    const ref = unit(r.referenceRaDeg, r.referenceDecDeg);
    if (!Number.isFinite(r.differenceArcsec) || Math.abs(separation(original, ref) - r.differenceArcsec) > 1e-9) throw new Error('original comparison mismatch');
    const a = engine.experimentalLongTermApparentTdb(simpleTdb, r.id).xx, b = engine.experimentalLongTermApparentTdb(erfaTdb, r.id).xx;
    if (![a, b].every(v => v.length === 6 && v.every(Number.isFinite))) throw new Error('invalid ecliptic result');
    const simpleSeconds = 0.001657 * Math.sin((357.5291092 + 35999.0502909 * ((tt - 2451545) / 36525)) * Math.PI / 180);
    rows.push({ year: r.year, month: r.month, id: r.id, jdTt: tt, originalTdb: r.jdTdb, simpleTdb, erfaTdb,
      offsetDifferenceMs: (erfaSeconds - simpleSeconds) * 1000, roundedDifferenceMs: (erfaTdb - simpleTdb) * 86400000, roundTripDifferenceMs: (simpleTdb - r.jdTdb) * 86400000,
      icrfImpactArcsec: separation(simple, alternative), eclipticImpactArcsec: separation(unit(a[0], a[1]), unit(b[0], b[1])), simpleResidualArcsec: separation(simple, ref), erfaResidualArcsec: separation(alternative, ref) });
  }
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'swiss-ancient-tt-tdb-sensitivity/1', status: 'EXPLORATORY', grid: sourceData.grid, kernelSha256: hashes[0], sourceSha256: hashes[1], pythonSha256: hashes[2], addonSha256: hashes[3], helperSha256: hashes[4], swissVersion: sourceData.swissVersion,
    reference: { pyerfaVersion: erfa.pyerfaVersion, erfaVersion: erfa.erfaVersion, sofaVersion: erfa.sofaVersion, numpyVersion: erfa.numpyVersion, pythonVersion: erfa.pythonVersion, ...Object.fromEntries(refHashes.map(k => [k, erfa[k]])) },
    notes: 'Same TT instants, shared DE441 and stored Swiss fixed-axis directions. ICRF and local pole-based ecliptic; time models are extrapolated, not known Swiss time conversion or physical historical accuracy. Single-f64 JD rounding included.', summary: summarize(rows), rows };
  fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(report.summary));
}
module.exports = { summarize };
if (require.main === module) { try { main(); } catch (e) { console.error(e.message); process.exitCode = 1; } }
