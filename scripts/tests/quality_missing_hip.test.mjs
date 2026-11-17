// SPDX-License-Identifier: CPAL-1.0. Fictional HIP identifiers.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { inspectAvailable } from '../../bindings/node/scripts/audit-star-quality.mjs';

test('isolates absent records and preserves available order', () => {
  const inspect = (_path, hips) => {
    if (hips.includes(2) || hips.includes(4)) {
      const e = new Error('E_CATALOG_FILE: hip2 field HIP not found is out of range'); e.code = 'E_CATALOG_FILE'; throw e;
    }
    return hips.map(hip => ({ hip }));
  };
  assert.deepEqual(inspectAvailable('mock', [1, 2, 3, 4], inspect), { rows: [{ hip: 1 }, { hip: 3 }], missing: [2, 4] });
});
test('does not hide malformed catalog or unrelated failures', () => {
  assert.throws(() => inspectAvailable('mock', [1], () => { throw new Error('bad catalog'); }), /bad catalog/);
});
