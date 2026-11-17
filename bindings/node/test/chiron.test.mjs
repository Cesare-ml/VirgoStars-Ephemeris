// Chiron from a caller-supplied JPL small-body SPK. Needs DE441_PATH; the genuine-file tests also
// need CHIRON_SPK_PATH (JPL SPK of 2060 Chiron, type 21); the shape test also needs SWISSEPH_PATH.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, basename } from 'node:path';

const require = createRequire(import.meta.url);
const { Engine, constants, createSwissShapedFacade } = require('../index.js');
const de441 = process.env.DE441_PATH;
const spk = process.env.CHIRON_SPK_PATH;
const swissPath = process.env.SWISSEPH_PATH;
const SPEED = constants.SEFLG_SPEED;
const needKernel = { skip: de441 ? false : 'DE441_PATH is not set' };
const needSpk = { skip: de441 && spk ? false : 'DE441_PATH and CHIRON_SPK_PATH are required' };
const needSwiss = { skip: de441 && spk && swissPath ? false : 'DE441_PATH, CHIRON_SPK_PATH and SWISSEPH_PATH are required' };
const wrap = (d) => ((((d + 180) % 360) + 360) % 360) - 180;

test('without a small-body file Chiron is refused and says how to enable it', needKernel, () => {
  const e = new Engine(de441);
  assert.throws(() => e.calcTdb(2451545.0, constants.SE_CHIRON, SPEED), (x) => x.code === 'E_UNSUPPORTED_BODY' && /loadChiron/.test(x.message));
});

test('a missing, foreign or wrong file is E_SPK_FILE and keeps the previous state', needKernel, () => {
  const e = new Engine(de441);
  assert.throws(() => e.loadChiron('D:/definitely/missing.bsp'), (x) => x.code === 'E_SPK_FILE');
  assert.throws(() => e.loadChiron(de441), (x) => x.code === 'E_SPK_FILE', 'a DE441 binary is not an SPK');
  assert.throws(() => e.calcTdb(2451545.0, constants.SE_CHIRON, SPEED), (x) => x.code === 'E_UNSUPPORTED_BODY');
});

test('Chiron at J2000 is plausible and works in every frame and mode', needSpk, () => {
  const e = new Engine(de441);
  e.loadChiron(spk);
  const geo = e.calcTdb(2451545.0, constants.SE_CHIRON, SPEED);
  assert.ok(Math.abs(geo.xx[0] - 251.6175) < 0.01 && Math.abs(geo.xx[1] - 4.0717) < 0.01, JSON.stringify(geo.xx));
  assert.ok(geo.xx[2] > 10 && geo.xx[2] < 11);
  assert.equal(geo.rflag, 1 | 256);
  assert.ok(geo.notes.some((n) => /small-body SPK/.test(n)));
  const equ = e.calcTdb(2451545.0, constants.SE_CHIRON, SPEED | constants.SEFLG_EQUATORIAL);
  assert.ok(Math.abs(equ.xx[0] - 249.9) < 5 && equ.rflag === (1 | 256 | 2048));
  const helio = e.calcTdb(2451545.0, constants.SE_CHIRON, SPEED | constants.SEFLG_HELCTR);
  assert.ok(helio.xx[2] > 8 && helio.xx[2] < 19.5 && helio.rflag === (1 | 8 | 256 | 512 | 1024));
  // UT entry works as for any body; outside the SPK coverage is an explicit error.
  assert.ok(Number.isFinite(e.calcUt(2451545.0, constants.SE_CHIRON, SPEED).xx[0]));
  assert.throws(() => e.calcTdb(2400000.5, constants.SE_CHIRON, SPEED), (x) => x.code === 'E_CALC' || x.code === 'E_RANGE');
});

test('engines keep independent small-body files', needSpk, () => {
  const a = new Engine(de441);
  const b = new Engine(de441);
  a.loadChiron(spk);
  assert.ok(Number.isFinite(a.calcTdb(2451545.0, 15, SPEED).xx[0]));
  assert.throws(() => b.calcTdb(2451545.0, 15, SPEED), (x) => x.code === 'E_UNSUPPORTED_BODY');
});

test('facade: Chiron has the reference name and keys, and agrees within the orbit-fit difference', needSwiss, () => {
  const swiss = require(swissPath);
  swiss.swe_set_ephe_path(dirname(de441));
  swiss.swe_set_jpl_file(basename(de441));
  const e = new Engine(de441);
  e.loadChiron(spk);
  const ours = createSwissShapedFacade(e);
  assert.equal(ours.swe_get_planet_name(15).name, swiss.swe_get_planet_name(15).name);
  for (const jd of [2415100.5, 2440000.5, 2451545.0, 2460000.5, 2480000.5]) {
    for (const extra of [0, 8, 2048]) {
      const flags = 1 | 256 | extra;
      const a = swiss.swe_calc(jd, 15, flags);
      const b = ours.swe_calc(jd, 15, flags);
      assert.deepEqual(Object.keys(b), Object.keys(a), `keys ${jd} ${extra}`);
      assert.equal(b.rflag, a.rflag, `rflag ${jd} ${extra}`);
      const [lon, lat] = extra === 2048 ? ['rectAscension', 'declination'] : ['longitude', 'latitude'];
      // The reference fits an older orbit solution: up to ~1.7 arcsec at the ends of 1900-2100.
      assert.ok(Math.abs(wrap(b[lon] - a[lon])) * 3600 < 2.0, `${lon} ${jd} ${extra}`);
      assert.ok(Math.abs(b[lat] - a[lat]) * 3600 < 2.0, `${lat} ${jd} ${extra}`);
      assert.ok(Math.abs(b.distance / a.distance - 1) < 1e-5, `distance ${jd} ${extra}`);
    }
  }
});
