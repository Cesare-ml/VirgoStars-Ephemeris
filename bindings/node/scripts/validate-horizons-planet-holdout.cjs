'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Implementation comparison using shared DE441, not independent orbit accuracy.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');
const { Engine, julday } = require('../index.js');
const { parse } = require('./compare-horizons-orientation-components.cjs');
const { unit, separation } = require('./compare-horizons-frame-isolation.cjs');
const { ttToTdb, direction, checkCoordinates, checkConsistency, summarize } = require('./validate-horizons-planet-frame-grid.cjs');

const bodies = [[0, '10', 'Sun'], [1, '301', 'Moon'], [2, '199', 'Mercury'], [3, '299', 'Venus'], [4, '4', 'Mars Barycenter'], [5, '5', 'Jupiter Barycenter'], [6, '6', 'Saturn Barycenter'], [7, '7', 'Uranus Barycenter'], [8, '8', 'Neptune Barycenter'], [9, '9', 'Pluto Barycenter']];
const years = [-9950, -9250, -8750, -7750, -6750, -5750, -4750, -3750, -2750, -1750, -750, 250, 1250, 2750, 3975];
function epochGrid() {
  return years.flatMap(year => [2, 8].map(month => ({ year, month, day: 17, hourTt: 6, jdTt: julday(year, month, 17, 6, true) })));
}
function digest(file) {
  const hash = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; hash.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return hash.digest('hex');
}
function urlFor(command, epochs) {
  if (!bodies.some(b => b[1] === command) || !Array.isArray(epochs) || epochs.length !== 30 || !epochs.every(Number.isFinite)) throw new Error('invalid query');
  const q = new URLSearchParams({ format: 'json', COMMAND: `'${command}'`, OBJ_DATA: "'NO'", MAKE_EPHEM: "'YES'", EPHEM_TYPE: "'OBSERVER'", CENTER: "'500@399'", TLIST: `'${epochs.map(x => x.toFixed(9)).join(' ')}'`, TLIST_TYPE: "'JD'", TIME_TYPE: "'TT'", CAL_FORMAT: "'JD'", QUANTITIES: "'2,31,45'", REF_SYSTEM: "'ICRF'", ANG_FORMAT: "'DEG'", CSV_FORMAT: "'YES'", EXTRA_PREC: "'YES'", APPARENT: "'AIRLESS'" });
  return `https://ssd.jpl.nasa.gov/api/horizons.api?${q}`;
}
async function main() {
  const [kernel, kernelHash, projector, out, prior, priorHash, ...extra] = process.argv.slice(2);
  if (!kernel || !/^[a-f0-9]{64}$/.test(kernelHash || '') || !projector || !out || extra.length || Boolean(prior) !== Boolean(priorHash) || (prior && !/^[a-f0-9]{64}$/.test(priorHash)) || fs.existsSync(out)) throw new Error('usage: KERNEL KERNEL_SHA PROJECTOR NEW_REPORT [PRIOR_REPORT PRIOR_SHA]');
  const addon = path.resolve(__dirname, '../native/virgostars_ephemeris.node');
  const inputs = [path.resolve(kernel), path.resolve(projector), addon, ...(prior ? [path.resolve(prior)] : [])];
  if (new Set([...inputs, path.resolve(out)]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== kernelHash || (prior && hashes[3] !== priorHash)) throw new Error('input hash mismatch');
  const previous = prior ? JSON.parse(fs.readFileSync(prior, 'utf8')) : null;
  if (previous && (previous.schema !== 'horizons-planet-holdout/1' || previous.kernelSha256 !== kernelHash || !Array.isArray(previous.responses) || previous.responses.length !== bodies.length)) throw new Error('invalid prior report');
  const grid = epochGrid(), epochs = grid.map(r => r.jdTt), responses = [];
  for (const [bodyId, command, name] of bodies) {
    const url = urlFor(command, epochs);
    let raw;
    if (previous) {
      const matches = previous.responses.filter(r => r.name === name && r.id === command && r.url === url);
      if (matches.length !== 1 || typeof matches[0].raw !== 'string') throw new Error(`invalid stored response for ${name}`);
      raw = matches[0].raw;
    } else {
      let response;
      for (let attempt = 0; attempt < 3; attempt++) {
        response = await fetch(url, { signal: AbortSignal.timeout(30000) });
        if (response.ok || ![429, 502, 503, 504].includes(response.status)) break;
        if (attempt < 2) await new Promise(resolve => setTimeout(resolve, 1500 * (attempt + 1)));
      }
      if (!response.ok) throw new Error(`Horizons HTTP ${response.status}`);
      const payload = await response.json();
      if (payload.error || typeof payload.result !== 'string') throw new Error(`Horizons: ${payload.error || 'missing result'}`);
      raw = payload.result;
    }
    responses.push({ bodyId, id: command, name, url, raw, coordinates: checkCoordinates(parse(raw, name, command, epochs)) });
  }
  const input = responses.flatMap(r => r.coordinates.map((c, i) => `${epochs[i]},${c.raDeg},${c.decDeg}`)).join('\n') + '\n';
  const lines = execFileSync(inputs[1], [], { input, encoding: 'utf8', maxBuffer: 2 * 1048576, timeout: 30000 }).trim().split(/\r?\n/);
  if (lines.length !== bodies.length * epochs.length) throw new Error('wrong projection count');
  const projected = lines.map(line => { const f = line.split(','); if (f.length !== 10 || !f.every(x => x !== '' && Number.isFinite(Number(x)))) throw new Error('invalid projection'); return f.map(Number); });
  const engine = new Engine(inputs[0]), rows = [];
  for (let b = 0; b < responses.length; b++) for (let i = 0; i < epochs.length; i++) {
    const ref = responses[b].coordinates[i], p = projected[b * epochs.length + i], tdb = ttToTdb(epochs[i]);
    const icrf = engine.experimentalApparentIcrfTdb(tdb, responses[b].bodyId), pole = engine.experimentalLongTermApparentTdb(tdb, responses[b].bodyId), direct = engine.experimentalLongTermDirectObliquityTdb(tdb, responses[b].bodyId);
    if (![pole.xx, direct.xx].every(v => v.length === 6 && v.every(Number.isFinite))) throw new Error('invalid local output');
    const refIcrf = unit(ref.raDeg, ref.decDeg), refEcliptic = unit(ref.lon, ref.lat), sunIcrf = unit(responses[0].coordinates[i].raDeg, responses[0].coordinates[i].decDeg);
    const localPole = unit(pole.xx[0], pole.xx[1]), localDirect = unit(direct.xx[0], direct.xx[1]), projectedPole = unit(p[4], p[5]), projectedDirect = unit(p[6], p[7]);
    const row = { ...grid[i], id: responses[b].bodyId, body: responses[b].name, jdTdb: tdb, solarElongationDeg: separation(refIcrf, sunIcrf) / 3600,
      icrfArcsec: separation(direction(icrf.positionKm), refIcrf), poleArcsec: separation(localPole, refEcliptic), directArcsec: separation(localDirect, refEcliptic),
      samePoleArcsec: separation(localPole, projectedPole), sameDirectArcsec: separation(localDirect, projectedDirect), poleFrameArcsec: separation(projectedPole, refEcliptic), directFrameArcsec: separation(projectedDirect, refEcliptic) };
    checkConsistency(row); rows.push(row);
  }
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const report = { schema: 'horizons-planet-holdout/1', status: 'EXPLORATORY', kernelSha256: hashes[0], projectorSha256: hashes[1], addonSha256: hashes[2], priorSha256: priorHash || null,
    notes: 'New held-out TT instants, geocentric airless apparent ICRF and true ecliptic of date; TDB approximate. Shared DE441 is not independent orbital truth. Frame conventions differ; no scientific tolerance accepted.', summary: summarize(rows), rows,
    responses: responses.map(({ id, name, url, raw }) => ({ id, name, url, raw })) };
  fs.writeFileSync(out, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(report.summary));
}
module.exports = { epochGrid, urlFor };
if (require.main === module) main().catch(e => { console.error(e.message); process.exitCode = 1; });
