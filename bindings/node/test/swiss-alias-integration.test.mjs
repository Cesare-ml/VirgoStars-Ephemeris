// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { Engine, prepareStarCatalog, createSwissShapedFacade } = require('../index.js');
const { DE441_PATH, HIP2_PATH, SWISS_ALIAS_MANIFEST } = process.env;

test('local alias candidates resolve identically to their selected star',
  { skip: DE441_PATH && HIP2_PATH && SWISS_ALIAS_MANIFEST ? false : 'local kernel, HIP2 and alias manifest required' }, () => {
    const manifest = JSON.parse(readFileSync(SWISS_ALIAS_MANIFEST, 'utf8'));
    const catalog = prepareStarCatalog(manifest);
    const engine = new Engine(DE441_PATH);
    engine.loadHipparcos(HIP2_PATH, catalog.hips, catalog.qualityPolicy);
    const facade = createSwissShapedFacade(engine, { starCatalog: catalog.stars });
    let count = 0;
    for (const star of catalog.stars) {
      if (!star.aliases.length) continue;
      const canonical = facade.swe_fixstar_ut(star.name, 2451545, 0);
      assert.ok(Number.isFinite(canonical.longitude));
      for (const alias of star.aliases) {
        const actual=facade.swe_fixstar_ut(alias, 2451545, 0);
        const {compatibilityWarning: canonicalWarning,...canonicalPosition}=canonical;
        const {compatibilityWarning: aliasWarning,...aliasPosition}=actual;
        assert.deepEqual(aliasPosition, canonicalPosition);
        const expected=star.lookupWarnings?.find(w=>w.name.toLocaleLowerCase('en-US')===alias.toLocaleLowerCase('en-US'));
        assert.deepEqual(aliasWarning,expected);
        if(canonicalWarning) assert.equal(canonicalWarning.name.toLocaleLowerCase('en-US'),star.name.toLocaleLowerCase('en-US'));
        count++;
      }
    }
    assert.equal(count, Number(process.env.SWISS_ALIAS_EXPECTED || 275));
  });
