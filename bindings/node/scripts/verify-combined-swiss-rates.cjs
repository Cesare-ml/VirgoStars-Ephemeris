'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Offline check of the combined path using previously selected samples.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { Engine } = require('../index.js');
const { checkedSwiss, tdbToTt, matchingNonzeroSigns } = require('./validate-mvp1-swiss-rates.cjs');
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function main() {
  const [kernel, kernelSha, swissDir, stations1, stationSha1, stations2, stationSha2, radial, radialSha, out, ...extra] = process.argv.slice(2);
  if (!kernel || !swissDir || !stations1 || !stations2 || !radial || !out || extra.length || ![kernelSha, stationSha1, stationSha2, radialSha].every(x => /^[a-f0-9]{64}$/.test(x || '')) || fs.existsSync(out)) throw new Error('usage: KERNEL SHA SWISSEPH STATIONS_1 SHA STATIONS_2 SHA RADIAL_GRID SHA NEW_REPORT');
  const pkg = path.resolve(swissDir), inputs = [kernel, stations1, stations2, radial, path.join(pkg, 'package.json'), path.join(pkg, 'lib/swisseph.js'), path.join(pkg, 'build/Release/swisseph.node'), path.resolve(__dirname, '../native/virgostars_ephemeris.node'), __filename].map(x => path.resolve(x));
  if (new Set([...inputs, path.resolve(out)]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (![kernelSha, stationSha1, stationSha2, radialSha].every((h, i) => h === hashes[i])) throw new Error('input hash mismatch');
  const sources = inputs.slice(1, 4).map(file => JSON.parse(fs.readFileSync(file, 'utf8')));
  if (!sources.slice(0, 2).every(s => s.schema === 'experimental-longitude-stations/1' && s.kernelSha256 === kernelSha && Array.isArray(s.rows) && s.rows.length === 48 && s.rows.every(r => Number.isInteger(r.body) && Array.isArray(r.samples) && r.samples.length === 2 && r.samples.every(x => Number.isFinite(x.jdTdb)))) || sources[2].schema !== 'experimental-radial-candidate-holdout/1' || sources[2].grid !== 'confirmation' || sources[2].kernelSha256 !== kernelSha || !Array.isArray(sources[2].rows) || sources[2].rows.length !== 320 || !sources[2].rows.every(r => Number.isInteger(r.body) && Number.isFinite(r.jdTdb))) throw new Error('invalid source reports');
  const swiss = require(pkg), engine = new Engine(inputs[0]);
  swiss.swe_set_ephe_path(path.dirname(inputs[0])); swiss.swe_set_jpl_file(path.basename(inputs[0]));
  const ref = (jd, body) => checkedSwiss(swiss.swe_calc(tdbToTt(jd), body, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED), body, jd);
  function combined(jd, body) {
    const c = engine.experimentalCombinedRateCandidateTdb(jd, body), l = engine.experimentalLongitudeRateCandidateTdb(jd, body), r = engine.experimentalRadialRateCandidateTdb(jd, body);
    if (!Array.isArray(c.base.xx) || c.base.xx.length !== 6 || !c.base.xx.every(Number.isFinite) || !Array.isArray(c.candidateXx) || c.candidateXx.length !== 6 || !c.candidateXx.every(Number.isFinite) || c.candidateXx[3] !== l.candidateDegPerTdbDay || c.candidateXx[5] !== r.candidateAuPerTdbDay || c.radialCorrectionApplied !== r.correctionApplied || c.base.xx.some((v, i) => v !== l.base.xx[i] || v !== r.base.xx[i]) || [0, 1, 2, 4].some(i => c.candidateXx[i] !== c.base.xx[i])) throw new Error('combined/individual contract mismatch');
    return c;
  }
  const stations = sources.slice(0, 2).flatMap((s, sourceIndex) => s.rows.map(row => {
    const values = row.samples.map(sample => { const c = combined(sample.jdTdb, row.body), b = ref(sample.jdTdb, row.body); return { jdTdb: sample.jdTdb, baselineSign: Math.sign(c.base.xx[3]), candidateSign: Math.sign(c.candidateXx[3]), referenceSign: Math.sign(b[3]) }; });
    return { sourceIndex, body: row.body, year: row.year, kind: row.kind, baselineMatch: matchingNonzeroSigns(values.map(x => x.baselineSign), values.map(x => x.referenceSign)), candidateMatch: matchingNonzeroSigns(values.map(x => x.candidateSign), values.map(x => x.referenceSign)), values };
  }));
  const radialRows = sources[2].rows.map(row => {
    const c = combined(row.jdTdb, row.body), b = ref(row.jdTdb, row.body);
    return { body: row.body, jdTdb: row.jdTdb, correctionApplied: c.radialCorrectionApplied, baselineResidual: c.base.xx[5] - b[5], candidateResidual: c.candidateXx[5] - b[5] };
  });
  const summary = { stations: stations.length, baselineStationMatches: stations.filter(r => r.baselineMatch).length, combinedStationMatches: stations.filter(r => r.candidateMatch).length, radialCases: radialRows.length, radialImproved: radialRows.filter(r => Math.abs(r.candidateResidual) < Math.abs(r.baselineResidual)).length, radialRmsBeforeAuDay: Math.hypot(...radialRows.map(r => r.baselineResidual)) / Math.sqrt(radialRows.length), radialRmsCandidateAuDay: Math.hypot(...radialRows.map(r => r.candidateResidual)) / Math.sqrt(radialRows.length), sunUnchanged: radialRows.filter(r => r.body === 0).every(r => !r.correctionApplied && r.baselineResidual === r.candidateResidual) };
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'experimental-combined-swiss-rates/1', status: 'EXPERIMENTAL_NOT_ACCEPTED', inputSha256: hashes, swissVersion: swiss.swe_version(), notes: 'Geocentric apparent mixed ecliptic; shared DE441; TDB vs approximate TT. The first station set informed the angular hypothesis. Candidate rates are not derivatives; latitude remains unchanged. No continuous or historical accuracy implied.', summary, stations, radialRows };
  fs.writeFileSync(out, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(summary));
}
if (require.main === module) { try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; } }
