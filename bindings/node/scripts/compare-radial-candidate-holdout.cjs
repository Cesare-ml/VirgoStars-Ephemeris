'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Explicit offline Swiss comparison; no reference package is a runtime dependency.
const fs = require('node:fs');
const path = require('node:path');
const { Engine, julday } = require('../index.js');
const { checkedSwiss, tdbToTt } = require('./validate-mvp1-swiss-rates.cjs');
const crypto = require('node:crypto');
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function main() {
  const [kernel, kernelSha256, swissPath, output, grid, ...extra] = process.argv.slice(2);
  if (!kernel || !/^[a-f0-9]{64}$/.test(kernelSha256 || '') || !swissPath || !output || !['evaluation', 'confirmation'].includes(grid) || extra.length || fs.existsSync(output)) throw new Error('usage: KERNEL SHA256 SWISSEPH_PACKAGE NEW_REPORT evaluation|confirmation');
  const pkg = path.resolve(swissPath), inputs = [kernel, path.join(pkg, 'package.json'), path.join(pkg, 'lib/swisseph.js'), path.join(pkg, 'build/Release/swisseph.node'), path.resolve(__dirname, '../native/virgostars_ephemeris.node'), __filename].map(p => path.resolve(p));
  if (new Set([...inputs, path.resolve(output)]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== kernelSha256) throw new Error('kernel hash mismatch');
  const swiss = require(pkg), engine = new Engine(inputs[0]);
  swiss.swe_set_ephe_path(path.dirname(inputs[0])); swiss.swe_set_jpl_file(path.basename(inputs[0]));
  const rows = [];
  // The Sun exclusion was selected on evaluation; confirmation uses other dates.
  const years = grid === 'evaluation' ? [-12000, -9000, -7000, -2500, 500, 1500, 2500, 3500] : [-11000, -8000, -3000, -1000, 400, 1600, 2600, 3800];
  for (const year of years) for (const month of [1, 4, 7, 10]) for (let body = 0; body < 10; body++) {
    const jdTdb = julday(year, month, 15, 12, true);
    const b = checkedSwiss(swiss.swe_calc(tdbToTt(jdTdb), body, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED), body, jdTdb);
    const c = engine.experimentalRadialRateCandidateTdb(jdTdb, body);
    if (!c.base.xx.every(Number.isFinite) || !Number.isFinite(c.candidateAuPerTdbDay) || !Number.isFinite(c.correctionAuPerTdbDay)) throw new Error('invalid candidate');
    rows.push({ year, month, body, jdTdb, correctionApplied: c.correctionApplied, baselineResidualAuDay: c.base.xx[5] - b[5], candidateResidualAuDay: c.candidateAuPerTdbDay - b[5], correctionAuDay: c.correctionAuPerTdbDay });
  }
  const summary = items => ({ cases: items.length, improved: items.filter(r => Math.abs(r.candidateResidualAuDay) < Math.abs(r.baselineResidualAuDay)).length, rmsBeforeAuDay: Math.hypot(...items.map(r => r.baselineResidualAuDay)) / Math.sqrt(items.length), rmsCandidateAuDay: Math.hypot(...items.map(r => r.candidateResidualAuDay)) / Math.sqrt(items.length), maxBeforeAuDay: Math.max(...items.map(r => Math.abs(r.baselineResidualAuDay))), maxCandidateAuDay: Math.max(...items.map(r => Math.abs(r.candidateResidualAuDay))) });
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'experimental-radial-candidate-holdout/1', status: 'EXPERIMENTAL_NOT_ACCEPTED', grid, kernelSha256: hashes[0], swissPackageSha256: hashes[1], swissWrapperSha256: hashes[2], swissNativeSha256: hashes[3], addonSha256: hashes[4], scriptSha256: hashes[5], swissVersion: swiss.swe_version(), notes: 'Geocentric apparent mixed ecliptic, TDB vs approximate TT, shared DE441. Candidate is not the derivative and does not alter angular rates or stations. No tolerance or physical accuracy implied.', summary: summary(rows), perBody: Object.fromEntries(Array.from({ length: 10 }, (_, body) => [body, summary(rows.filter(r => r.body === body))])), rows };
  fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ summary: report.summary, perBody: report.perBody }));
}
if (require.main === module) { try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; } }
