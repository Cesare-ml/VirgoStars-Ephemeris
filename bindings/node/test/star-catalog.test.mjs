import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdtempSync, rmSync, rmdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const { prepareStarCatalog, auditStarCatalogCoverage, createSwissShapedFacade } = require('../index.js');

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

test('optional quality tier stays explicit and cannot exceed a strict policy', () => {
  const input = fixture(); input.stars[0].qualityTier = 'traditional';
  assert.throws(() => prepareStarCatalog(input), error => error.code === 'E_CATALOG');
  input.qualityPolicy = 'traditional';
  assert.equal(prepareStarCatalog(input).stars[0].qualityTier, 'traditional');
  input.stars[0].qualityTier = 'unreviewed';
  assert.throws(() => prepareStarCatalog(input), error => error.code === 'E_CATALOG');
  input.stars[0].qualityTier = 'experimental';
  assert.throws(() => prepareStarCatalog(input), error => error.code === 'E_CATALOG');
  input.qualityPolicy = 'experimental';
  assert.equal(prepareStarCatalog(input).stars[0].qualityTier, 'experimental');
  delete input.stars[0].qualityTier;
  assert.throws(() => prepareStarCatalog(input), error => error.code === 'E_CATALOG');
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
  bad((m) => { m.stars[0].displayName = 'Example,example'; });
});

test('a caller-supplied display name changes results without becoming a lookup alias', () => {
  const input = fixture();
  input.stars[0].displayName = 'Example,exFoo';
  input.stars[0].displayNameSource = 'Fictional application naming convention';
  const catalog = prepareStarCatalog(input);
  const engine = { fixstarUt: () => ({ xx: [1, 2, 3, 4, 5, 6], rflag: 1, timeSource: 'test', notes: [] }) };
  const facade = createSwissShapedFacade(engine, { starCatalog: catalog.stars });
  assert.equal(facade.swe_fixstar_ut('Example alias', 2451545, 0).name, 'Example,exFoo');
  assert.equal(facade.swe_fixstar_mag('Example').name, 'Example,exFoo');
  assert.equal(facade.swe_fixstar_mag('Example,exFoo').code, 'E_STAR_NOT_FOUND');
  assert.throws(() => createSwissShapedFacade(engine, { starCatalog: [{ ...catalog.stars[0], displayName: '' }] }), (e) => e.code === 'E_CATALOG');
});

test('Swiss differences are reported for the requested alias without changing other lookups', () => {
  const input=fixture();
  input.stars[0].lookupWarnings=[{name:'Example alias',code:'SWISS_POSITION_MISMATCH',detail:'Swiss resolves a different object.'}];
  const catalog=prepareStarCatalog(input);
  const engine={fixstarUt:()=>({xx:[1,2,3,4,5,6],rflag:1,timeSource:'test',notes:[]})};
  const facade=createSwissShapedFacade(engine,{starCatalog:catalog.stars});
  const warning={...input.stars[0].lookupWarnings[0],status:'CALLER_ASSERTED'};
  assert.deepEqual(facade.swe_fixstar_ut('example alias',2451545,0).compatibilityWarning,warning);
  assert.deepEqual(facade.swe_fixstar_mag('EXAMPLE ALIAS').compatibilityWarning,warning);
  assert.equal(facade.swe_fixstar_ut('Example',2451545,0).compatibilityWarning,undefined);
  input.stars[0].lookupWarnings[0].name='Unknown';
  assert.throws(()=>prepareStarCatalog(input),e=>e.code==='E_CATALOG');
  assert.throws(()=>createSwissShapedFacade(engine,{starCatalog:input.stars}),e=>e.code==='E_CATALOG');
});

