// Star tests need DE441_PATH and a caller-supplied genuine hip2.dat (HIP2_PATH).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { Engine, constants } = require('../index.js');
const de441 = process.env.DE441_PATH;
const hip2 = process.env.HIP2_PATH;
const opts = { skip: de441 && hip2 ? false : 'DE441_PATH and HIP2_PATH are required' };

test('traditional stars load under their policy; strict rejects a stochastic solution', opts, () => {
  const e = new Engine(de441);
  assert.equal(e.loadHipparcos(hip2, [49669, 21421], 'traditional'), 2);
  assert.throws(() => e.loadHipparcos(hip2, [21421], 'strict'), (x) => x.code === 'E_STAR_QUALITY');
  assert.equal(e.fixstarTdb(49669, 5.9, 2451545.0, constants.SEFLG_SPEED).xx[0] > 149.8, true);
  assert.throws(() => e.loadHipparcos(hip2, [49669], 'bogus'), (x) => x.code === 'E_RANGE');
  assert.throws(() => e.fixstarTdb(1, null, 2451545.0, 0), (x) => x.code === 'E_STAR_NOT_LOADED');
});

test('a failed load keeps the previous selection, and a missing radial velocity is reported', opts, () => {
  const e = new Engine(de441);
  e.loadHipparcos(hip2, [49669], 'strict');
  assert.throws(() => e.loadHipparcos(hip2, [21421], 'strict'));
  const r = e.fixstarTdb(49669, null, 2451545.0, constants.SEFLG_SPEED);
  assert.ok(r.notes.some((n) => n.includes('assumed to be 0 km/s')));
  assert.ok(Math.abs(r.xx[0] - 149.829) < 0.001);
});
