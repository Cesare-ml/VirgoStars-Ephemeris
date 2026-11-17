// Measured DeltaT in calcUt. Needs DE441_PATH; the genuine-file test also needs USNO_DELTAT_DIR
// (directory holding deltat.data, historic_deltat.data and deltat.preds).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const require = createRequire(import.meta.url);
const { Engine, constants } = require('../index.js');
const de441 = process.env.DE441_PATH;
const usno = process.env.USNO_DELTAT_DIR;
const SPEED = constants.SEFLG_SPEED;
const needKernel = { skip: de441 ? false : 'DE441_PATH is not set' };
const needFiles = { skip: de441 && usno ? false : 'DE441_PATH and USNO_DELTAT_DIR are required' };

const HISTORIC = `  Year    TDT-UT1    error     LOD      error
1950.000     29.15    0.10     1.00     1.00
1960.000     33.15    0.02     1.00     1.00
1970.000     40.18    0.001    1.00     1.00
1972.000     42.23    0.001    1.00     1.00
1972.500     42.81    0.001    1.00     1.00
1973.000     43.37    0.001    1.00     1.00
`;
const MONTHLY = ` 1973  2  1  43.4724
 1973  3  1  43.5648
 1973  4  1  43.6737
 1973  5  1  43.7782
 1973  6  1  43.8763
 1973  7  1  43.9562
 1973  8  1  44.0315
`;

function withFiles(fn) {
  const dir = mkdtempSync(join(tmpdir(), 'dt-'));
  try {
    writeFileSync(join(dir, 'deltat.data'), MONTHLY);
    writeFileSync(join(dir, 'historic_deltat.data'), HISTORIC);
    fn(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test('without a table the polynomial/leap-second policy is used and says so', needKernel, () => {
  const e = new Engine(de441);
  assert.match(e.calcUt(2433283.0, 0, SPEED).timeSource, /^DELTA_T_POLYNOMIAL:/);
});

test('with a table calcUt reports DELTA_T_TABLE and uses its value', needKernel, () => {
  withFiles((dir) => {
    const e = new Engine(de441);
    e.loadDeltaT(join(dir, 'deltat.data'), join(dir, 'historic_deltat.data'));
    const r = e.calcUt(2441714.5, 0, SPEED); // 1973-02-01 0h UT
    assert.equal(r.timeSource, 'DELTA_T_TABLE:MEASURED:43.4724:unknown');
    // 1950-01-01: historic row, with its uncertainty.
    assert.match(e.calcUt(2433282.5, 0, SPEED).timeSource, /^DELTA_T_TABLE:MEASURED:29\.15:0\.1$/);
    // Outside the table: an explicit range error, no extrapolation, no silent fallback.
    for (const jd of [2441896, 2433000]) {
      assert.throws(() => e.calcUt(jd, 0, SPEED), (x) => x.code === 'E_RANGE', String(jd));
    }
  });
});

test('a failed load keeps the previous table; missing and malformed files are E_DELTAT_FILE', needKernel, () => {
  withFiles((dir) => {
    const e = new Engine(de441);
    e.loadDeltaT(join(dir, 'deltat.data'), join(dir, 'historic_deltat.data'));
    assert.throws(() => e.loadDeltaT(join(dir, 'missing.data'), join(dir, 'historic_deltat.data')), (x) => x.code === 'E_DELTAT_FILE');
    writeFileSync(join(dir, 'bad.data'), ' 1973  2  1  not-a-number\n');
    assert.throws(() => e.loadDeltaT(join(dir, 'bad.data'), join(dir, 'historic_deltat.data')), (x) => x.code === 'E_DELTAT_FILE');
    assert.equal(e.calcUt(2441714.5, 0, SPEED).timeSource, 'DELTA_T_TABLE:MEASURED:43.4724:unknown');
  });
});

test('two engines keep independent tables', needKernel, () => {
  withFiles((dir) => {
    const a = new Engine(de441);
    const b = new Engine(de441);
    a.loadDeltaT(join(dir, 'deltat.data'), join(dir, 'historic_deltat.data'));
    assert.match(a.calcUt(2441714.5, 0, SPEED).timeSource, /^DELTA_T_TABLE/);
    assert.match(b.calcUt(2441714.5, 0, SPEED).timeSource, /^IERS_LEAP_SECONDS$/);
  });
});

test('genuine USNO files: measured to the last observation, predictions flagged after it', needFiles, () => {
  const e = new Engine(de441);
  e.loadDeltaT(join(usno, 'deltat.data'), join(usno, 'historic_deltat.data'), join(usno, 'deltat.preds'));
  const j2000 = e.calcUt(2451545.0, 0, SPEED);
  assert.match(j2000.timeSource, /^DELTA_T_TABLE:MEASURED:63\.8\d+:/);
  const recent = e.calcUt(2461000.5, 0, SPEED); // 2026
  assert.match(recent.timeSource, /^DELTA_T_TABLE:MEASURED:69\.\d+:/);
  const future = e.calcUt(2461400.5, 0, SPEED);
  assert.match(future.timeSource, /^DELTA_T_TABLE:PREDICTED:69\.\d+:0\.\d+$/);
  // The policy path stops at 2027-06-28; the table path goes on, flagged.
  assert.throws(() => new Engine(de441).calcUt(2461600, 0, SPEED), (x) => x.code === 'E_RANGE');
});
