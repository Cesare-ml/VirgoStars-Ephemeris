'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Selects new Swiss-position-derivative station brackets; never alters the engine.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { julday } = require('../index.js');
const { checkedSwiss, tdbToTt } = require('./validate-mvp1-swiss-rates.cjs');
const { extrapolatedRate } = require('./compare-swiss-seasonal-speeds.cjs');
const { nearestBrackets } = require('./compare-swiss-station-matrix.cjs');
const { bisect } = require('./compare-swiss-station-roots.cjs');
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function main() {
  const [kernel, hash, swissDir, out, ...extra] = process.argv.slice(2);
  if (!kernel || !/^[a-f0-9]{64}$/.test(hash || '') || !swissDir || !out || extra.length || fs.existsSync(out)) throw new Error('usage: KERNEL SHA SWISSEPH NEW_REPORT');
  const pkg = path.resolve(swissDir), inputs = [kernel, path.join(pkg, 'package.json'), path.join(pkg, 'lib/swisseph.js'), path.join(pkg, 'build/Release/swisseph.node'), __filename].map(x => path.resolve(x));
  if (new Set([...inputs, path.resolve(out)]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== hash) throw new Error('kernel hash mismatch');
  const swiss = require(pkg);
  swiss.swe_set_ephe_path(path.dirname(inputs[0])); swiss.swe_set_jpl_file(path.basename(inputs[0]));
  const ref = (jd, body) => checkedSwiss(swiss.swe_calc(tdbToTt(jd), body, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED), body, jd)[0];
  const stations = [];
  for (const year of [-11000, 1500, 3500]) for (let body = 2; body <= 9; body++) {
    const seed = julday(year, 7, 1, 12, true), h = 0.02;
    const derivative = jd => extrapolatedRate(...[-2, -1, 1, 2].map(k => ref(jd + k * h, body)), jd, h, true);
    const found = nearestBrackets(derivative, seed, 500, 5);
    for (const kind of ['retrograde', 'direct']) {
      const initial = found[kind];
      if (!initial) throw new Error(`missing station bracket: ${year}, ${body}, ${kind}`);
      const b = bisect(derivative, initial[0] - 1, initial[1] + 1, 0.00001);
      stations.push({ status: 'ok', year, body, kind, initial, jdTdb: (b.lo + b.hi) / 2 });
    }
  }
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'longitude-candidate-station-selection/1', status: 'HOLDOUT_STATION_SELECTION_ONLY', kernelSha256: hashes[0], swissPackageSha256: hashes[1], swissWrapperSha256: hashes[2], swissNativeSha256: hashes[3], scriptSha256: hashes[4], swissVersion: swiss.swe_version(), notes: 'Swiss JPL/speed flags; fourth-order position derivative with 0.02 day half-step; TDB mapped approximately to TT. Selected before comparison with candidate.', stations };
  fs.writeFileSync(out, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ cases: stations.length, years: [-11000, 1500, 3500] }));
}
if (require.main === module) { try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; } }
