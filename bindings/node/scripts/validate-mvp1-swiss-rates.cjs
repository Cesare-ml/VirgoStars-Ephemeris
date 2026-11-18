'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Finite Swiss compatibility exercise. Shared DE441 is not independent orbital truth.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { Engine, julday } = require('../index.js');
const { extrapolatedRate } = require('./compare-swiss-seasonal-speeds.cjs');
const { nearestBrackets } = require('./compare-swiss-station-matrix.cjs');
const { bisect } = require('./compare-swiss-station-roots.cjs');
const { unit, separation } = require('./compare-horizons-frame-isolation.cjs');
const { nextDown } = require('./audit-de441-epoch-quantization.cjs');
const { projectedDelayAcceleration } = require('./audit-swiss-apparent-rate-flags.cjs');

const YEARS = [-12500, 0, 3900];
const POSITION_LIMIT_ARCSEC = 1; // Owner-selected positional criterion only.
function tdbToTt(jd) { return jd - 0.001657 * Math.sin((357.5291092 + 35999.0502909 * ((jd - 2451545) / 36525)) * Math.PI / 180) / 86400; }
function digest(file) {
  const h = crypto.createHash('sha256'), fd = fs.openSync(file, 'r'), buffer = Buffer.allocUnsafe(1048576);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; h.update(buffer.subarray(0, n)); } } finally { fs.closeSync(fd); }
  return h.digest('hex');
}
function checkedSwiss(result, body, jd) {
  const fields = ['longitude', 'latitude', 'distance', 'longitudeSpeed', 'latitudeSpeed', 'distanceSpeed'];
  if (!result || result.error || result.rflag !== 257 || !fields.every(k => Number.isFinite(result[k])) || result.longitude < 0 || result.longitude >= 360 || Math.abs(result.latitude) > 90 || result.distance <= 0) throw new Error(`Swiss JPL/speed result unavailable: body ${body}, TDB ${jd}`);
  return fields.map(k => result[k]);
}
function checkedLocal(result) {
  const xx = result?.xx;
  if (!Array.isArray(xx) || xx.length !== 6 || !xx.every(Number.isFinite) || xx[0] < 0 || xx[0] >= 360 || Math.abs(xx[1]) > 90 || xx[2] <= 0) throw new Error('invalid local state');
  return xx;
}
function expectError(fn, expected) {
  try { fn(); } catch (error) { if (error.code === expected) return expected; throw error; }
  throw new Error(`expected ${expected}`);
}
function summary(points, stations) {
  if (!Array.isArray(points) || !points.length || !points.every(r => Number.isInteger(r.body) && r.positionArcsec >= 0 && [r.positionArcsec, r.lonRateSwissArcsecDay, r.lonRateDerivativeArcsecDay, r.latRateSwissArcsecDay, r.latRateDerivativeArcsecDay, r.distanceRateSwissAuDay, r.distanceRateDerivativeAuDay, r.radialCandidateResidualAuDay].every(Number.isFinite)) || !['endpoint', 'near-sun'].every(category => points.some(r => r.category === category)) || !Array.isArray(stations) || !stations.length || !stations.every(r => typeof r.status === 'string' && (r.status !== 'ok' || (typeof r.signsMatch === 'boolean' && typeof r.publishedSignsMatch === 'boolean' && Number.isFinite(r.engineMinusSwissDerivativeSeconds) && (r.engineMinusSwissPublishedSeconds === null || Number.isFinite(r.engineMinusSwissPublishedSeconds)))))) throw new Error('invalid results');
  const maximum = (list, field, abs = false) => {
    if (!list.length) return null;
    const row = list.reduce((a, b) => (abs ? Math.abs(b[field]) : b[field]) > (abs ? Math.abs(a[field]) : a[field]) ? b : a);
    return { absolute: Math.abs(row[field]), body: row.body, year: row.year, jdTdb: row.jdTdb, category: row.category };
  };
  const group = list => ({ cases: list.length, position: maximum(list, 'positionArcsec'), lonVsSwissPublished: maximum(list, 'lonRateSwissArcsecDay', true), lonVsSwissDerivative: maximum(list, 'lonRateDerivativeArcsecDay', true), latVsSwissPublished: maximum(list, 'latRateSwissArcsecDay', true), latVsSwissDerivative: maximum(list, 'latRateDerivativeArcsecDay', true), distanceVsSwissPublished: maximum(list, 'distanceRateSwissAuDay', true), distanceVsSwissDerivative: maximum(list, 'distanceRateDerivativeAuDay', true), radialCandidateVsSwiss: maximum(list, 'radialCandidateResidualAuDay', true), radialRmsBeforeAuDay: Math.hypot(...list.map(r => r.distanceRateSwissAuDay)) / Math.sqrt(list.length), radialRmsCandidateAuDay: Math.hypot(...list.map(r => r.radialCandidateResidualAuDay)) / Math.sqrt(list.length), radialCandidateImproved: list.filter(r => Math.abs(r.radialCandidateResidualAuDay) < Math.abs(r.distanceRateSwissAuDay)).length, positionFailures: list.filter(r => r.positionArcsec > POSITION_LIMIT_ARCSEC).length });
  const ok = stations.filter(r => r.status === 'ok');
  const published = ok.filter(r => r.engineMinusSwissPublishedSeconds !== null);
  return { points: group(points), endpoints: group(points.filter(r => r.category === 'endpoint')), nearSun: group(points.filter(r => r.category === 'near-sun')), stations: { requested: stations.length, bracketed: ok.length, unresolved: stations.length - ok.length, derivativeSignsMatch: ok.filter(r => r.signsMatch).length, derivativeSignsMismatch: ok.filter(r => !r.signsMatch).length, publishedSignsMatch: ok.filter(r => r.publishedSignsMatch).length, publishedSignsMismatch: ok.filter(r => !r.publishedSignsMatch).length, maxDerivativeDifferenceSeconds: ok.length ? Math.max(...ok.map(r => Math.abs(r.engineMinusSwissDerivativeSeconds))) : null, maxEngineVsPublishedSeconds: published.length ? Math.max(...published.map(r => Math.abs(r.engineMinusSwissPublishedSeconds))) : null, publishedUnresolved: ok.length - published.length } };
}
function gateFailed(result) {
  return Boolean(result.points.positionFailures || result.stations.unresolved || result.stations.publishedUnresolved || result.stations.publishedSignsMismatch);
}
function main() {
  const [kernel, kernelHash, swissDir, nearSunFile, nearSunHash, out, ...extra] = process.argv.slice(2);
  if (!kernel || !swissDir || !nearSunFile || !out || extra.length || ![kernelHash, nearSunHash].every(x => /^[a-f0-9]{64}$/.test(x || '')) || fs.existsSync(out)) throw new Error('usage: KERNEL KERNEL_SHA SWISSEPH_PACKAGE NEAR_SUN_REPORT NEAR_SUN_SHA NEW_REPORT');
  const pkg = path.resolve(swissDir), inputs = [kernel, nearSunFile, path.join(pkg, 'package.json'), path.join(pkg, 'lib/swisseph.js'), path.join(pkg, 'build/Release/swisseph.node'), path.resolve(__dirname, '../native/virgostars_ephemeris.node'), __filename].map(p => path.resolve(p));
  if (new Set([...inputs, path.resolve(out)]).size !== inputs.length + 1) throw new Error('inputs and output must differ');
  const hashes = inputs.map(digest);
  if (hashes[0] !== kernelHash || hashes[1] !== nearSunHash) throw new Error('input hash mismatch');
  const near = JSON.parse(fs.readFileSync(inputs[1], 'utf8'));
  if (near.schema !== 'emission-deflection-matrix/1' || near.kernelSha256 !== kernelHash || !Array.isArray(near.rows) || near.rows.length !== 14 || !near.rows.every(r => Number.isInteger(r.id) && r.id >= 2 && r.id <= 8 && Number.isInteger(r.year) && Number.isFinite(r.jd) && Number.isFinite(r.elongationDeg) && r.elongationDeg > 0 && r.elongationDeg < 3)) throw new Error('invalid near-Sun input');
  const swiss = require(pkg), engine = new Engine(inputs[0]);
  swiss.swe_set_ephe_path(path.dirname(inputs[0])); swiss.swe_set_jpl_file(path.basename(inputs[0]));
  const ref = (jd, id) => checkedSwiss(swiss.swe_calc(tdbToTt(jd), id, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED), id, jd);
  const local = (jd, id) => checkedLocal(engine.experimentalLongTermDirectObliquityTdb(jd, id));
  const derivative = (fn, jd, id, component, h = 0.02) => {
    const a = [-2, -1, 1, 2].map(k => fn(jd + k * h, id)[component]);
    return extrapolatedRate(...a, jd, h, component === 0);
  };
  const targets = [
    { year: -12999, month: 1, day: 1, hour: 0, category: 'endpoint' },
    { year: 3999, month: 12, day: 31, hour: null, jdTdb: nextDown(julday(4000, 1, 1, 0, true)), category: 'endpoint' },
    ...[-12500, -10000, -5000, 0, 1000, 2000, 3000, 3900].map(year => ({ year, month: 7, day: 1, hour: 12, category: 'interior' })),
  ].flatMap(x => Array.from({ length: 10 }, (_, body) => ({ ...x, body, jdTdb: x.jdTdb ?? julday(x.year, x.month, x.day, x.hour, true) })));
  for (const r of near.rows) targets.push({ body: r.id, year: r.year, jdTdb: r.jd, category: 'near-sun', storedElongationDeg: r.elongationDeg });
  const points = targets.map(t => {
    const a = local(t.jdTdb, t.body), b = ref(t.jdTdb, t.body), localFd = [0, 1, 2].map(i => derivative(local, t.jdTdb, t.body, i)), swissFd = [0, 1, 2].map(i => derivative(ref, t.jdTdb, t.body, i));
    const h = 0.02, left = t.jdTdb - h, right = t.jdTdb + h;
    const before = engine.geometricStateTdb(left, 0).velocityKmPerDay, after = engine.geometricStateTdb(right, 0).velocityKmPerDay;
    const position = engine.experimentalApparentIcrfTdb(t.jdTdb, t.body).positionKm;
    const radialCandidateCorrectionAuDay = projectedDelayAcceleration(position, before, after, right - left);
    let actualElongationDeg = null;
    if (t.category === 'near-sun') {
      const target = engine.experimentalApparentIcrfTdb(t.jdTdb, t.body).positionKm, sun = engine.experimentalApparentIcrfTdb(t.jdTdb, 0).positionKm;
      const norm = v => { if (!Array.isArray(v) || v.length !== 3 || !v.every(Number.isFinite) || !(Math.hypot(...v) > 0)) throw new Error('invalid elongation direction'); return v.map(x => x / Math.hypot(...v)); };
      actualElongationDeg = separation(norm(target), norm(sun)) / 3600;
      if (!(actualElongationDeg > 0 && actualElongationDeg < 3)) throw new Error('near-Sun geometry outside selection');
    }
    return { ...t, jdTt: tdbToTt(t.jdTdb), actualElongationDeg, positionArcsec: separation(unit(a[0], a[1]), unit(b[0], b[1])),
      lonRateSwissArcsecDay: (a[3] - b[3]) * 3600, lonRateDerivativeArcsecDay: (a[3] - swissFd[0]) * 3600, lonRateOwnDerivativeArcsecDay: (a[3] - localFd[0]) * 3600,
      latRateSwissArcsecDay: (a[4] - b[4]) * 3600, latRateDerivativeArcsecDay: (a[4] - swissFd[1]) * 3600, latRateOwnDerivativeArcsecDay: (a[4] - localFd[1]) * 3600,
      distanceRateSwissAuDay: a[5] - b[5], distanceRateDerivativeAuDay: a[5] - swissFd[2], distanceRateOwnDerivativeAuDay: a[5] - localFd[2], radialCandidateCorrectionAuDay, radialCandidateResidualAuDay: a[5] + radialCandidateCorrectionAuDay - b[5] };
  });
  const stations = [];
  for (const year of YEARS) for (let body = 2; body <= 9; body++) {
    const seed = julday(year, 7, 1, 12, true), swissDerivative = jd => derivative(ref, jd, body, 0), ours = jd => local(jd, body)[3], published = jd => ref(jd, body)[3];
    const found = nearestBrackets(swissDerivative, seed, 500, 5);
    for (const kind of ['retrograde', 'direct']) {
      const initial = found[kind];
      if (!initial) { stations.push({ year, body, kind, status: 'missing-bracket' }); continue; }
      const lo = initial[0] - 1, hi = initial[1] + 1;
      let sb, eb;
      try { sb = bisect(swissDerivative, lo, hi, 0.00001); eb = bisect(ours, lo, hi, 0.00001); }
      catch (error) { if (!/not bracketed/.test(error.message)) throw error; stations.push({ year, body, kind, status: 'unresolved', initial }); continue; }
      const s = (sb.lo + sb.hi) / 2, e = (eb.lo + eb.hi) / 2, delta = 120 / 86400;
      const swissSigns = [-1, 1].map(k => Math.sign(swissDerivative(s + k * delta))), localSigns = [-1, 1].map(k => Math.sign(ours(s + k * delta))), publishedSigns = [-1, 1].map(k => Math.sign(published(s + k * delta)));
      const expected = kind === 'retrograde' ? [1, -1] : [-1, 1];
      let publishedDelta = null, enginePublishedDelta = null;
      try { const pb = bisect(published, lo, hi, 0.00001), p = (pb.lo + pb.hi) / 2; publishedDelta = (p - s) * 86400; enginePublishedDelta = (e - p) * 86400; }
      catch (error) { if (!/not bracketed/.test(error.message)) throw error; }
      stations.push({ year, body, kind, status: 'ok', jdTdb: s, initial, engineBracket: eb, swissDerivativeBracket: sb, engineMinusSwissDerivativeSeconds: (e - s) * 86400, swissPublishedMinusDerivativeSeconds: publishedDelta, engineMinusSwissPublishedSeconds: enginePublishedDelta, swissSigns, localSigns, publishedSigns, signsMatch: expected.every((v, i) => v === swissSigns[i] && v === localSigns[i]), publishedSignsMatch: expected.every((v, i) => v === publishedSigns[i] && v === localSigns[i]) });
    }
  }
  const first = julday(-12999, 1, 1, 0, true), nextYear = julday(4000, 1, 1, 0, true);
  const boundaries = { firstTdb: first, finalTdb: nextDown(nextYear), standardAtFirst: expectError(() => engine.calcTdb(first, 4), 'E_CALC'), standardAtFinal: expectError(() => engine.calcTdb(nextDown(nextYear), 4), 'E_CALC'), numericGuardBelow: expectError(() => local(julday(-13001, 7, 1, 12, true), 4), 'E_CALC'), numericGuardAbove: expectError(() => local(julday(5002, 7, 1, 12, true), 4), 'E_CALC'), invalidBody: expectError(() => local(first, 15), 'E_UNSUPPORTED_BODY'), nonFiniteEpoch: expectError(() => local(NaN, 4), 'E_RANGE'),
    adjacentOutsideRequestedStillSupported: [nextDown(first), nextYear].every(jd => local(jd, 4).every(Number.isFinite)) };
  if (inputs.some((p, i) => digest(p) !== hashes[i])) throw new Error('input changed');
  const result = { schema: 'mvp1-swiss-rate-boundaries/1', status: 'OPEN_SWISS_PUBLISHED_SPEED_COMPATIBILITY', kernelSha256: hashes[0], nearSunSourceSha256: hashes[1], swissPackageSha256: hashes[2], swissWrapperSha256: hashes[3], swissNativeSha256: hashes[4], addonSha256: hashes[5], auditScriptSha256: hashes[6], swissVersion: swiss.swe_version(), positionLimitArcsec: POSITION_LIMIT_ARCSEC, boundaries,
    notes: 'Geocentric apparent mixed direct-obliquity ecliptic, TDB vs approximate TT; shared DE441. Fixed-grid speed and station comparisons use Swiss published speeds and position derivatives separately. Radial candidate is a diagnostic approximation, not a production correction. No velocity/station tolerance or physical historical accuracy is implied.', summary: summary(points, stations), points, stations };
  fs.writeFileSync(out, JSON.stringify(result, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(result.summary));
  if (gateFailed(result.summary)) process.exitCode = 2;
}
module.exports = { checkedSwiss, checkedLocal, expectError, summary, gateFailed, tdbToTt };
if (require.main === module) { try { main(); } catch (e) { console.error(e.message); process.exitCode = 1; } }
