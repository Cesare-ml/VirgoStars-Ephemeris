'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Offline numerical comparison, not independent orbital validation.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { Engine } = require('../index.js');
const { checkedSwiss, tdbToTt, matchingNonzeroSigns } = require('./validate-mvp1-swiss-rates.cjs');
const { bisect } = require('./compare-swiss-station-roots.cjs');
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function main() {
  const [kernel, kernelHash, swissDir, source, sourceHash, out, ...extra] = process.argv.slice(2);
  if (!kernel || !swissDir || !source || !out || extra.length || ![kernelHash, sourceHash].every(x => /^[a-f0-9]{64}$/.test(x || '')) || fs.existsSync(out)) throw new Error('usage: KERNEL SHA SWISSEPH STATION_REPORT SHA NEW_REPORT');
  const pkg = path.resolve(swissDir), inputs = [kernel, source, path.join(pkg, 'package.json'), path.join(pkg, 'lib/swisseph.js'), path.join(pkg, 'build/Release/swisseph.node'), path.resolve(__dirname, '../native/virgostars_ephemeris.node'), __filename].map(x => path.resolve(x));
  if (new Set([...inputs, path.resolve(out)]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== kernelHash || hashes[1] !== sourceHash) throw new Error('input hash mismatch');
  const old = JSON.parse(fs.readFileSync(inputs[1], 'utf8'));
  if (!['mvp1-swiss-rate-boundaries/1', 'longitude-candidate-station-selection/1'].includes(old.schema) || old.kernelSha256 !== kernelHash || !Array.isArray(old.stations) || old.stations.length !== 48 || !old.stations.every(s => s.status === 'ok' && Number.isFinite(s.jdTdb) && Number.isInteger(s.body) && Array.isArray(s.initial) && s.initial.length === 2 && s.initial.every(Number.isFinite))) throw new Error('invalid station source');
  const swiss = require(pkg), engine = new Engine(inputs[0]);
  swiss.swe_set_ephe_path(path.dirname(inputs[0])); swiss.swe_set_jpl_file(path.basename(inputs[0]));
  const reference = (jd, body) => checkedSwiss(swiss.swe_calc(tdbToTt(jd), body, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED), body, jd)[3];
  const local = (jd, body) => { const r = engine.experimentalLongitudeRateCandidateTdb(jd, body); if (!r.base.xx.every(Number.isFinite) || !Number.isFinite(r.candidateDegPerTdbDay)) throw new Error('invalid candidate'); return r; };
  const rows = old.stations.map(s => {
    const offsets = [-120 / 86400, 120 / 86400];
    const samples = offsets.map(offset => { const jd = s.jdTdb + offset, candidate = local(jd, s.body), published = reference(jd, s.body); return { jdTdb: jd, baseline: candidate.base.xx[3], candidate: candidate.candidateDegPerTdbDay, published }; });
    const sign = field => samples.map(x => Math.sign(x[field]));
    const publishedSigns = sign('published'), baseMatch = matchingNonzeroSigns(sign('baseline'), publishedSigns), candidateMatch = matchingNonzeroSigns(sign('candidate'), publishedSigns);
    const lo = s.initial[0] - 1, hi = s.initial[1] + 1;
    let baselineDelta = null, candidateDelta = null;
    try {
      const root = fn => { const b = bisect(fn, lo, hi, 0.00001); return (b.lo + b.hi) / 2; };
      const p = root(jd => reference(jd, s.body));
      baselineDelta = (root(jd => local(jd, s.body).base.xx[3]) - p) * 86400;
      candidateDelta = (root(jd => local(jd, s.body).candidateDegPerTdbDay) - p) * 86400;
    } catch (error) { if (!/not bracketed/.test(error.message)) throw error; }
    return { year: s.year, body: s.body, kind: s.kind, jdTdb: s.jdTdb, baseMatch, candidateMatch, baselineDeltaSeconds: baselineDelta, candidateDeltaSeconds: candidateDelta, samples };
  });
  const bracketed = rows.filter(r => r.baselineDeltaSeconds !== null && r.candidateDeltaSeconds !== null);
  const summary = { cases: rows.length, baselineSignMatches: rows.filter(r => r.baseMatch).length, candidateSignMatches: rows.filter(r => r.candidateMatch).length, bracketed: bracketed.length, baselineMaxSeconds: Math.max(...bracketed.map(r => Math.abs(r.baselineDeltaSeconds))), candidateMaxSeconds: Math.max(...bracketed.map(r => Math.abs(r.candidateDeltaSeconds))), candidateImprovedRoots: bracketed.filter(r => Math.abs(r.candidateDeltaSeconds) < Math.abs(r.baselineDeltaSeconds)).length };
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'experimental-longitude-stations/1', status: 'EXPERIMENTAL_NOT_ACCEPTED', kernelSha256: hashes[0], stationSourceSha256: hashes[1], sourceSchema: old.schema, swissPackageSha256: hashes[2], swissWrapperSha256: hashes[3], swissNativeSha256: hashes[4], addonSha256: hashes[5], scriptSha256: hashes[6], swissVersion: swiss.swe_version(), notes: 'Geocentric apparent mixed ecliptic; TDB versus approximate TT. Shared DE441; empirical candidate is not a position derivative and not an established Swiss algorithm. Four original Pluto cases informed the hypothesis; final bracket width 0.864 s is not physical timing accuracy.', summary, rows };
  fs.writeFileSync(out, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ summary, failedSigns: rows.filter(r => !r.candidateMatch).map(r => ({ year: r.year, body: r.body, kind: r.kind, deltaSeconds: r.candidateDeltaSeconds })) }));
}
if (require.main === module) { try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; } }
