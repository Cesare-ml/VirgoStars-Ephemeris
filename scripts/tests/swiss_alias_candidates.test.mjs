// SPDX-License-Identifier: CPAL-1.0. Fictional records only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { attachAliases } from '../attach_swiss_alias_candidates.mjs';
const row = (name, des, x = 3) => `${name},${des},ICRS,1,2,${x},4,5,6,7,8,9,10,11,0,0`;
const star = name => ({ name, hip: name === 'Alpha' ? 1 : 2, aliases: [], identitySource: 'fixture', qualityDecision: 'fixture', magnitude: 1, magnitudeSource: 'fixture', radialVelocityKmS: null, radialVelocityMissingReason: 'unknown' });
const manifest = { schema: 'virgostars-star-manifest/1', reviewer: 'fixture', astrometrySource: 'fixture', dataUseDecision: 'fixture', qualityPolicy: 'strict', stars: [star('Alpha'), star('Beta')] };
const hash = text => createHash('sha256').update(text).digest('hex');
test('adds only aliases sharing identical rows and unique designations', () => {
  const text = `# sefstars.txt\n${row('Alpha', 'a1')}\n${row('Alias', 'a1')}\n${row('Beta', 'b1')}\n${row('Beta', 'b2')}\n`;
  const { manifest: result, report } = attachAliases(manifest, text, hash(text));
  assert.deepEqual(result.stars[0].aliases, [',a1', 'Alias']);
  assert.deepEqual(result.stars[1].aliases, []);
  assert.deepEqual(report.ambiguous, ['Beta']);
  assert.equal(manifest.stars[0].aliases.length, 0);
});
test('does not group a designation with differing astrometry', () => {
  const text = `# sefstars.txt\n${row('Alpha', 'a1')}\n${row('Alias', 'a1', 5)}\n${row('Beta', 'b1')}\n`;
  const { manifest: result, report } = attachAliases(manifest, text, hash(text));
  assert.deepEqual(result.stars[0].aliases, []); assert.ok(report.ambiguous.includes('Alpha'));
  assert.throws(() => attachAliases(manifest, text, 'b'.repeat(64)), /hash mismatch/);
});
