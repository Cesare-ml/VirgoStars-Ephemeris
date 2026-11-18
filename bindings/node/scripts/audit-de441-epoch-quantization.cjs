'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// One adjacent floating-point epoch is a sensitivity sample, not an error bound.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');
const { Engine } = require('../index.js');
const { validateSource } = require('./audit-nutation-direction-impact.cjs');
const { direction } = require('./validate-horizons-planet-frame-grid.cjs');
const { unit, separation } = require('./compare-horizons-frame-isolation.cjs');

function nextUp(value) {
  if (Number.isNaN(value) || value === Infinity) return value;
  if (value === 0) return Number.MIN_VALUE;
  const view = new DataView(new ArrayBuffer(8));
  view.setFloat64(0, value);
  const bits = view.getBigUint64(0);
  view.setBigUint64(0, value > 0 ? bits + 1n : bits - 1n);
  return view.getFloat64(0);
}
function nextDown(value) { return -nextUp(-value); }
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function summarize(rows) {
  if (!Array.isArray(rows) || !rows.length || !rows.every(r => Number.isInteger(r.year) && Number.isInteger(r.id) && r.id >= 0 && r.id < 10 && [r.errorMicroseconds, r.stepMicroseconds, r.icrfStepArcsec, r.eclipticStepArcsec].every(Number.isFinite) && r.stepMicroseconds > 0 && r.icrfStepArcsec >= 0 && r.eclipticStepArcsec >= 0)) throw new Error('invalid rows');
  const max = (list, key, absolute = false) => {
    if (!list.length) throw new Error('empty group');
    const r = list.reduce((a, b) => (absolute ? Math.abs(b[key]) : b[key]) > (absolute ? Math.abs(a[key]) : a[key]) ? b : a);
    return { value: r[key], year: r.year, month: r.month, id: r.id };
  };
  const group = list => ({ cases: list.length, maxRoundingErrorMicroseconds: max(list, 'errorMicroseconds', true), maxAdjacentStepMicroseconds: max(list, 'stepMicroseconds'), maxIcrfStepArcsec: max(list, 'icrfStepArcsec'), maxEclipticStepArcsec: max(list, 'eclipticStepArcsec'), maxMoonIcrfStepArcsec: max(list.filter(r => r.id === 1), 'icrfStepArcsec') });
  return { all: group(rows), preHorizons: group(rows.filter(r => r.year < -9998)) };
}
function main() {
  const [kernel, kernelHash, source, sourceHash, python, projector, out, ...extra] = process.argv.slice(2);
  if (!kernel || !source || !python || !projector || !out || extra.length || ![kernelHash, sourceHash].every(x => /^[a-f0-9]{64}$/.test(x || '')) || fs.existsSync(out)) throw new Error('usage: KERNEL KERNEL_SHA SWISS_GRID GRID_SHA PYTHON PROJECTOR NEW_REPORT');
  const addon = path.resolve(__dirname, '../native/virgostars_ephemeris.node'), helper = path.resolve(__dirname, '../../../scripts/compare_erfa_tdb.py');
  const inputs = [kernel, source, python, projector, addon, helper].map(p => path.resolve(p)), output = path.resolve(out);
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
  const input = erfa.rows.map(([jd, seconds]) => `${jd},${seconds}`).join('\n') + '\n';
  const lines = execFileSync(inputs[3], [], { input, encoding: 'utf8', timeout: 30000, maxBuffer: 1048576 }).trim().split(/\r?\n/);
  if (lines.length !== epochs.length) throw new Error('wrong quantization row count');
  const measurements = new Map(lines.map((line, i) => {
    const f = line.split(',');
    if (f.length !== 3 || !f.every(x => x !== '' && Number.isFinite(Number(x)))) throw new Error('invalid quantization row');
    const [rounded, signed, magnitude] = f.map(Number), adjacent = signed > 0 ? nextDown(rounded) : nextUp(rounded);
    const step = Math.abs(adjacent - rounded) * 86400;
    if (Math.abs(Math.abs(signed) - magnitude) > 1e-15 || !Number.isFinite(step) || step <= 0 || magnitude > step / 2 + 1e-9 || rounded !== epochs[i] + erfa.rows[i][1] / 86400) throw new Error('quantization consistency failed');
    return [epochs[i], { rounded, adjacent, errorMicroseconds: signed * 1e6, stepMicroseconds: step * 1e6 }];
  }));
  const engine = new Engine(inputs[0]), rows = [];
  for (const r of sourceData.rows) {
    const t = measurements.get(r.jdTt), ref = unit(r.referenceRaDeg, r.referenceDecDeg);
    const original = direction(engine.experimentalApparentIcrfTdb(r.jdTdb, r.id).positionKm);
    if (!Number.isFinite(r.differenceArcsec) || Math.abs(separation(original, ref) - r.differenceArcsec) > 1e-9) throw new Error('original comparison mismatch');
    const a = direction(engine.experimentalApparentIcrfTdb(t.rounded, r.id).positionKm), b = direction(engine.experimentalApparentIcrfTdb(t.adjacent, r.id).positionKm);
    const c = engine.experimentalLongTermApparentTdb(t.rounded, r.id).xx, d = engine.experimentalLongTermApparentTdb(t.adjacent, r.id).xx;
    if (![c, d].every(v => v.length === 6 && v.every(Number.isFinite))) throw new Error('invalid ecliptic result');
    rows.push({ year: r.year, month: r.month, id: r.id, jdTt: r.jdTt, roundedTdb: t.rounded, adjacentTdb: t.adjacent, errorMicroseconds: t.errorMicroseconds, stepMicroseconds: t.stepMicroseconds, icrfStepArcsec: separation(a, b), eclipticStepArcsec: separation(unit(c[0], c[1]), unit(d[0], d[1])) });
  }
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'de441-epoch-quantization/1', status: 'EXPLORATORY', grid: sourceData.grid, kernelSha256: hashes[0], sourceSha256: hashes[1], pythonSha256: hashes[2], projectorSha256: hashes[3], addonSha256: hashes[4], helperSha256: hashes[5],
    reference: { pyerfaVersion: erfa.pyerfaVersion, erfaVersion: erfa.erfaVersion, sofaVersion: erfa.sofaVersion, numpyVersion: erfa.numpyVersion, pythonVersion: erfa.pythonVersion, ...Object.fromEntries(refHashes.map(k => [k, erfa[k]])) },
    notes: 'ERFA geocentric extrapolated TT-to-TDB model; actual split-to-f64 error, plus local apparent direction change to neighboring representable epoch. The latter is a sensitivity sample, not an estimate of actual astronomical error or physical ancient accuracy.', summary: summarize(rows), rows };
  fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(report.summary));
}
module.exports = { nextUp, nextDown, summarize };
if (require.main === module) { try { main(); } catch (e) { console.error(e.message); process.exitCode = 1; } }
