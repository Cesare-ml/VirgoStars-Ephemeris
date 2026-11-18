// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const { summarize } = createRequire(import.meta.url)('../scripts/audit-emission-deflection-matrix.cjs');

test('matrix separates deflection, refinement and their interaction', () => {
  const paths = Object.fromEntries(Object.entries({oldUnrefined: [0, 0], analyticUnrefined: [2, 0], oldRefined: [0, 3], analyticRefined: [3, 3]})
    .map(([name, rate]) => [name, { rate }]));
  const fit = [0, 0];
  const result = summarize([{ paths, fits: {wide5: fit, wide7: fit, narrow5: fit, narrow7: fit} }]);
  assert.equal(result.maxDeflectionEffectUnrefined, 2);
  assert.equal(result.maxDeflectionEffectRefined, 3);
  assert.equal(result.maxRefinementEffectOld, 3);
  assert.equal(result.maxRefinementEffectAnalytic, Math.sqrt(10));
  assert.equal(result.interactionMax, 1);
  assert.equal(result.fitResults.wide5.oldUnrefined.rms, 0);
  assert.ok(Math.abs(result.fitResults.narrow7.analyticRefined.rms - Math.sqrt(18)) < 1e-14);
});
