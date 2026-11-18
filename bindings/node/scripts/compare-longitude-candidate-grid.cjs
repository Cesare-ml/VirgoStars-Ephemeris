'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Offline Swiss comparison on dates distinct from the station selection.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { Engine, julday } = require('../index.js');
const { checkedSwiss, tdbToTt } = require('./validate-mvp1-swiss-rates.cjs');
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function main() {
  const [kernel, hash, swissDir, out, ...extra] = process.argv.slice(2);
  if (!kernel || !/^[a-f0-9]{64}$/.test(hash || '') || !swissDir || !out || extra.length || fs.existsSync(out)) throw new Error('usage: KERNEL SHA SWISSEPH NEW_REPORT');
  const pkg = path.resolve(swissDir), inputs = [kernel, path.join(pkg, 'package.json'), path.join(pkg, 'lib/swisseph.js'), path.join(pkg, 'build/Release/swisseph.node'), path.resolve(__dirname, '../native/virgostars_ephemeris.node'), __filename].map(x => path.resolve(x));
  if (new Set([...inputs, path.resolve(out)]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== hash) throw new Error('kernel hash mismatch');
  const swiss = require(pkg), engine = new Engine(inputs[0]);
  swiss.swe_set_ephe_path(path.dirname(inputs[0])); swiss.swe_set_jpl_file(path.basename(inputs[0]));
  const rows = [];
  for (const year of [-12000, -9000, -7000, -2500, 500, 1600, 2600, 3500]) for (const month of [1, 4, 7, 10]) for (let body = 0; body < 10; body++) {
    const jd = julday(year, month, 15, 12, true);
    const reference = checkedSwiss(swiss.swe_calc(tdbToTt(jd), body, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED), body, jd)[3];
    const candidate = engine.experimentalLongitudeRateCandidateTdb(jd, body);
    if (!candidate.base.xx.every(Number.isFinite) || !Number.isFinite(candidate.candidateDegPerTdbDay)) throw new Error('invalid candidate');
    rows.push({ year, month, body, jdTdb: jd, baseResidualArcsecDay: (candidate.base.xx[3] - reference) * 3600, candidateResidualArcsecDay: (candidate.candidateDegPerTdbDay - reference) * 3600 });
  }
  const summarize = list => ({ cases: list.length, improved: list.filter(r => Math.abs(r.candidateResidualArcsecDay) < Math.abs(r.baseResidualArcsecDay)).length, rmsBefore: Math.hypot(...list.map(r => r.baseResidualArcsecDay)) / Math.sqrt(list.length), rmsCandidate: Math.hypot(...list.map(r => r.candidateResidualArcsecDay)) / Math.sqrt(list.length), maxBefore: Math.max(...list.map(r => Math.abs(r.baseResidualArcsecDay))), maxCandidate: Math.max(...list.map(r => Math.abs(r.candidateResidualArcsecDay))) });
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'experimental-longitude-grid/1', status: 'EXPERIMENTAL_NOT_ACCEPTED', kernelSha256: hashes[0], swissPackageSha256: hashes[1], swissWrapperSha256: hashes[2], swissNativeSha256: hashes[3], addonSha256: hashes[4], scriptSha256: hashes[5], swissVersion: swiss.swe_version(), units: 'arcsec/TDB day', notes: 'Geocentric apparent mixed ecliptic, TDB versus approximate TT, shared DE441. Candidate is not a position derivative. No continuous or physical accuracy claim.', summary: summarize(rows), perBody: Object.fromEntries(Array.from({ length: 10 }, (_, body) => [body, summarize(rows.filter(r => r.body === body))])), rows };
  fs.writeFileSync(out, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ summary: report.summary, perBody: report.perBody }));
}
if (require.main === module) { try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; } }
