'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Finite component comparison only: this does not assess radial speed or release.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { julday } = require('../index.js');
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function assess(report, kernelHash, limit) {
  if (!/^[a-f0-9]{64}$/.test(kernelHash || '') || !Number.isFinite(limit) || limit <= 0 || report?.schema !== 'experimental-latitude-moon-rates/1' || report.status !== 'EXPERIMENTAL_NOT_ACCEPTED' || !Array.isArray(report.inputSha256) || report.inputSha256[0] !== kernelHash || !Array.isArray(report.rows) || report.rows.length !== 320) throw new Error('invalid angular-speed report or threshold');
  const years = [-12500, -10000, -5000, 0, 1000, 2000, 3000, 3900], months = [1, 4, 7, 10], seen = new Set(), checks = [];
  for (const r of report.rows) {
    if (!years.includes(r.year) || !months.includes(r.month) || !Number.isInteger(r.body) || r.body < 0 || r.body > 9 || r.jdTdb !== julday(r.year, r.month, 15, 12, true) || !Number.isFinite(r.latitudeCandidate) || !Number.isFinite(r.longitudeCandidate)) throw new Error('invalid angular-speed row');
    const key = `${r.year}/${r.month}/${r.body}`;
    if (seen.has(key)) throw new Error('duplicate angular-speed row');
    seen.add(key);
    checks.push({ component: 'latitude', body: r.body, jdTdb: r.jdTdb, residual: r.latitudeCandidate });
    if (r.body !== 1) checks.push({ component: 'longitude', body: r.body, jdTdb: r.jdTdb, residual: r.longitudeCandidate });
  }
  if (seen.size !== 320 || checks.length !== 608) throw new Error('incomplete angular-speed matrix');
  const worst = checks.reduce((a, b) => Math.abs(b.residual) > Math.abs(a.residual) ? b : a);
  return { bodyEpochCases: seen.size, componentChecks: checks.length, limitArcsecPerTdbDay: limit, maxArcsecPerTdbDay: Math.abs(worst.residual), worst: { component: worst.component, body: worst.body, jdTdb: worst.jdTdb }, failures: checks.filter(r => Math.abs(r.residual) > limit).length };
}
function main() {
  const [source, sourceHash, kernelHash, limitArg, out, ...extra] = process.argv.slice(2);
  if (!source || !/^[a-f0-9]{64}$/.test(sourceHash || '') || !/^[a-f0-9]{64}$/.test(kernelHash || '') || !limitArg || !out || extra.length || fs.existsSync(out)) throw new Error('usage: REPORT REPORT_SHA KERNEL_SHA LIMIT_ARCSEC_PER_TDB_DAY NEW_REPORT');
  const inputs = [source, __filename].map(x => path.resolve(x));
  if (new Set([...inputs, path.resolve(out)]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== sourceHash) throw new Error('source hash mismatch');
  const result = assess(JSON.parse(fs.readFileSync(inputs[0], 'utf8')), kernelHash, Number(limitArg));
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const output = { schema: 'mvp1-sampled-angular-speed-assessment/1', status: result.failures ? 'LIMIT_EXCEEDED' : 'PASS_SELECTED_ANGULAR_COMPONENTS_ONLY', sourceSha256: hashes[0], assessorSha256: hashes[1], kernelSha256: kernelHash, notes: 'Latitude for Sun, Moon, Mercury, Venus and Mars–Pluto system barycenters; longitude for the same except Moon. Lunar longitude has a separate criterion. Shared DE441; TDB vs approximate TT. No radial, unsampled, physical accuracy or release claim.', ...result };
  fs.writeFileSync(out, JSON.stringify(output, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(output));
  if (result.failures) process.exitCode = 2;
}
module.exports = { assess };
if (require.main === module) { try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; } }
