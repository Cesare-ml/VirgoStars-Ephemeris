'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Checks only sampled lunar longitude speed; no release or physical accuracy claim.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { julday } = require('../index.js');
const { nextDown } = require('./audit-de441-epoch-quantization.cjs');
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function expectedEpochs() {
  const expected = [];
  for (const year of [-12999, -12500, -10000, -5000, 0, 2000, 3900, 3999]) for (let month = 1; month <= 12; month++) for (const day of [3, 21]) expected.push(julday(year, month, day, 12, true));
  expected.push(julday(-12999, 1, 1, 0, true), nextDown(julday(4000, 1, 1, 0, true)));
  return expected;
}
function assess(report, kernelHash, limit) {
  if (!/^[a-f0-9]{64}$/.test(kernelHash || '') || !Number.isFinite(limit) || limit <= 0 || report?.schema !== 'lunar-speed-resolution/1' || report.status !== 'OPEN_NO_ACCEPTED_RATE_TOLERANCE' || !Array.isArray(report.inputSha256) || report.inputSha256[0] !== kernelHash || !Array.isArray(report.rows) || report.rows.length !== 194 || !report.rows.every(r => Number.isFinite(r.jdTdb) && Number.isFinite(r.longitudeCandidateMinusSwiss))) throw new Error('invalid lunar report or threshold');
  const expected = expectedEpochs(), seen = new Set(report.rows.map(r => r.jdTdb));
  if (seen.size !== expected.length || !expected.every(jd => seen.has(jd))) throw new Error('incomplete or duplicate lunar epochs');
  const first = expected.at(-2), last = expected.at(-1);
  if (report.rows.filter(r => r.category === 'first' && r.jdTdb === first).length !== 1 || report.rows.filter(r => r.category === 'last' && r.jdTdb === last).length !== 1 || report.rows.filter(r => r.category === 'grid').length !== 192) throw new Error('invalid endpoint labels');
  const maximum = report.rows.reduce((a, b) => Math.abs(b.longitudeCandidateMinusSwiss) > Math.abs(a.longitudeCandidateMinusSwiss) ? b : a);
  const failures = report.rows.filter(r => Math.abs(r.longitudeCandidateMinusSwiss) > limit).length;
  return { cases: report.rows.length, limitArcsecPerTdbDay: limit, maxArcsecPerTdbDay: Math.abs(maximum.longitudeCandidateMinusSwiss), worstJdTdb: maximum.jdTdb, failures };
}
function main() {
  const [source, expectedHash, kernelHash, limitArg, out, ...extra] = process.argv.slice(2);
  if (!source || !/^[a-f0-9]{64}$/.test(expectedHash || '') || !/^[a-f0-9]{64}$/.test(kernelHash || '') || !limitArg || !out || extra.length || fs.existsSync(out)) throw new Error('usage: LUNAR_REPORT REPORT_SHA KERNEL_SHA LIMIT_ARCSEC_PER_TDB_DAY NEW_REPORT');
  const inputs = [path.resolve(source), path.resolve(__filename)];
  if (new Set([...inputs, path.resolve(out)]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== expectedHash) throw new Error('input hash mismatch');
  const result = assess(JSON.parse(fs.readFileSync(inputs[0], 'utf8')), kernelHash, Number(limitArg));
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const output = { schema: 'sampled-lunar-longitude-speed-assessment/1', status: result.failures ? 'LIMIT_EXCEEDED' : 'PASS_SAMPLED_LUNAR_LONGITUDE_ONLY', sourceSha256: hashes[0], assessorSha256: hashes[1], kernelSha256: kernelHash, reference: 'Swiss published geocentric apparent ecliptic longitude speed; shared DE441; approximate TT', notes: 'This checks only the specified samples, not latitude, radial velocity, stations, UT1, unsampled dates, physical accuracy, or MVP release.', ...result };
  fs.writeFileSync(out, JSON.stringify(output, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(output));
  if (result.failures) process.exitCode = 2;
}
module.exports = { assess, expectedEpochs };
if (require.main === module) { try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; } }
