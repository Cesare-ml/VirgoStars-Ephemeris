// SPDX-License-Identifier: CPAL-1.0. Fictional records only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { inventorySwissStars } from '../inventory_swiss_stars.mjs';

const data = `# sefstars.txt fixture
Alpha,des1,ICRS,1,2,3,4,5,6,7,8,9,10,11,0,0
Alias,des1,ICRS,1,2,3,4,5,6,7,8,9,10,11,0,0 # comment, with comma
,des2,2000,1,2,3,4,5,6,7,8,9,10,11
alpha,des3,1950,1,2,3,4,5,6,7,8,9,10,11
`;
test('inventories every row, preserving aliases, unnamed entries, repeats and line numbers', () => {
  const report = inventorySwissStars(data);
  assert.deepEqual(report.counts, { records: 4, namedRecords: 3, uniqueNames: 2, uniqueDesignations: 3, repeatedNames: 1, repeatedDesignations: 1 });
  assert.deepEqual(report.designations[0].lines, [2, 3]);
  assert.equal(report.entries[2].name, null);
});
test('rejects malformed rows instead of silently omitting them', () => {
  assert.throws(() => inventorySwissStars('wrong header'));
  assert.throws(() => inventorySwissStars('# sefstars.txt\nmalformed'));
  assert.throws(() => inventorySwissStars(data.replace('1,2,3,4,5,6,7,8,9,10,11', '1,2,NaN,4,5,6,7,8,9,10,11')));
});
