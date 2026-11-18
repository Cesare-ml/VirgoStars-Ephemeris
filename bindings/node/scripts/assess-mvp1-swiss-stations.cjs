'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// A finite TDB station comparison; it is not a physical or historical time bound.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function assess(reports, kernelHash, limitSeconds) {
  if (!/^[a-f0-9]{64}$/.test(kernelHash || '') || !Number.isFinite(limitSeconds) || limitSeconds <= 0 || !Array.isArray(reports) || reports.length !== 2) throw new Error('invalid input or threshold');
  const expectedYears = [[-12500, 0, 3900], [-11000, 1500, 3500]], all = [];
  reports.forEach((report, index) => {
    if (report?.schema !== 'experimental-longitude-stations/1' || report.status !== 'EXPERIMENTAL_NOT_ACCEPTED' || report.kernelSha256 !== kernelHash || !Array.isArray(report.rows) || report.rows.length !== 48) throw new Error('invalid station report');
    const seen = new Set();
    for (const row of report.rows) {
      if (!expectedYears[index].includes(row.year) || !Number.isInteger(row.body) || row.body < 2 || row.body > 9 || !['retrograde', 'direct'].includes(row.kind) || !Number.isFinite(row.jdTdb) || !Number.isFinite(row.candidateDeltaSeconds) || typeof row.candidateMatch !== 'boolean' || !Array.isArray(row.samples) || row.samples.length !== 2) throw new Error('invalid station record');
      const key = `${row.year}/${row.body}/${row.kind}`;
      if (seen.has(key)) throw new Error('duplicate station');
      seen.add(key);
      for (let i = 0; i < 2; i++) {
        const sample = row.samples[i], target = i === 0 ? -120 : 120;
        if (![sample?.jdTdb, sample?.candidate, sample?.published].every(Number.isFinite) || Math.abs((sample.jdTdb - row.jdTdb) * 86400 - target) > 0.001) throw new Error('invalid station samples');
      }
      const match = row.samples.every(s => Math.sign(s.candidate) !== 0 && Math.sign(s.candidate) === Math.sign(s.published));
      if (match !== row.candidateMatch) throw new Error('inconsistent station sign');
      all.push(row);
    }
    if (seen.size !== 48) throw new Error('incomplete station matrix');
  });
  const worst = all.reduce((a, b) => Math.abs(b.candidateDeltaSeconds) > Math.abs(a.candidateDeltaSeconds) ? b : a);
  return { cases: all.length, limitSeconds, maxDifferenceSeconds: Math.abs(worst.candidateDeltaSeconds), worst: { year: worst.year, body: worst.body, kind: worst.kind }, timeFailures: all.filter(r => Math.abs(r.candidateDeltaSeconds) > limitSeconds).length, signFailures: all.filter(r => !r.candidateMatch).length };
}
function main() {
  const [first, firstHash, second, secondHash, kernelHash, limitArg, out, ...extra] = process.argv.slice(2);
  if (!first || !second || !out || !limitArg || extra.length || ![firstHash, secondHash, kernelHash].every(x => /^[a-f0-9]{64}$/.test(x || '')) || fs.existsSync(out)) throw new Error('usage: FIRST FIRST_SHA SECOND SECOND_SHA KERNEL_SHA LIMIT_SECONDS NEW_REPORT');
  const inputs = [first, second, __filename].map(x => path.resolve(x));
  if (new Set([...inputs, path.resolve(out)]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== firstHash || hashes[1] !== secondHash) throw new Error('input hash mismatch');
  const result = assess(inputs.slice(0, 2).map(p => JSON.parse(fs.readFileSync(p, 'utf8'))), kernelHash, Number(limitArg));
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const output = { schema: 'mvp1-sampled-swiss-station-assessment/1', status: result.timeFailures || result.signFailures ? 'LIMIT_EXCEEDED' : 'PASS_96_SAMPLED_STATIONS_ONLY', sourceSha256: hashes.slice(0, 2), assessorSha256: hashes[2], kernelSha256: kernelHash, notes: 'TDB numerical root centers with approximate TT reference, shared DE441. Four initial Pluto cases informed the model. No unsampled, UT1 or physical historical-time accuracy implied.', ...result };
  fs.writeFileSync(out, JSON.stringify(output, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(output));
  if (result.timeFailures || result.signFailures) process.exitCode = 2;
}
module.exports = { assess };
if (require.main === module) { try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; } }
