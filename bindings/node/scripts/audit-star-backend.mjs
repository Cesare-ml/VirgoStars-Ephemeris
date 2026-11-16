// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Local diagnostic only: caller-supplied catalogs, kernel, backend and reference.
// Prints aggregates; never writes catalog data or Swiss reference rows.
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { basename, dirname, resolve } from 'node:path';

const require = createRequire(import.meta.url);
const Module = require('node:module');
const { Engine, createSwissShapedFacade } = require('../index.js');
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const signedCircular = (a, b) => (((a - b) % 360) + 540) % 360 - 180;
const circular = (a, b) => Math.abs(signedCircular(a, b));

function main() {
  const [candidatePath, qualityPath, hip2Path, de441Path, swissPath, backendPath, ...extra] = process.argv.slice(2);
  if (!candidatePath || !qualityPath || !hip2Path || !de441Path || !swissPath || !backendPath
      || (extra.length && (extra.length !== 1 || extra[0] !== '--seconds'))) {
    throw new Error('usage: node audit-star-backend.mjs <candidates.json> <quality.json> <hip2.dat> <de441.eph> <local-swisseph> <backend-func.js> [--seconds]');
  }
  const checkSeconds = extra[0] === '--seconds';
  const candidateBytes = readFileSync(candidatePath);
  const qualityBytes = readFileSync(qualityPath);
  const candidates = JSON.parse(candidateBytes.toString('utf8'));
  const quality = JSON.parse(qualityBytes.toString('utf8'));
  if (candidates.schema !== 'virgostars-iau-candidates/1' || candidates.status !== 'REVIEW_REQUIRED'
    || quality.schema !== 'virgostars-hip2-quality-audit/1' || quality.status !== 'REVIEW_REQUIRED'
    || !Array.isArray(candidates.results) || !Array.isArray(quality.results)
    || quality.candidateReportSha256 !== hash(candidateBytes) || quality.hip2Sha256 !== hash(readFileSync(hip2Path))) {
    throw new Error('candidate, quality, or HIP2 provenance mismatch');
  }
  const strictRows = quality.results.filter((r) => r.strictEligible === true);
  const eligible = new Map(strictRows.map((r) => [r.requestedName, r.hip]));
  if (eligible.size !== strictRows.length) throw new Error('duplicate strict candidate names');
  const entries = candidates.results.flatMap((row) => {
    if (row.status !== 'CANDIDATE_REVIEW_REQUIRED' || row.candidates.length !== 1) return [];
    const c = row.candidates[0];
    if (eligible.get(row.requestedName) !== c.hip || c.magnitudeBand !== 'V' || !Number.isFinite(c.magnitude)) return [];
    return [{ name: row.requestedName, aliases: [], hip: c.hip, magnitude: c.magnitude, radialVelocityKmS: null }];
  });
  if (!entries.length || entries.length !== eligible.size || new Set(entries.map((e) => e.hip)).size !== entries.length) {
    throw new Error('strict candidates are missing, ambiguous or lack finite V-band magnitudes');
  }
  const engine = new Engine(resolve(de441Path));
  engine.loadHipparcos(resolve(hip2Path), entries.map((e) => e.hip), 'strict');
  const dir = dirname(resolve(de441Path));
  const facade = createSwissShapedFacade(engine, { starCatalog: entries, legacyEphePath: dir, provenance: true });
  const swiss = require(resolve(swissPath));
  swiss.swe_set_ephe_path(dir);
  swiss.swe_set_jpl_file(basename(de441Path));
  const originalLoad = Module._load;
  let backend;
  try {
    Module._load = function (request, parent, isMain) {
      return request === 'swisseph' ? facade : originalLoad.call(this, request, parent, isMain);
    };
    const module = require(resolve(backendPath));
    backend = typeof module.createAstrologyFunctions === 'function'
      ? module.createAstrologyFunctions(facade) : module;
  } finally {
    Module._load = originalLoad;
  }
  process.env.ASTRODIENST_EPHE_PATH = dir;
  const dates = checkSeconds ? [
    ['1950-05-15 12:34:59', 1950, 5, 15, 12 + 34 / 60 + 59 / 3600],
    ['2000-01-01 00:00:01', 2000, 1, 1, 1 / 3600],
    ['2024-03-20 23:59:58', 2024, 3, 20, 23 + 59 / 60 + 58 / 3600],
  ] : [
    ['1950-05-15 12:00:00', 1950, 5, 15],
    ['2000-01-01 12:00:00', 2000, 1, 1],
    ['2024-03-20 12:00:00', 2024, 3, 20],
  ];
  const stats = { selectedCandidates: entries.length, requestedEpochs: dates.length, comparisons: 0,
    engineErrors: 0, referenceErrors: 0, backendErrors: 0, backendComparisons: 0, backendNameDifferences: 0,
    maxLongitudeArcsec: 0, maxLatitudeArcsec: 0, maxLongitudeRateArcsecPerDay: 0,
    maxLatitudeRateArcsecPerDay: 0, magnitudeComparisons: 0, magnitudeReferenceErrors: 0, maxMagnitudeDifference: 0,
    maxOurLongitudeRateVsReferencePositions: 0, maxOurLatitudeRateVsReferencePositions: 0,
    maxReferenceLongitudeRateVsReferencePositions: 0, maxReferenceLatitudeRateVsReferencePositions: 0 };
  const sites = checkSeconds ? [
    { longitude: 12.5, latitude: 41.9, inversion: false },
    { longitude: 144.96, latitude: -37.81, inversion: true },
    { longitude: 18.96, latitude: 69.65, inversion: false },
  ] : [{ longitude: 12.5, latitude: 41.9, inversion: false }];
  if (checkSeconds) stats.backendEpochCheck = { sites: sites.length, toleranceDegrees: 1e-10,
    mismatches: 0, maxPositionDifferenceDegrees: 0, maxHorizonDifferenceDegrees: 0 };
  for (const [date, year, month, day, hour = 12] of dates) {
    const jd = facade.swe_julday(year, month, day, hour, facade.SE_GREG_CAL);
    for (const entry of entries) {
      const ours = facade.swe_fixstar_ut(entry.name, jd, facade.SEFLG_SPEED);
      if (ours.error) { stats.engineErrors++; continue; }
      const ref = swiss.swe_fixstar_ut(entry.name, jd, swiss.SEFLG_JPLEPH | swiss.SEFLG_SPEED);
      if (ref.error) { stats.referenceErrors++; continue; }
      if (!(ref.rflag & swiss.SEFLG_JPLEPH)) throw new Error('reference did not use the explicitly requested JPL data');
      const fields = ['longitude', 'latitude', 'longitudeSpeed', 'latitudeSpeed'];
      if (!fields.every((field) => Number.isFinite(ours[field]) && Number.isFinite(ref[field]))) throw new Error('nonfinite reference or engine field');
      stats.comparisons++;
      stats.maxLongitudeArcsec = Math.max(stats.maxLongitudeArcsec, circular(ours.longitude, ref.longitude) * 3600);
      stats.maxLatitudeArcsec = Math.max(stats.maxLatitudeArcsec, Math.abs(ours.latitude - ref.latitude) * 3600);
      stats.maxLongitudeRateArcsecPerDay = Math.max(stats.maxLongitudeRateArcsecPerDay, Math.abs(ours.longitudeSpeed - ref.longitudeSpeed) * 3600);
      stats.maxLatitudeRateArcsecPerDay = Math.max(stats.maxLatitudeRateArcsecPerDay, Math.abs(ours.latitudeSpeed - ref.latitudeSpeed) * 3600);
      const plusJd = jd + 0.01;
      const minusJd = jd - 0.01;
      const plus = swiss.swe_fixstar_ut(entry.name, plusJd, swiss.SEFLG_JPLEPH);
      const minus = swiss.swe_fixstar_ut(entry.name, minusJd, swiss.SEFLG_JPLEPH);
      if (plus.error || minus.error || !(plus.rflag & swiss.SEFLG_JPLEPH) || !(minus.rflag & swiss.SEFLG_JPLEPH)
        || ![plus.longitude, minus.longitude, plus.latitude, minus.latitude].every(Number.isFinite)) {
        throw new Error('reference position derivative unavailable');
      }
      const referenceLonRate = signedCircular(plus.longitude, minus.longitude) / (plusJd - minusJd);
      const referenceLatRate = (plus.latitude - minus.latitude) / (plusJd - minusJd);
      stats.maxOurLongitudeRateVsReferencePositions = Math.max(stats.maxOurLongitudeRateVsReferencePositions, Math.abs(ours.longitudeSpeed - referenceLonRate) * 3600);
      stats.maxOurLatitudeRateVsReferencePositions = Math.max(stats.maxOurLatitudeRateVsReferencePositions, Math.abs(ours.latitudeSpeed - referenceLatRate) * 3600);
      stats.maxReferenceLongitudeRateVsReferencePositions = Math.max(stats.maxReferenceLongitudeRateVsReferencePositions, Math.abs(ref.longitudeSpeed - referenceLonRate) * 3600);
      stats.maxReferenceLatitudeRateVsReferencePositions = Math.max(stats.maxReferenceLatitudeRateVsReferencePositions, Math.abs(ref.latitudeSpeed - referenceLatRate) * 3600);
      const magnitude = swiss.swe_fixstar_mag(entry.name);
      if (Number.isFinite(magnitude.magnitude)) {
        stats.magnitudeComparisons++;
        stats.maxMagnitudeDifference = Math.max(stats.maxMagnitudeDifference, Math.abs(entry.magnitude - magnitude.magnitude));
      } else stats.magnitudeReferenceErrors++;
      for (const site of sites) {
        const originalLog = console.log;
        let star;
        try {
          console.log = () => {};
          star = backend.getStella(date, site.longitude, site.latitude, entry.name, site.inversion);
        } catch {
          stats.backendErrors++;
          continue;
        } finally {
          console.log = originalLog;
        }
        if (!star || !Number.isFinite(star.latlng?.longitude) || !Number.isFinite(star.azalt?.azimuth) || star.magnitude !== entry.magnitude) stats.backendErrors++;
        else {
          stats.backendComparisons++;
          if (star.name !== ref.name) stats.backendNameDifferences++;
          if (checkSeconds) {
            const expectedLongitude = site.inversion && site.latitude < 0 ? (ours.longitude + 180) % 360 : ours.longitude;
            const horizon = facade.swe_azalt(jd, facade.SE_ECL2HOR, site.longitude, site.latitude,
              0, 0, 0, ours.longitude, ours.latitude, 0);
            if (![star.latlng.latitude, star.azalt.trueAltitude, horizon.azimuth, horizon.trueAltitude].every(Number.isFinite)) {
              stats.backendErrors++;
              continue;
            }
            const positionDiff = Math.max(circular(star.latlng.longitude, expectedLongitude), Math.abs(star.latlng.latitude - ours.latitude));
            const horizonDiff = Math.max(circular(star.azalt.azimuth, (horizon.azimuth + 180) % 360), Math.abs(star.azalt.trueAltitude - horizon.trueAltitude));
            stats.backendEpochCheck.maxPositionDifferenceDegrees = Math.max(stats.backendEpochCheck.maxPositionDifferenceDegrees, positionDiff);
            stats.backendEpochCheck.maxHorizonDifferenceDegrees = Math.max(stats.backendEpochCheck.maxHorizonDifferenceDegrees, horizonDiff);
            if (positionDiff > stats.backendEpochCheck.toleranceDegrees || horizonDiff > stats.backendEpochCheck.toleranceDegrees) stats.backendEpochCheck.mismatches++;
          }
        }
      }
    }
  }
  if (hash(readFileSync(candidatePath)) !== hash(candidateBytes) || hash(readFileSync(qualityPath)) !== hash(qualityBytes)
    || hash(readFileSync(hip2Path)) !== quality.hip2Sha256) throw new Error('candidate, quality, or HIP2 input changed during audit');
  console.log(JSON.stringify({ status: 'DIAGNOSTIC_ONLY_NOT_APPROVED', timePolicy: 'engine default UT policy',
    note: 'Distinct star catalogs and assumed zero radial velocities; maxima are not accuracy claims.', ...stats }, null, 2));
  if (stats.engineErrors || stats.referenceErrors || stats.backendErrors || stats.magnitudeReferenceErrors
    || stats.comparisons !== entries.length * dates.length || stats.backendComparisons !== stats.comparisons * sites.length
    || (checkSeconds && stats.backendEpochCheck.mismatches)
    || stats.magnitudeComparisons !== stats.comparisons) process.exitCode = 1;
}

try { main(); } catch (error) { console.error(`Star backend audit failed: ${error.message}`); process.exitCode = 1; }
