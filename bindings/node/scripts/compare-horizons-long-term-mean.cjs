'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Manual JPL query: pairwise angular separations are invariant under frame
// rotations, so this cannot validate precession, nutation or absolute longitude.
const fs = require('node:fs');
const crypto = require('node:crypto');
const { Engine, julday } = require('../index.js');
const bodies = [[0, '10', 'Sun'], [1, '301', 'Moon'], [2, '199', 'Mercury'], [3, '299', 'Venus'],
  [4, '4', 'Mars Barycenter'], [5, '5', 'Jupiter Barycenter'], [6, '6', 'Saturn Barycenter'],
  [7, '7', 'Uranus Barycenter'], [8, '8', 'Neptune Barycenter'], [9, '9', 'Pluto Barycenter']];
const years = [0, 1000, 2000, 3000, 4000, 5000];
const deg = Math.PI / 180;
const toTt = tdb => tdb - 0.001657 * Math.sin((357.5291092 + 35999.0502909 * ((tdb - 2451545) / 36525)) * deg) / 86400;
const unit = (a, b) => [Math.cos(a * deg) * Math.cos(b * deg), Math.sin(a * deg) * Math.cos(b * deg), Math.sin(b * deg)];
function separation(a, b) {
  const cross = [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]];
  return Math.atan2(Math.hypot(...cross), a[0]*b[0]+a[1]*b[1]+a[2]*b[2]) / deg * 3600;
}
async function hash(path) { const h = crypto.createHash('sha256'); for await (const chunk of fs.createReadStream(path)) h.update(chunk); return h.digest('hex'); }
async function main() {
  const [kernel, expected, output, mode, ...extra] = process.argv.slice(2);
  if (!kernel || !/^[a-f0-9]{64}$/.test(expected || '') || !output || !['mean', 'apparent'].includes(mode) || extra.length) throw new Error('usage: <kernel> <sha256> <new-output.json> <mean|apparent>');
  if (fs.existsSync(output) || await hash(kernel) !== expected) throw new Error('output exists or kernel hash mismatch');
  const engine = new Engine(kernel);
  const tdb = years.map(y => julday(y, 7, 1, 12, true));
  const tt = tdb.map(toTt);
  const rows = [];
  for (const [id, command, name] of bodies) {
    const params = new URLSearchParams({ format: 'json', COMMAND: `'${command}'`, OBJ_DATA: "'NO'", MAKE_EPHEM: "'YES'",
      EPHEM_TYPE: "'OBSERVER'", CENTER: "'500@399'", TLIST: `'${tt.map(x => x.toFixed(9)).join(' ')}'`, TLIST_TYPE: "'JD'", TIME_TYPE: "'TT'", CAL_FORMAT: "'JD'",
      QUANTITIES: "'2,20,31'", ANG_FORMAT: "'DEG'", CSV_FORMAT: "'YES'", EXTRA_PREC: "'YES'" });
    const url = `https://ssd.jpl.nasa.gov/api/horizons.api?${params}`;
    const response = await fetch(url, { signal: AbortSignal.timeout(30000) });
    if (!response.ok) throw new Error(`Horizons HTTP ${response.status}`);
    const json = await response.json(); const raw = json.result;
    if (typeof raw !== 'string' || !raw.includes(`Target body name: ${name} (${command})`)
        || !/Target body name:.*\{source: DE441\}/.test(raw) || !/Center body name: Earth \(399\).*\{source: DE441\}/.test(raw)) {
      throw new Error(`unexpected Horizons target metadata for ${name}`);
    }
    const header = raw.split('\n').find(line => line.includes('R.A.__(a-app)'));
    const start = raw.indexOf('$$SOE'); const end = raw.indexOf('$$EOE');
    if (!header || start < 0 || end <= start || !header.includes('Date_________JDTT')) throw new Error(`unexpected Horizons table for ${name}`);
    const cols = header.split(',').map(x => x.trim()); const ra = cols.indexOf('R.A.__(a-app)'); const dec = cols.indexOf('DEC___(a-app)');
    const lon = cols.indexOf('ObsEcLon'); const lat = cols.indexOf('ObsEcLat');
    const data = raw.slice(start + 5, end).trim().split('\n').map(line => line.split(',').map(x => x.trim()));
    if (data.length !== years.length || ra < 0 || dec < 0 || lon < 0 || lat < 0) throw new Error(`wrong Horizons rows for ${name}`);
    if (data.some((fields,i) => !fields[0] || !Number.isFinite(Number(fields[0])) || Math.abs(Number(fields[0])-tt[i]) > 1e-8)) throw new Error(`wrong TT epochs for ${name}`);
    const reference = data.map(fields => {
      if (!fields[ra] || !fields[dec] || !Number.isFinite(Number(fields[ra])) || !Number.isFinite(Number(fields[dec]))) throw new Error(`invalid Horizons angle for ${name}`);
      return unit(Number(fields[ra]), Number(fields[dec]));
    });
    const computed = tdb.map(jd => (mode === 'mean' ? engine.experimentalLongTermMeanEclipticTdb(jd, id)
      : engine.experimentalLongTermApparentTdb(jd, id)).xx);
    const calculated = computed.map(xx => unit(xx[0], xx[1]));
    const absoluteEclipticDiagnostic = data.map((fields, i) => {
      if (!fields[lon] || !fields[lat] || !Number.isFinite(Number(fields[lon])) || !Number.isFinite(Number(fields[lat]))) throw new Error(`invalid Horizons ecliptic for ${name}`);
      const signedLongitude = ((computed[i][0] - Number(fields[lon]) + 540) % 360) - 180;
      return { year: years[i], longitudeDifferenceArcsec: signedLongitude * 3600,
        latitudeDifferenceArcsec: (computed[i][1] - Number(fields[lat])) * 3600 };
    });
    rows.push({ name, command, url, raw, reference, calculated, absoluteEclipticDiagnostic });
  }
  let moonMaxArcsec = 0; let otherMaxArcsec = 0;
  for (let i = 0; i < rows.length; i++) for (let j = i + 1; j < rows.length; j++) for (let k = 0; k < years.length; k++) {
    const residual = Math.abs(separation(rows[i].reference[k], rows[j].reference[k]) - separation(rows[i].calculated[k], rows[j].calculated[k]));
    if (i === 1 || j === 1) moonMaxArcsec = Math.max(moonMaxArcsec, residual);
    else otherMaxArcsec = Math.max(otherMaxArcsec, residual);
  }
  if (await hash(kernel) !== expected) throw new Error('kernel changed during comparison');
  // Thresholds declared before comparison; they are diagnostics, not release criteria.
  const limits = { moonArcsec: 0.05, otherArcsec: 0.01 };
  const passed = moonMaxArcsec <= limits.moonArcsec && otherMaxArcsec <= limits.otherArcsec;
  fs.writeFileSync(output, JSON.stringify({ schema: 'virgostars-horizons-long-term-separations/1', kernelSha256: expected,
    mode, years, tdb, tt, limits, moonMaxArcsec, otherMaxArcsec, pairs: 45 * years.length, passed, rows }, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ moonMaxArcsec, otherMaxArcsec, pairs: 45 * years.length, limits, passed }));
  if (!passed) process.exitCode = 1;
}
main().catch(error => { console.error(error.message); process.exitCode = 1; });
