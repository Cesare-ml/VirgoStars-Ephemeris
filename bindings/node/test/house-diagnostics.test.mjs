import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { createHouseDiagnosticSession } = require('../index.js');

function fakeFacade() {
  return Object.freeze({
    SE_SUN: 0,
    swe_houses(_jd, _lat, _lon, system) {
      const result = { house: [1, 2] };
      Object.defineProperties(result, {
        status: { value: system === 'P' || system === 'K' ? 'FALLBACK_PORPHYRY' : 'OK' },
        warnings: { value: [{ code: 'POLAR_CIRCLE_LATITUDE', secret: 'not copied' }] },
      });
      return result;
    },
    swe_houses_pos(_armc, _lat, _eps, _system, lon) {
      if (lon === 1) return { housePosition: 1 };
      const result = { error: 'private message' };
      Object.defineProperty(result, 'code', { value: 'E_POSITION_UNAVAILABLE' });
      return result;
    },
  });
}

test('diagnostics survive JSON and aggregate polar fallback and unavailable positions', () => {
  const session = createHouseDiagnosticSession(fakeFacade());
  const { result, diagnostics } = session.run(() => {
    const p = session.facade.swe_houses(2451545, 69.65, 18.96, 'P');
    const k = session.facade.swe_houses(2451545, 69.65, 18.96, 'K');
    session.facade.swe_houses_pos(100, 69.65, 23, 'P', 2, 0);
    session.facade.swe_houses_pos(100, 69.65, 23, 'P', 3, 0);
    return { p, k };
  });
  assert.deepEqual(Object.keys(result.p), ['house']);
  assert.equal(session.facade.SE_SUN, 0);
  assert.deepEqual(diagnostics.events, [
    { operation: 'swe_houses', code: 'FALLBACK_PORPHYRY', system: 'P', count: 1 },
    { operation: 'swe_houses', code: 'POLAR_CIRCLE_LATITUDE', system: 'ALL', count: 1 },
    { operation: 'swe_houses', code: 'FALLBACK_PORPHYRY', system: 'K', count: 1 },
    { operation: 'swe_houses_pos', code: 'E_POSITION_UNAVAILABLE', system: 'P', count: 2 },
  ]);
  assert.deepEqual(JSON.parse(JSON.stringify({ result, diagnostics })).diagnostics, diagnostics);
  assert.doesNotMatch(JSON.stringify(diagnostics), /private|secret|2451545|69\.65|18\.96/);
});

test('sessions reset after success or failure and reject asynchronous and nested use', () => {
  const session = createHouseDiagnosticSession(fakeFacade());
  assert.throws(() => session.run(() => { session.facade.swe_houses(1, 2, 3, 'P'); throw new Error('failure'); }), /failure/);
  assert.deepEqual(session.run(() => 3).diagnostics.events, []);
  assert.throws(() => session.run(() => session.run(() => 1)), (e) => e.code === 'E_DIAGNOSTIC_STATE');
  assert.deepEqual(session.run(() => 4).diagnostics.events, []);
  assert.throws(() => session.run(async () => 1), (e) => e.code === 'E_DIAGNOSTIC_STATE');
  assert.throws(() => session.run(() => Promise.resolve(1)), (e) => e.code === 'E_DIAGNOSTIC_STATE');
  assert.throws(() => createHouseDiagnosticSession({}), (e) => e.code === 'E_DIAGNOSTIC_STATE');
});

test('unknown external error codes and system strings are not copied into diagnostics', () => {
  const facade = { swe_houses: () => ({ error: 'private', code: 'private code' }), swe_houses_pos: () => ({ housePosition: 1 }) };
  const session = createHouseDiagnosticSession(facade);
  const { diagnostics } = session.run(() => session.facade.swe_houses(1, 2, 3, 'private system'));
  assert.deepEqual(diagnostics.events, [{ operation: 'swe_houses', code: 'E_OTHER', system: 'OTHER', count: 1 }]);
});
