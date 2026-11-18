// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Model implementation comparison only, not independent physical validation.
// Usage: node scripts/check-analytic-deflection-against-prototype.mjs KERNEL GRID_JSON EXAMPLE_EXE
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import assert from 'node:assert/strict';
const { Engine } = createRequire(import.meta.url)('../index.js');
const [kernel, gridPath, executable] = process.argv.slice(2);
if (!kernel || !gridPath || !executable) throw new Error('expected KERNEL GRID_JSON EXAMPLE_EXE');
const rows = JSON.parse(readFileSync(gridPath, 'utf8')).rows;
assert.ok(Array.isArray(rows) && rows.length > 0);
const input = rows.map(r => `${r.id},${r.jd}`).join('\n') + '\n';
const run = spawnSync(executable, [kernel, '--apparent-state'], { input, encoding: 'utf8', maxBuffer: 10_000_000 });
if (run.error) throw run.error;
if (run.status !== 0) throw new Error(run.stderr);
const lines = run.stdout.trim().split(/\r?\n/).slice(1);
assert.equal(lines.length, rows.length);
const engine = new Engine(kernel), as = 206264.80624709636;
let maxPosition = 0, maxRate = 0;
for (const [i, line] of lines.entries()) {
  const v = line.split(',').map(Number), row = rows[i];
  assert.equal(v.length, 10); assert.ok(v.every(Number.isFinite));
  assert.equal(v[0], row.id); assert.equal(v[1], row.jd);
  const state = engine.experimentalAnalyticDeflectionIcrfTdb(row.jd, row.id);
  const p = state.positionKm, vel = state.velocityKmPerDay, r = Math.hypot(...p);
  const u = p.map(x => x / r), radial = u.reduce((sum, x, j) => sum + x * vel[j], 0);
  const d = u.map((x, j) => (vel[j] - x * radial) / r), rho = Math.hypot(u[0], u[1]);
  const ra = Math.atan2(u[1], u[0]) * 180 / Math.PI, dec = Math.asin(u[2]) * 180 / Math.PI;
  const dra = (u[0]*d[1] - u[1]*d[0]) / rho * as, ddec = d[2] / rho * as;
  const deltaRa = ((ra - v[4] + 540) % 360) - 180;
  maxPosition = Math.max(maxPosition, Math.hypot(deltaRa * Math.cos(dec * Math.PI / 180), dec - v[5]) * 3600);
  maxRate = Math.max(maxRate, Math.hypot(dra - v[8], ddec - v[9]));
}
assert.ok(maxPosition < 1e-6, `position difference ${maxPosition} arcsec`);
assert.ok(maxRate < 1e-6, `rate difference ${maxRate} arcsec/day`);
console.log(JSON.stringify({ cases: rows.length, maxPositionArcsec: maxPosition, maxRateArcsecPerDay: maxRate, interpretation: 'same-model prototype comparison; not physical validation' }, null, 2));
