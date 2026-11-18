'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Explicit offline compatibility and numerical-resolution audit; no Swiss runtime dependency.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { Engine, julday } = require('../index.js');
const { checkedSwiss, tdbToTt } = require('./validate-mvp1-swiss-rates.cjs');
const { extrapolatedRate } = require('./compare-swiss-seasonal-speeds.cjs');
const { nextDown } = require('./audit-de441-epoch-quantization.cjs');
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function summary(rows, key) {
  if (!rows.length || !rows.every(r => Number.isFinite(r[key]))) throw new Error('invalid result');
  const worst = rows.reduce((a, b) => Math.abs(b[key]) > Math.abs(a[key]) ? b : a);
  return { cases: rows.length, rms: Math.hypot(...rows.map(r => r[key])) / Math.sqrt(rows.length), max: Math.abs(worst[key]), worstJdTdb: worst.jdTdb, worstCategory: worst.category };
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
  const ref = jd => checkedSwiss(swiss.swe_calc(tdbToTt(jd), 1, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED), 1, jd);
  const epochs = [];
  for (const year of [-12999, -12500, -10000, -5000, 0, 2000, 3900, 3999]) for (let month = 1; month <= 12; month++) for (const day of [3, 21]) epochs.push({ year, month, day, category: 'grid', jdTdb: julday(year, month, day, 12, true) });
  epochs.push({ category: 'first', jdTdb: julday(-12999, 1, 1, 0, true) }, { category: 'last', jdTdb: nextDown(julday(4000, 1, 1, 0, true)) });
  const wrap = delta => ((delta + 540) % 360) - 180, h = 0.02, tiny = 0.00002;
  const rows = epochs.map(epoch => {
    const jd = epoch.jdTdb, reference = ref(jd), samples = [-2, -1, 1, 2].map(k => ref(jd + k * h));
    const longDerivative = extrapolatedRate(...samples.map(x => x[0]), jd, h, true);
    const forwardReference = ref(jd + tiny), forwardSpan = (jd + tiny) - jd;
    if (!(forwardSpan > 0)) throw new Error('short interval not representable');
    const shortForward = wrap(forwardReference[0] - reference[0]) / forwardSpan;
    const local = engine.experimentalCombinedRateCandidateTdb(jd, 1);
    if (!Array.isArray(local.base.xx) || local.base.xx.length !== 6 || !local.base.xx.every(Number.isFinite) || !Array.isArray(local.candidateXx) || local.candidateXx.length !== 6 || !local.candidateXx.every(Number.isFinite) || [0, 1, 2, 4].some(i => local.base.xx[i] !== local.candidateXx[i])) throw new Error('invalid candidate');
    return { ...epoch, longitudeBaseMinusSwiss: (local.base.xx[3] - reference[3]) * 3600, longitudeCandidateMinusSwiss: (local.candidateXx[3] - reference[3]) * 3600, latitudeBaseMinusSwiss: (local.base.xx[4] - reference[4]) * 3600, radialBaseMinusSwiss: local.base.xx[5] - reference[5], radialCandidateMinusSwiss: local.candidateXx[5] - reference[5], swissPublishedMinusLongDerivative: (reference[3] - longDerivative) * 3600, swissPublishedMinusShortForward: (reference[3] - shortForward) * 3600 };
  });
  const names = ['longitudeBaseMinusSwiss', 'longitudeCandidateMinusSwiss', 'latitudeBaseMinusSwiss', 'radialBaseMinusSwiss', 'radialCandidateMinusSwiss', 'swissPublishedMinusLongDerivative', 'swissPublishedMinusShortForward'];
  const summaries = Object.fromEntries(names.map(k => [k, summary(rows, k)]));
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'lunar-speed-resolution/1', status: 'OPEN_NO_ACCEPTED_RATE_TOLERANCE', inputSha256: hashes, swissVersion: swiss.swe_version(), notes: 'Moon, geocentric apparent mixed ecliptic, shared DE441; TDB vs approximate TT. Long difference is fourth order centered with 0.02-day half-step; short difference is one-sided over 0.00002 day. No continuous bound, physical historical accuracy, or inference about Swiss internals.', summaries, rows };
  fs.writeFileSync(out, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(summaries));
}
module.exports = { summary };
if (require.main === module) { try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; } }
