'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Offline assessment of existing evidence. A threshold is supplied by the caller;
// passing this one check does not approve a release or establish physical accuracy.
const fs = require('node:fs');
const crypto = require('node:crypto');
const path = require('node:path');
const { julday } = require('../index.js');
const { yearsFor, tdbToTt } = require('./validate-swiss-fixed-icrs-grid.cjs');
const { checkConsistency } = require('./validate-horizons-planet-frame-grid.cjs');

function digest(file) { return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'); }
function validateTolerance(value) {
  if (typeof value !== 'number' || !Number.isFinite(value) || value <= 0 || value > 180 * 3600) throw new Error('invalid tolerance in arcseconds');
  return value;
}
function summarizeRows(rows, tolerance) {
  validateTolerance(tolerance);
  if (!Array.isArray(rows) || !rows.length || !rows.every(r => Number.isInteger(r.id) && r.id >= 0 && r.id < 10 && Number.isInteger(r.year) && Number.isInteger(r.month) && Number.isFinite(r.directActualArcsec) && r.directActualArcsec >= 0)) throw new Error('invalid evidence rows');
  const group = list => {
    if (!list.length) throw new Error('empty evidence group');
    const worst = list.reduce((a, b) => b.directActualArcsec > a.directActualArcsec ? b : a);
    const failures = list.filter(r => r.directActualArcsec > tolerance).length;
    return { cases: list.length, failures, maximumArcsec: worst.directActualArcsec, year: worst.year, month: worst.month, id: worst.id, meetsTolerance: failures === 0 };
  };
  return { all: group(rows), beforeHorizons: group(rows.filter(r => r.year < -9998)), byBody: Array.from({ length: 10 }, (_, id) => ({ id, ...group(rows.filter(r => r.id === id)) })) };
}
function validateReport(report, grid) {
  if (!report || report.schema !== 'swiss-ancient-frame-decomposition/1' || report.grid !== grid || !Array.isArray(report.rows)) throw new Error('invalid reference report');
  for (const key of ['kernelSha256', 'sourceSha256', 'projectorSha256', 'swissPackageSha256', 'swissWrapperSha256', 'swissNativeSha256', 'addonSha256']) if (!/^[a-f0-9]{64}$/.test(report[key] || '')) throw new Error('missing provenance hash');
  if (typeof report.swissVersion !== 'string' || !report.swissVersion) throw new Error('missing Swiss version');
  const expected = yearsFor(grid).flatMap(year => [1, 4, 7, 10].flatMap(month => Array.from({ length: 10 }, (_, id) => ({ year, month, id }))));
  if (report.rows.length !== expected.length) throw new Error('wrong row count');
  for (let i = 0; i < expected.length; i++) {
    const r = report.rows[i], e = expected[i], jd = julday(e.year, e.month, 1, 12, true);
    if (!r || r.year !== e.year || r.month !== e.month || r.id !== e.id || r.jdTdb !== jd || r.jdTt !== tdbToTt(jd) || r.swissFixedFlags !== 133217 || r.swissEclipticFlags !== 1 || r.swissEquatorialFlags !== 2049 || !Array.isArray(r.swissEclipticDeg) || r.swissEclipticDeg.length !== 2 || !r.swissEclipticDeg.every(Number.isFinite) || r.swissEclipticDeg[0] < 0 || r.swissEclipticDeg[0] >= 360 || Math.abs(r.swissEclipticDeg[1]) > 90) throw new Error(`invalid reference row ${i}`);
    checkConsistency({ icrfArcsec: r.fixedArcsec, poleArcsec: r.poleActualArcsec, directArcsec: r.directActualArcsec, samePoleArcsec: r.samePoleArcsec, sameDirectArcsec: r.sameDirectArcsec, poleFrameArcsec: r.poleFrameArcsec, directFrameArcsec: r.directFrameArcsec });
  }
}
function main() {
  const [baseline, baselineSha, interleaved, interleavedSha, limitText, out, ...extra] = process.argv.slice(2);
  if (!baseline || !interleaved || !out || extra.length || ![baselineSha, interleavedSha].every(x => /^[a-f0-9]{64}$/.test(x || '')) || !limitText || !limitText.trim() || fs.existsSync(out)) throw new Error('usage: BASELINE BASELINE_SHA INTERLEAVED INTERLEAVED_SHA TOLERANCE_ARCSEC NEW_REPORT');
  const tolerance = validateTolerance(Number(limitText)), inputs = [path.resolve(baseline), path.resolve(interleaved)], output = path.resolve(out);
  if (new Set([...inputs, output]).size !== 3) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== baselineSha || hashes[1] !== interleavedSha) throw new Error('input hash mismatch');
  const reports = inputs.map(file => JSON.parse(fs.readFileSync(file, 'utf8')));
  validateReport(reports[0], 'baseline'); validateReport(reports[1], 'interleaved');
  for (const key of ['kernelSha256', 'swissPackageSha256', 'swissWrapperSha256', 'swissNativeSha256', 'addonSha256', 'swissVersion']) if (reports[0][key] !== reports[1][key]) throw new Error(`reference mismatch: ${key}`);
  if (reports[0].sourceSha256 === reports[1].sourceSha256) throw new Error('source grids are not distinct');
  const rows = reports.flatMap(r => r.rows), summary = summarizeRows(rows, tolerance);
  if (inputs.some((file, i) => digest(file) !== hashes[i])) throw new Error('input changed');
  const result = { schema: 'mvp1-swiss-position-evidence/1', status: 'CONDITIONAL_EVIDENCE_ONLY', criterion: 'experimental direct-obliquity geocentric apparent ecliptic great-circle separation against Swiss JPL/DE441', toleranceArcsec: tolerance, baselineSha256: hashes[0], interleavedSha256: hashes[1], kernelSha256: reports[0].kernelSha256, swissVersion: reports[0].swissVersion, summary,
    limitations: 'Caller-supplied tolerance is not product approval. Same DE441, sampled dates and mixed ecliptic convention; no physical or continuous accuracy claim. Rates, stations, historical UT1 and Desktop integration are separate gates.' };
  fs.writeFileSync(output, JSON.stringify(result, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(result.summary.all));
  if (!summary.all.meetsTolerance) process.exitCode = 2;
}
module.exports = { validateTolerance, summarizeRows, validateReport };
if (require.main === module) { try { main(); } catch (e) { console.error(e.message); process.exitCode = 1; } }
