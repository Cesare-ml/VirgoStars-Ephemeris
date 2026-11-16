// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Offline, opt-in diagnostic: synthetic dates/sites, unchanged backend, local references.
// Prints aggregates only; no application records or raw reference rows are retained.
import { createRequire } from 'node:module';
import { basename, dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const Module = require('node:module');
const { Engine, createSwissShapedFacade } = require('../index.js');
const angle = (a, b) => Math.abs((((a - b) % 360) + 540) % 360 - 180);

function loadBackend(path, swisseph) {
  const target = require.resolve(resolve(path));
  delete require.cache[target];
  const previous = Module._load;
  try {
    Module._load = function (name, parent, isMain) {
      return name === 'swisseph' ? swisseph : previous.call(this, name, parent, isMain);
    };
    const module = require(target);
    return typeof module.createAstrologyFunctions === 'function'
      ? module.createAstrologyFunctions(swisseph) : module;
  } finally {
    Module._load = previous;
    delete require.cache[target];
  }
}

function observe(bucket, label, ours, reference, isAngle, context) {
  const record = bucket[label] ??= { compared: 0, missing: 0, missingEngine: 0, missingReference: 0,
    firstMissingCase: null, maxAbsoluteDifference: null, worstCase: null };
  if (!Number.isFinite(ours) || !Number.isFinite(reference)) {
    record.missing++;
    if (!Number.isFinite(ours)) record.missingEngine++;
    if (!Number.isFinite(reference)) record.missingReference++;
    if (record.firstMissingCase === null) record.firstMissingCase = context;
    return;
  }
  const difference = isAngle ? angle(ours, reference) : Math.abs(ours - reference);
  record.compared++;
  if (record.maxAbsoluteDifference === null || difference > record.maxAbsoluteDifference) {
    record.maxAbsoluteDifference = difference;
    record.worstCase = context;
  }
}

async function main() {
  const [de441Path, chironPath, houseSource, swissPath, backendPath, ...extra] = process.argv.slice(2);
  if (!de441Path || !chironPath || !houseSource || !swissPath || !backendPath || extra.length) {
    throw new Error('usage: node audit-chart-backend.mjs <de441.eph> <chiron.bsp> <house-src> <local-swisseph> <backend-func.js>');
  }
  const a = await import(pathToFileURL(join(resolve(houseSource), 'house-set-with-warnings.mjs')).href);
  const b = await import(pathToFileURL(join(resolve(houseSource), 'placidus-position.mjs')).href);
  const engine = new Engine(resolve(de441Path));
  engine.loadChiron(resolve(chironPath));
  const directory = dirname(resolve(de441Path));
  const facade = createSwissShapedFacade(engine, { acceptSwiephFlag: true, legacyEphePath: directory,
    houseMethods: { calculateHouseSetWithWarnings: a.calculateHouseSetWithWarnings, calculatePlacidusHousePosition: b.calculatePlacidusHousePosition } });
  const positionErrors = {};
  let activeCase = null;
  const observedFacade = { ...facade, swe_houses_pos(...args) {
    const result = facade.swe_houses_pos(...args);
    if (result.error) {
      const code = result.code ?? 'UNKNOWN';
      const entry = positionErrors[code] ??= { count: 0, firstCase: activeCase };
      entry.count++;
    }
    return result;
  } };
  const swiss = require(resolve(swissPath));
  swiss.swe_set_ephe_path(directory);
  swiss.swe_set_jpl_file(basename(de441Path));
  const sourceCounts = { swiss: 0, jpl: 0, moshier: 0, errors: 0, other: 0 };
  const swissAdapter = { ...swiss, swe_calc_ut(...args) {
    const result = swiss.swe_calc_ut(...args);
    if (((args[2] ?? 0) & (swiss.SEFLG_SPEED | swiss.SEFLG_SWIEPH)) === (swiss.SEFLG_SPEED | swiss.SEFLG_SWIEPH)) {
      if (result.error) sourceCounts.errors++;
      else if (result.rflag & swiss.SEFLG_SWIEPH) sourceCounts.swiss++;
      else if (result.rflag & swiss.SEFLG_JPLEPH) sourceCounts.jpl++;
      else if (result.rflag & swiss.SEFLG_MOSEPH) sourceCounts.moshier++;
      else sourceCounts.other++;
    }
    return result;
  } };
  const ours = loadBackend(backendPath, observedFacade);
  const reference = loadBackend(backendPath, swissAdapter);
  process.env.ASTRODIENST_EPHE_PATH = directory;
  const dates = ['1950-05-15 12:00:00', '2000-01-01 12:00:00', '2024-03-20 12:00:00'];
  const sites = [
    { name: 'Rome', longitude: 12.5, latitude: 41.9 },
    { name: 'Melbourne', longitude: 144.96, latitude: -37.81 },
    { name: 'Quito', longitude: -78.47, latitude: -0.18 },
    { name: 'Equator', longitude: 0, latitude: 0 },
    { name: 'Tromso', longitude: 18.96, latitude: 69.65 },
  ];
  const bodyNames = ['Sun', 'Moon', 'Mercury', 'Venus', 'Mars', 'Jupiter', 'Saturn', 'Uranus', 'Neptune', 'Pluto', 'mean Node', 'true Node', 'mean Apogee', 'osc. Apogee', 'Chiron'];
  const houseNames = ['egizia', 'placido', 'koch', 'regio-montano', 'porfirio'];
  const stats = { status: 'DIAGNOSTIC_ONLY_NOT_APPROVED', chartsAttempted: 0, chartErrors: 0,
    referencePrimarySourceCounts: sourceCounts, housePositionErrors: positionErrors, bodies: {}, houses: {} };
  for (const date of dates) for (const site of sites) {
    const context = { date, site: site.name };
    activeCase = context;
    stats.chartsAttempted++;
    let x; let y;
    const previousLog = console.log;
    try {
      console.log = () => {};
      x = ours.getPianetiCase(date, site.longitude, site.latitude, undefined, true, false);
      y = reference.getPianetiCase(date, site.longitude, site.latitude, undefined, true, false);
    } catch {
      stats.chartErrors++;
      continue;
    } finally {
      console.log = previousLog;
    }
    for (const name of bodyNames) {
      const xo = x.planets[name] ?? {};
      const yr = y.planets[name] ?? {};
      const bucket = stats.bodies[name] ??= {};
      observe(bucket, 'longitudeDegrees', xo.longitude, yr.longitude, true, context);
      observe(bucket, 'latitudeDegrees', xo.latitude, yr.latitude, false, context);
      observe(bucket, 'longitudeSpeedDegreesPerDay', xo.longitudeSpeed, yr.longitudeSpeed, false, context);
      observe(bucket, 'latitudeSpeedDegreesPerDay', xo.latitudeSpeed, yr.latitudeSpeed, false, context);
      observe(bucket, 'distanceAu', xo.distance, yr.distance, false, context);
      observe(bucket, 'heliocentricLongitudeDegrees', xo.eliocentrico?.longitude, yr.eliocentrico?.longitude, true, context);
      observe(bucket, 'rightAscensionDegrees', xo.equatoriale?.rectAscension, yr.equatoriale?.rectAscension, true, context);
      observe(bucket, 'azimuthDegrees', xo.azalt?.azimuth, yr.azalt?.azimuth, true, context);
      observe(bucket, 'trueAltitudeDegrees', xo.azalt?.trueAltitude, yr.azalt?.trueAltitude, false, context);
      observe(bucket, 'housePosition', xo.housePosLatLng, yr.housePosLatLng, false, context);
    }
    for (const name of houseNames) {
      const xo = x.houses[name] ?? {};
      const yr = y.houses[name] ?? {};
      const bucket = stats.houses[name] ??= { referenceErrors: 0, engineErrors: 0, metrics: {} };
      if (xo.error) bucket.engineErrors++;
      if (yr.error) bucket.referenceErrors++;
      for (const field of ['ascendant', 'mc', 'armc', 'vertex']) observe(bucket.metrics, `${field}Degrees`, xo[field], yr[field], true, context);
      for (let i = 0; i < 12; i++) observe(bucket.metrics, 'cuspDegrees', xo.house?.[i], yr.house?.[i], true, { ...context, cusp: i + 1 });
    }
  }
  console.log(JSON.stringify(stats, null, 2));
  if (stats.chartErrors || stats.chartsAttempted !== dates.length * sites.length) process.exitCode = 1;
}

try { await main(); } catch (error) { console.error(`Chart backend audit failed: ${error.message}`); process.exitCode = 1; }