test('Swiss magnitude discrepancies are surfaced only for the affected lookup', () => {
  const input=fixture();
  input.stars[0].lookupWarnings=[{name:'Example alias',code:'SWISS_MAGNITUDE_MISMATCH',detail:'Source magnitudes differ.'}];
  const facade=createSwissShapedFacade({}, {starCatalog:prepareStarCatalog(input).stars});
  assert.equal(facade.swe_fixstar_mag('example alias').compatibilityWarning.code,'SWISS_MAGNITUDE_MISMATCH');
  assert.equal(facade.swe_fixstar_mag('Example').compatibilityWarning,undefined);
});

test('coverage reports aliases, missing names and explicit catalog assumptions without approving them', () => {
  const manifest = fixture();
  manifest.stars.push({ name: 'Second', aliases: [], hip: 456, identitySource: 'Fictional', qualityDecision: 'Fictional',
    magnitude: null, magnitudeMissingReason: 'Missing example magnitude',
    radialVelocityKmS: 0, radialVelocitySource: 'Fictional supplied value' });
  const before = structuredClone(manifest);
  const report = auditStarCatalogCoverage(manifest, [' Example ', 'example alias', 'Second', 'Not present']);
  assert.equal(report.schema, 'virgostars-star-coverage/1');
  assert.equal(report.status, 'REVIEW_REQUIRED');
  assert.equal(report.matchedCount, 3);
  assert.equal(report.uniqueMatchedStarCount, 2);
  assert.equal(report.unmatchedCount, 1);
  assert.equal(report.missingMagnitudeCount, 1);
  assert.equal(report.assumedZeroRadialVelocityCount, 2, 'counts requested names, not distinct stars');
  assert.deepEqual(report.results[0], { requestedName: 'Example', status: 'MATCHED_CALLER_ASSERTION', canonicalName: 'Example',
    hip: 123, magnitude: 'VALUE_SUPPLIED', radialVelocity: 'ASSUMED_ZERO' });
  assert.equal(report.results[2].radialVelocity, 'VALUE_SUPPLIED');
  assert.equal(report.results[3].status, 'NOT_IN_MANIFEST');
  assert.equal(report.hip2Eligibility, 'NOT_CHECKED');
  assert.equal(report.sourceRights, 'NOT_VERIFIED');
  assert.equal(report.scientificIdentity, 'NOT_VERIFIED');
  assert.deepEqual(manifest, before);
  for (const bad of [[], ['Example', ' example '], [''], [123], 'Example']) {
    assert.throws(() => auditStarCatalogCoverage(manifest, bad), (error) => error.code === 'E_CATALOG');
  }
  assert.throws(() => auditStarCatalogCoverage({ ...manifest, reviewer: '' }, ['Example']), (error) => error.code === 'E_CATALOG');
});

test('offline coverage CLI reports gaps and fails on invalid input', () => {
  const directory = mkdtempSync(join(tmpdir(), 'virgostars-catalog-coverage-'));
  const manifestPath = join(directory, 'manifest.json');
  const requestsPath = join(directory, 'requests.json');
  try {
    const script = fileURLToPath(new URL('../scripts/audit-star-manifest-coverage.mjs', import.meta.url));
    writeFileSync(manifestPath, JSON.stringify(fixture()));
    writeFileSync(requestsPath, JSON.stringify(['Example alias', 'Missing']));
    const completed = spawnSync(process.execPath, [script, manifestPath, requestsPath], { encoding: 'utf8' });
    assert.equal(completed.status, 0, completed.stderr);
    const report = JSON.parse(completed.stdout);
    assert.equal(report.status, 'REVIEW_REQUIRED');
    assert.equal(report.matchedCount, 1);
    assert.equal(report.unmatchedCount, 1);
    writeFileSync(requestsPath, JSON.stringify(['Example', 'EXAMPLE']));
    const invalid = spawnSync(process.execPath, [script, manifestPath, requestsPath], { encoding: 'utf8' });
    assert.equal(invalid.status, 1);
    assert.equal(invalid.stdout, '');
    assert.match(invalid.stderr, /duplicate requested name/);
  } finally {
    rmSync(manifestPath, { force: true });
    rmSync(requestsPath, { force: true });
    rmdirSync(directory);
  }
});
