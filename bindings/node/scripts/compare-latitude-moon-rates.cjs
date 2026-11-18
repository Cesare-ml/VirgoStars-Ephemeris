'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Offline Swiss compatibility investigation; no reference runtime dependency.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { Engine, julday } = require('../index.js');
const { tdbToTt } = require('./validate-mvp1-swiss-rates.cjs');
const { extrapolatedRate } = require('./compare-swiss-seasonal-speeds.cjs');
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function summarize(rows, base, candidate) {
  if (!rows.length || !rows.every(r => Number.isFinite(r[base]) && Number.isFinite(r[candidate]))) throw new Error('invalid summary rows');
  const rms = key => Math.hypot(...rows.map(r => r[key])) / Math.sqrt(rows.length);
  return { cases: rows.length, improved: rows.filter(r => Math.abs(r[candidate]) < Math.abs(r[base])).length, rmsBefore: rms(base), rmsCandidate: rms(candidate), maxBefore: Math.max(...rows.map(r => Math.abs(r[base]))), maxCandidate: Math.max(...rows.map(r => Math.abs(r[candidate]))) };
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
  const modes = { apparent: 0, noAberration: swiss.SEFLG_NOABERR, noDeflection: swiss.SEFLG_NOGDEFL, noAberrationOrDeflection: swiss.SEFLG_NOABERR | swiss.SEFLG_NOGDEFL, truePosition: swiss.SEFLG_TRUEPOS };
  if (!Object.entries(modes).every(([k, v]) => k === 'apparent' || Number.isInteger(v) && v > 0)) throw new Error('missing reference flags');
  function reference(jd, body, extraFlags = 0) {
    const flags = swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED | extraFlags;
    const r = swiss.swe_calc(tdbToTt(jd), body, flags);
    const values = ['longitude', 'latitude', 'distance', 'longitudeSpeed', 'latitudeSpeed', 'distanceSpeed'].map(k => r?.[k]);
    if (!r || r.error || (r.rflag & flags) !== flags || !values.every(Number.isFinite) || values[0] < 0 || values[0] >= 360 || Math.abs(values[1]) > 90 || values[2] <= 0) throw new Error(`reference unavailable ${jd} ${body} ${flags}`);
    return values;
  }
  const years = [-12500, -10000, -5000, 0, 1000, 2000, 3000, 3900], rows = [], moon = [];
  for (const year of years) for (const month of [1, 4, 7, 10]) {
    const jd = julday(year, month, 15, 12, true);
    for (let body = 0; body < 10; body++) {
      const b = reference(jd, body), c = engine.experimentalThreeRateCandidateTdb(jd, body), original = engine.experimentalCombinedRateCandidateTdb(jd, body), lat = engine.experimentalLatitudeRateCandidateTdb(jd, body);
      if (!c.base.xx.every(Number.isFinite) || !c.candidateXx.every(Number.isFinite) || c.base.xx.some((v, i) => v !== original.base.xx[i] || v !== lat.base.xx[i]) || [0, 1, 2, 3, 5].some(i => c.candidateXx[i] !== original.candidateXx[i]) || c.candidateXx[4] !== lat.candidateDegPerTdbDay) throw new Error('candidate contract mismatch');
      rows.push({ year, month, body, jdTdb: jd, latitudeBefore: (c.base.xx[4] - b[4]) * 3600, latitudeCandidate: (c.candidateXx[4] - b[4]) * 3600, longitudeCandidate: (c.candidateXx[3] - b[3]) * 3600 });
    }
    for (const day of [5, 15, 25]) {
      const t = julday(year, month, day, 12, true), result = {};
      for (const [mode, flags] of Object.entries(modes)) {
        const center = reference(t, 1, flags); result[mode] = {};
        for (const h of [0.01, 0.02, 0.04]) {
          const sample = [-2, -1, 1, 2].map(k => reference(t + k * h, 1, flags));
          const derivative = [0, 1].map(i => extrapolatedRate(...sample.map(x => x[i]), t, h, i === 0));
          result[mode][String(h)] = { longitude: (center[3] - derivative[0]) * 3600, latitude: (center[4] - derivative[1]) * 3600 };
        }
      }
      moon.push({ year, month, day, jdTdb: t, modes: result });
    }
  }
  const stats = list => summarize(list, 'latitudeBefore', 'latitudeCandidate');
  const moonModes = Object.fromEntries(Object.keys(modes).map(mode => [mode, Object.fromEntries([0.01, 0.02, 0.04].map(h => [String(h), Object.fromEntries(['longitude', 'latitude'].map(component => {
    const values = moon.map(r => r.modes[mode][String(h)][component]);
    return [component, { rms: Math.hypot(...values) / Math.sqrt(values.length), max: Math.max(...values.map(Math.abs)) }];
  }))]))]));
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'experimental-latitude-moon-rates/1', status: 'EXPERIMENTAL_NOT_ACCEPTED', inputSha256: hashes, swissVersion: swiss.swe_version(), units: 'arcsec/TDB day', notes: 'Apparent geocentric mixed ecliptic; TDB vs approximate TT; shared DE441. New dates relative to latitude discovery grid. True-position changes multiple corrections. Numerical derivatives are not exact, and candidate rates do not differentiate positions. No acceptance tolerance or physical accuracy claim.', summary: { latitude: stats(rows), latitudePerBody: Object.fromEntries(Array.from({ length: 10 }, (_, body) => [body, stats(rows.filter(r => r.body === body))])), moonCases: moon.length, moonModes }, rows, moon };
  fs.writeFileSync(out, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(report.summary));
}
module.exports = { summarize };
if (require.main === module) { try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; } }
