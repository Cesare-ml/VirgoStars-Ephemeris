// Opt-in local integration: run the unchanged backend module against the facade.
// No database, server, personal data or third-party catalog is used.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';

const require = createRequire(import.meta.url);
const Module = require('node:module');
const { Engine, createSwissShapedFacade, prepareStarCatalog } = require('../index.js');
const { DE441_PATH: de441, HIP2_PATH: hip2, CHIRON_SPK_PATH: chiron,
  ASTROLOGICAL_FUNCTIONS_SRC: afSrc, BACKEND_ASTROLOGY_FUNC: backendPath } = process.env;
const needs = { skip: de441 && hip2 && chiron && afSrc && backendPath ? false : 'local kernel, catalog, Chiron, house methods and backend module required' };

test('unchanged backend builds a fictional chart and star via the explicit adapter', needs, async () => {
  const a = await import(pathToFileURL(join(afSrc, 'house-set-with-warnings.mjs')).href);
  const b = await import(pathToFileURL(join(afSrc, 'placidus-position.mjs')).href);
  const engine = new Engine(de441);
  engine.loadChiron(chiron);
  const catalog = prepareStarCatalog({
    schema: 'virgostars-star-manifest/1', reviewer: 'test',
    astrometrySource: 'Caller-provided HIP2 file', dataUseDecision: 'Local test only', qualityPolicy: 'strict',
    // A deliberately fictional name and magnitude; this is not a catalog source.
    stars: [{ name: 'Test star', displayName: 'Test star,example', displayNameSource: 'Fictional backend convention', aliases: [], hip: 49669, radialVelocityKmS: null, magnitude: 1.23,
      identitySource: 'Fictional test mapping', qualityDecision: 'Test of strict policy',
      magnitudeSource: 'Fictional test magnitude', radialVelocityMissingReason: 'Test assumption' }],
  });
  engine.loadHipparcos(hip2, catalog.hips, catalog.qualityPolicy);
  const directory = dirname(de441);
  const facade = createSwissShapedFacade(engine, {
    acceptSwiephFlag: true, legacyEphePath: directory,
    houseMethods: { calculateHouseSetWithWarnings: a.calculateHouseSetWithWarnings, calculatePlacidusHousePosition: b.calculatePlacidusHousePosition },
    starCatalog: catalog.stars,
  });
  assert.throws(() => facade.swe_set_ephe_path(join(directory, 'other')), (e) => e.code === 'E_EPHEMERIS_FILE');
  process.env.ASTRODIENST_EPHE_PATH = directory;
  const originalLoad = Module._load;
  let backend;
  try {
    Module._load = function (request, parent, isMain) {
      return request === 'swisseph' ? facade : originalLoad.call(this, request, parent, isMain);
    };
    backend = require(backendPath);
  } finally {
    Module._load = originalLoad;
  }
  const chart = backend.getPianetiCase('2024-03-20 12:00:00', 12.5, 41.9);
  for (const name of ['Sun', 'Moon', 'Mercury', 'Venus', 'Mars', 'Jupiter', 'Saturn', 'Uranus', 'Neptune', 'Pluto', 'mean Node', 'true Node', 'mean Apogee', 'osc. Apogee', 'Chiron']) {
    assert.ok(Number.isFinite(chart.planets[name]?.longitude), `${name} longitude`);
    assert.ok(Number.isFinite(chart.planets[name]?.longitudeSpeed), `${name} speed`);
  }
  for (const name of ['Sun', 'Moon', 'Mercury', 'Venus', 'Mars', 'Jupiter', 'Saturn', 'Uranus', 'Neptune', 'Pluto', 'mean Node', 'true Node', 'mean Apogee', 'osc. Apogee', 'Chiron']) {
    assert.ok(Number.isFinite(chart.planets[name].eliocentrico.longitude), `${name} heliocentric longitude`);
    assert.ok(Number.isFinite(chart.planets[name].equatoriale.rectAscension), `${name} right ascension`);
  }
  for (const name of ['egizia', 'placido', 'koch', 'regio-montano', 'porfirio']) {
    assert.equal(chart.houses[name].house.length, 12, name);
    assert.ok(chart.houses[name].house.every(Number.isFinite), name);
  }
  const star = backend.getStella('2024-03-20 12:00:00', 12.5, 41.9, 'Test star');
  assert.equal(star.name, 'Test star,example');
  assert.equal(star.magnitude, 1.23);
  assert.ok(Number.isFinite(star.latlng.longitude));
  assert.ok(Number.isFinite(star.azalt.azimuth));
});
