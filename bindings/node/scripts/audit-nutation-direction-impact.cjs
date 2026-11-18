'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// The same reference direction is projected twice; this is not orbital validation.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');
const { julday } = require('../index.js');
const { yearsFor, tdbToTt } = require('./validate-swiss-fixed-icrs-grid.cjs');
const { unit, separation } = require('./compare-horizons-frame-isolation.cjs');

function digest(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function validateSource(source) {
  if (!source || source.schema !== 'swiss-fixed-icrs-grid/1' || !['baseline', 'interleaved'].includes(source.grid) || !Number.isInteger(source.flags) || !Array.isArray(source.rows)) throw new Error('invalid source report');
  const expected = yearsFor(source.grid).flatMap(year => [1, 4, 7, 10].flatMap(month => Array.from({ length: 10 }, (_, id) => ({ year, month, id }))));
  if (source.rows.length !== expected.length) throw new Error('wrong source row count');
  for (let i = 0; i < expected.length; i++) {
    const r = source.rows[i], e = expected[i], tdb = julday(e.year, e.month, 1, 12, true);
    if (r.year !== e.year || r.month !== e.month || r.id !== e.id || r.jdTdb !== tdb || r.jdTt !== tdbToTt(tdb) || !Number.isInteger(r.rflag) || (r.rflag & source.flags) !== source.flags || !Number.isFinite(r.referenceRaDeg) || r.referenceRaDeg < 0 || r.referenceRaDeg >= 360 || !Number.isFinite(r.referenceDecDeg) || Math.abs(r.referenceDecDeg) > 90) throw new Error(`invalid source row ${i}`);
  }
}

function checkedVectors(fields, reference) {
  if (!Array.isArray(fields) || fields.length !== 12 || !fields.every(Number.isFinite)) throw new Error('invalid projection');
  const vectors = Array.from({ length: 4 }, (_, i) => fields.slice(i * 3, i * 3 + 3));
  if (!vectors.every(v => Math.abs(Math.hypot(...v) - 1) < 1e-12)) throw new Error('non-unit projection');
  if (!Array.isArray(reference) || reference.length !== 10 || !reference.every(Number.isFinite) || separation(vectors[0], unit(reference[2], reference[3])) > 1e-7 || separation(vectors[2], unit(reference[4], reference[5])) > 1e-7) throw new Error('full-argument projection differs from existing diagnostic');
  return { equatorialArcsec: separation(vectors[0], vectors[1]), eclipticArcsec: separation(vectors[2], vectors[3]) };
}

function summarize(rows) {
  if (!Array.isArray(rows) || !rows.length || !rows.every(r => Number.isInteger(r.id) && r.id >= 0 && r.id < 10 && Number.isInteger(r.year) && [r.equatorialArcsec, r.eclipticArcsec].every(x => Number.isFinite(x) && x >= 0))) throw new Error('invalid results');
  const max = (list, key) => {
    if (!list.length) throw new Error('empty group');
    const row = list.reduce((a, b) => b[key] > a[key] ? b : a);
    return { arcsec: row[key], year: row.year, month: row.month, id: row.id };
  };
  const group = list => ({ cases: list.length, equatorial: max(list, 'equatorialArcsec'), ecliptic: max(list, 'eclipticArcsec') });
  return { all: group(rows), preHorizons: group(rows.filter(r => r.year < -9998)), perBody: Array.from({ length: 10 }, (_, id) => ({ id, ...group(rows.filter(r => r.id === id)) })) };
}

function main() {
  const [sourceFile, sourceSha, projector, referenceProjector, out, ...extra] = process.argv.slice(2);
  if (!sourceFile || !/^[a-f0-9]{64}$/.test(sourceSha || '') || !projector || !referenceProjector || !out || extra.length || fs.existsSync(out)) throw new Error('usage: FIXED_GRID_REPORT REPORT_SHA PROJECTOR EXISTING_PROJECTOR NEW_REPORT');
  const inputs = [path.resolve(sourceFile), path.resolve(projector), path.resolve(referenceProjector)], output = path.resolve(out);
  if (new Set([...inputs, output]).size !== 4) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== sourceSha) throw new Error('source hash mismatch');
  const source = JSON.parse(fs.readFileSync(inputs[0], 'utf8'));
  validateSource(source);
  const input = source.rows.map(r => `${r.jdTt},${r.referenceRaDeg},${r.referenceDecDeg}`).join('\n') + '\n';
  const lines = execFileSync(inputs[1], [], { input, encoding: 'utf8', maxBuffer: 4 * 1048576, timeout: 30000 }).trim().split(/\r?\n/);
  const referenceLines = execFileSync(inputs[2], [], { input, encoding: 'utf8', maxBuffer: 4 * 1048576, timeout: 30000 }).trim().split(/\r?\n/);
  if (lines.length !== source.rows.length || referenceLines.length !== source.rows.length) throw new Error('projection count mismatch');
  const rows = source.rows.map((r, i) => {
    const fields = lines[i].split(',');
    if (fields.length !== 12 || fields.some(x => x.trim() === '')) throw new Error('invalid projection fields');
    const ref = referenceLines[i].split(',');
    if (ref.length !== 10 || ref.some(x => x.trim() === '')) throw new Error('invalid reference projection fields');
    return { year: r.year, month: r.month, id: r.id, jdTt: r.jdTt, ...checkedVectors(fields.map(Number), ref.map(Number)) };
  });
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'nutation-direction-impact/1', status: 'EXPLORATORY', grid: source.grid, sourceSha256: hashes[0], projectorSha256: hashes[1], referenceProjectorSha256: hashes[2], sourceKernelSha256: source.kernelSha256, sourceSwissVersion: source.swissVersion, notes: 'Same Swiss apparent fixed-axis direction projected using full versus linear 77-term arguments. TT. No change to the direction or to the remaining orientation model. Model sensitivity, not a physical accuracy bound.', summary: summarize(rows), rows };
  fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(report.summary));
}

module.exports = { validateSource, checkedVectors, summarize };
if (require.main === module) { try { main(); } catch (e) { console.error(e.message); process.exitCode = 1; } }
