// SPDX-License-Identifier: CPAL-1.0. Fictional delta T table.
import {test} from 'node:test';import assert from 'node:assert/strict';import {createRequire} from 'node:module';
const {auditHistoricalDeltaT}=createRequire(import.meta.url)('../scripts/audit-historical-deltat.cjs');
test('summarizes time-model residuals without inferring uncertainties',()=>{
  const r=auditHistoricalDeltaT('Year TDT-UT1 error\n(s) (s)\n1657.000 40 12\n1657.500 41 15\n',y=>y===1657?42:40);
  assert.equal(r.summary.all.count,2);assert.equal(r.summary.all.maxAbsoluteSeconds,2);
  assert.equal(r.rows[0].referenceErrorSeconds,12);assert.equal(r.rows[1].differenceSeconds,-1);
});
test('rejects duplicate epochs, invalid uncertainties and nonfinite model outputs',()=>{
  assert.throws(()=>auditHistoricalDeltaT('1657 40 1\n1657 41 1',()=>1),/sequence/);
  assert.throws(()=>auditHistoricalDeltaT('1657 40 -1',()=>1),/row/);
  assert.throws(()=>auditHistoricalDeltaT('1657 40 1',()=>NaN),/estimate/);
});
