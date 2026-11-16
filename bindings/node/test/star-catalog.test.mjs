import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { prepareStarCatalog, createSwissShapedFacade } = require('../index.js');

function fixture() {
  return {
    schema: 'virgostars-star-manifest/1', reviewer: 'Example reviewer',
    astrometrySource: 'Example catalog, fictional test', dataUseDecision: 'Fictional test only', qualityPolicy: 'strict',
    stars: [{ name: 'Example', aliases: ['Example alias'], hip: 123,
      identitySource: 'Fictional identity source', qualityDecision: 'Reviewed fictional object',
      magnitude: 1.2, magnitudeSource: 'Fictional photometry',
      radialVelocityKmS: null, radialVelocityMissingReason: 'No fictional measurement' }],
  };
}

test('catalog manifests require explicit sources and assumptions before facade use', () => {
  const input = fixture();
  const result = prepareStarCatalog(input);
  assert.deepEqual(result.hips, [123]);
  assert.equal(result.qualityPolicy, 'strict');
  const facade = createSwissShapedFacade({}, { starCatalog: result.stars });
  assert.equal(facade.swe_fixstar_mag('example alias').magnitude, 1.2);
  input.stars[0].aliases[0] = 'changed';
  assert.equal(result.stars[0].aliases[0], 'Example alias');
});

test('catalog manifests reject missing evidence, implicit values and ambiguous identities', () => {
  const bad = (mutate) => { const m = fixture(); mutate(m); assert.throws(() => prepareStarCatalog(m), (e) => e.code === 'E_CATALOG'); };
  bad((m) => { m.stars[0].identitySource = ''; });
  bad((m) => { m.stars[0].qualityDecision = ''; });
  bad((m) => { m.stars[0].magnitudeSource = ''; });
  bad((m) => { m.stars[0].radialVelocityMissingReason = ''; });
  bad((m) => { delete m.stars[0].radialVelocityKmS; });
  bad((m) => { m.stars[0].magnitude = NaN; });
  bad((m) => { m.stars[0].aliases.push(' example '); });
  bad((m) => { m.stars.push({ ...m.stars[0], name: 'Other', aliases: [] }); });
  bad((m) => { m.qualityPolicy = 'automatic'; });
  bad((m) => { m.dataUseDecision = ''; });
});
