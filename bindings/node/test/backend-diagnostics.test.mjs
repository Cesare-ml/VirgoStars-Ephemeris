// Opt-in: unchanged local backend, caller-supplied kernel and house methods.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';

const require = createRequire(import.meta.url);
const Module = require('node:module');
const { Engine, createSwissShapedFacade, createHouseDiagnosticSession } = require('../index.js');
const { DE441_PATH: de441, CHIRON_SPK_PATH: chiron, ASTROLOGICAL_FUNCTIONS_SRC: afSrc,
  BACKEND_ASTROLOGY_FUNC: backendPath } = process.env;
const needs = { skip: de441 && chiron && afSrc && backendPath ? false : 'local kernel, Chiron, house methods and backend module required' };

test('actual backend returns serializable polar diagnostics without changing legacy chart keys', needs, async () => {
  const a = await import(pathToFileURL(join(afSrc, 'house-set-with-warnings.mjs')).href);
  const b = await import(pathToFileURL(join(afSrc, 'placidus-position.mjs')).href);
  const engine = new Engine(de441);
  engine.loadChiron(chiron);
  const dir = dirname(de441);
  const facade = createSwissShapedFacade(engine, { acceptSwiephFlag: true, legacyEphePath: dir,
    houseMethods: { calculateHouseSetWithWarnings: a.calculateHouseSetWithWarnings, calculatePlacidusHousePosition: b.calculatePlacidusHousePosition } });
  const session = createHouseDiagnosticSession(facade);
  process.env.ASTRODIENST_EPHE_PATH = dir;
  const oldLoad = Module._load;
  let backend;
  try {
    Module._load = function (request, parent, isMain) {
      return request === 'swisseph' ? session.facade : oldLoad.call(this, request, parent, isMain);
    };
    backend = require(backendPath);
  } finally { Module._load = oldLoad; }

  const normal = session.run(() => backend.getPianetiCase('2024-03-20 12:00:00', 12.5, 41.9));
  assert.deepEqual(normal.diagnostics.events, []);
  const polar = session.run(() => backend.getPianetiCase('1950-05-15 12:00:00', 18.96, 69.65));
  const events = polar.diagnostics.events;
  assert.ok(events.some((e) => e.code === 'FALLBACK_PORPHYRY' && e.system === 'P'));
  assert.ok(events.some((e) => e.code === 'FALLBACK_PORPHYRY' && e.system === 'K'));
  assert.ok(events.some((e) => e.code === 'E_POSITION_UNAVAILABLE' && e.count > 0));
  const wire = JSON.parse(JSON.stringify(polar));
  assert.deepEqual(wire.diagnostics, polar.diagnostics);
  assert.equal(Object.hasOwn(wire.result.houses.placido, 'warnings'), false);
  assert.doesNotMatch(JSON.stringify(wire.diagnostics), /1950|69\.65|18\.96/);
});
