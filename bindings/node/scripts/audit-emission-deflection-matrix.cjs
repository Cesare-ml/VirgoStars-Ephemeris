// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// The fit references and all four paths share DE441. This audit does not
// establish independent planetary-ephemeris accuracy.
'use strict';
const fs = require('node:fs');
const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');
const assert = require('node:assert/strict');
const names = ['oldUnrefined', 'analyticUnrefined', 'oldRefined', 'analyticRefined'];
const fits = ['wide5', 'wide7', 'narrow5', 'narrow7'];
const sub = (a, b) => a.map((x, i) => x - b[i]);
const norm = v => Math.hypot(...v);
function summarize(rows) {
  const pairMax = (a, b) => Math.max(...rows.map(r => norm(sub(r.paths[a].rate, r.paths[b].rate))));
  const interactionMax = Math.max(...rows.map(r => norm(sub(
    sub(r.paths.analyticRefined.rate, r.paths.oldRefined.rate),
    sub(r.paths.analyticUnrefined.rate, r.paths.oldUnrefined.rate)))));
  const fitResults = Object.fromEntries(fits.map(fit => [fit, Object.fromEntries(names.map(name => {
    const residuals = rows.map(r => norm(sub(r.paths[name].rate, r.fits[fit])));
    return [name, { max: Math.max(...residuals), rms: Math.sqrt(residuals.reduce((s, x) => s + x*x, 0) / residuals.length) }];
  }))]));
  return { cases: rows.length, maxRefinementEffectOld: pairMax('oldRefined', 'oldUnrefined'),
    maxRefinementEffectAnalytic: pairMax('analyticRefined', 'analyticUnrefined'),
    maxDeflectionEffectUnrefined: pairMax('analyticUnrefined', 'oldUnrefined'),
    maxDeflectionEffectRefined: pairMax('analyticRefined', 'oldRefined'), interactionMax, fitResults };
}
function sha256(path) {
  const hash = crypto.createHash('sha256'), fd = fs.openSync(path, 'r'), buffer = Buffer.allocUnsafe(1024 * 1024);
  try { for (;;) { const n = fs.readSync(fd, buffer, 0, buffer.length, null); if (!n) break; hash.update(buffer.subarray(0, n)); } }
  finally { fs.closeSync(fd); }
  return hash.digest('hex');
}
function main() {
  const [kernel, kernelHash, input, inputHash, exe, output] = process.argv.slice(2);
  if (!output || process.argv.length !== 8) throw new Error('usage: node audit-emission-deflection-matrix.cjs KERNEL KERNEL_SHA256 DENSE_JSON DENSE_SHA256 EXAMPLE_EXE NEW_REPORT');
  const inputBefore = sha256(input), kernelBefore = sha256(kernel);
  assert.equal(inputBefore, inputHash.toLowerCase(), 'input hash mismatch');
  assert.equal(kernelBefore, kernelHash.toLowerCase(), 'kernel hash mismatch');
  const source = JSON.parse(fs.readFileSync(input, 'utf8'));
  assert.equal(source.schema, 'horizons-near-sun-polynomial-rates/1');
  assert.equal(source.kernelSha256, kernelBefore);
  assert.ok(Array.isArray(source.rows) && source.rows.length > 0);
  const keys = new Set();
  for (const row of source.rows) {
    assert.ok(Number.isInteger(row.id) && row.id >= 0 && row.id <= 9 && Number.isFinite(row.jd));
    const key = `${row.id},${row.jd}`; assert.ok(!keys.has(key), `duplicate ${key}`); keys.add(key);
    for (const fit of fits) assert.ok(row[fit].rate.length === 2 && row[fit].rate.every(Number.isFinite));
  }
  const inputCsv = source.rows.map(r => `${r.id},${r.jd}`).join('\n') + '\n';
  const run = spawnSync(exe, [kernel], { input: inputCsv, encoding: 'utf8', maxBuffer: 8_000_000 });
  if (run.error) throw run.error;
  if (run.status !== 0) throw new Error(`matrix failed: ${run.stderr}`);
  const lines = run.stdout.trim().split(/\r?\n/);
  assert.equal(lines[0], 'body,jd_tdb,old_unref_ra,old_unref_dec,old_unref_dra,old_unref_ddec,analytic_unref_ra,analytic_unref_dec,analytic_unref_dra,analytic_unref_ddec,old_ref_ra,old_ref_dec,old_ref_dra,old_ref_ddec,analytic_ref_ra,analytic_ref_dec,analytic_ref_dra,analytic_ref_ddec');
  assert.equal(lines.length, source.rows.length + 1);
  const rows = source.rows.map((sourceRow, index) => {
    const v = lines[index + 1].split(',').map(Number);
    assert.equal(v.length, 18); assert.ok(v.every(Number.isFinite));
    assert.equal(v[0], sourceRow.id); assert.equal(v[1], sourceRow.jd);
    const paths = Object.fromEntries(names.map((name, i) => [name, { raDeg: v[2 + 4*i], decDeg: v[3 + 4*i], rate: v.slice(4 + 4*i, 6 + 4*i) }]));
    // Cross-check against the independently written same-model prototype.
    assert.ok(norm(sub(paths.oldUnrefined.rate, sourceRow.old)) < 1e-7, 'old prototype mismatch');
    assert.ok(norm(sub(paths.analyticUnrefined.rate, sourceRow.candidate)) < 1e-7, 'analytic prototype mismatch');
    return { id: sourceRow.id, year: sourceRow.year, jd: sourceRow.jd, elongationDeg: sourceRow.elongationDeg,
      paths, fits: Object.fromEntries(fits.map(fit => [fit, sourceRow[fit].rate])) };
  });
  assert.equal(sha256(input), inputBefore, 'input changed');
  assert.equal(sha256(kernel), kernelBefore, 'kernel changed');
  const report = { schema: 'emission-deflection-matrix/1', status: 'EXPLORATORY', inputSha256: inputBefore,
    kernelSha256: kernelBefore, units: 'ICRF tangent rates arcsec/TDB day; RA and Dec degrees',
    note: 'Both paths and Horizons use DE441. Fitted Horizons positions are correlated, and fit spread is not an error bound.',
    summary: summarize(rows), rows };
  fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify(report.summary, null, 2));
}
if (require.main === module) main();
module.exports = { summarize };
