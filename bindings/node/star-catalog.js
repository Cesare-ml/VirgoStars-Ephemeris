'use strict';
// SPDX-License-Identifier: CPAL-1.0; see LICENSE and required VirgoStars attribution.
// Validates caller-supplied star metadata; it neither downloads nor approves data.

function fail(message) {
  const error = new Error(`E_CATALOG: ${message}`);
  error.code = 'E_CATALOG';
  throw error;
}

function nonempty(value, label) {
  if (typeof value !== 'string' || !value.trim()) fail(`${label} must be a non-empty string`);
  return value.trim();
}

/** Warnings describe a caller-asserted lookup difference for a specific name. */
function validateLookupWarnings(warnings, names, label = 'lookupWarnings') {
  if (warnings === undefined) return undefined;
  if (!Array.isArray(warnings) || warnings.length > names.length) fail(`${label} must be a bounded array`);
  const validNames = new Set(names.map(name => name.trim().toLocaleLowerCase('en-US')));
  const seen = new Set();
  return warnings.map((warning) => {
    if (!warning || typeof warning !== 'object' || Array.isArray(warning)) fail(`${label} has invalid warning`);
    const name = nonempty(warning.name, `${label}.name`);
    const key = name.toLocaleLowerCase('en-US');
    if (!validNames.has(key) || seen.has(key)) fail(`${label} must reference distinct catalog names`);
    seen.add(key);
    if (!['SWISS_POSITION_MISMATCH', 'SWISS_MAGNITUDE_MISMATCH', 'SWISS_LOOKUP_UNAVAILABLE', 'SWISS_RETURNED_DIFFERENT_NAME'].includes(warning.code)
      || typeof warning.detail !== 'string' || !warning.detail.trim() || warning.detail.length > 500) fail(`${label} has invalid warning fields`);
    return { name, code: warning.code, detail: warning.detail.trim(), status:'CALLER_ASSERTED' };
  });
}

/** Explicit finite-distance ICRS astrometry for a non-HIP source. */
function validateExplicitAstrometry(value, label = 'astrometry') {
  const fields = ['epochTdbJd', 'rightAscensionDeg', 'declinationDeg', 'properMotionRaCosDecMasPerYear',
    'properMotionDecMasPerYear', 'parallaxMas'];
  if (!value || typeof value !== 'object' || Array.isArray(value)
    || fields.some(key => typeof value[key] !== 'number' || !Number.isFinite(value[key]))
    || value.rightAscensionDeg < 0 || value.rightAscensionDeg >= 360 || Math.abs(value.declinationDeg) >= 90
    || value.parallaxMas < 0.001 || value.parallaxMas > 10000) fail(`${label} has invalid ICRS astrometry`);
  return Object.fromEntries(fields.map(key => [key, value[key]]));
}

/** Prepare a reviewed-by-the-caller manifest for loadHipparcos and the facade. */
function prepareStarCatalog(manifest) {
  if (!manifest || typeof manifest !== 'object' || !['virgostars-star-manifest/1','virgostars-star-manifest/2'].includes(manifest.schema)) {
    fail('expected star manifest schema /1 or /2');
  }
  const reviewer = nonempty(manifest.reviewer, 'reviewer');
  const astrometrySource = nonempty(manifest.astrometrySource, 'astrometrySource');
  const dataUseDecision = nonempty(manifest.dataUseDecision, 'dataUseDecision');
  if (!['strict', 'traditional', 'experimental'].includes(manifest.qualityPolicy)) fail('invalid explicit qualityPolicy');
  if (!Array.isArray(manifest.stars) || manifest.stars.length === 0) fail('stars must be a non-empty array');
  const names = new Set();
  const hips = new Set();
  const sourceIds = new Set();
  const stars = manifest.stars.map((entry, index) => {
    const label = `stars[${index}]`;
    if (!entry || typeof entry !== 'object') fail(`${label} must be an object`);
    const name = nonempty(entry.name, `${label}.name`);
    const displayName = entry.displayName === undefined ? undefined : nonempty(entry.displayName, `${label}.displayName`);
    if (displayName !== undefined) nonempty(entry.displayNameSource, `${label}.displayNameSource`);
    const direct = entry.astrometry !== undefined;
    let astrometry, sourceId;
    if (direct) {
      if (manifest.schema !== 'virgostars-star-manifest/2' || entry.hip !== undefined || manifest.qualityPolicy !== 'experimental')
        fail(`${label} direct astrometry requires schema /2 and experimental policy`);
      sourceId = nonempty(entry.sourceId, `${label}.sourceId`);
      if (sourceIds.has(sourceId.toLocaleLowerCase('en-US'))) fail(`${label}.sourceId must be unique`);
      sourceIds.add(sourceId.toLocaleLowerCase('en-US'));
      nonempty(entry.astrometrySource, `${label}.astrometrySource`);
      astrometry = validateExplicitAstrometry(entry.astrometry, `${label}.astrometry`);
    } else {
      if (entry.sourceId !== undefined || !Number.isSafeInteger(entry.hip) || entry.hip <= 0 || entry.hip > 0xffffffff || hips.has(entry.hip))
        fail(`${label}.hip must be unique and within the u32 range`);
      hips.add(entry.hip);
    }
    if (!Array.isArray(entry.aliases)) fail(`${label}.aliases must be an array`);
    const aliases = entry.aliases.map((alias) => nonempty(alias, `${label}.alias`));
    const lookupWarnings = validateLookupWarnings(entry.lookupWarnings, [name, ...aliases], `${label}.lookupWarnings`);
    for (const alias of [name, ...aliases]) {
      const key = alias.toLocaleLowerCase('en-US');
      if (names.has(key)) fail(`duplicate star name or alias: ${alias}`);
      names.add(key);
    }
    nonempty(entry.identitySource, `${label}.identitySource`);
    nonempty(entry.qualityDecision, `${label}.qualityDecision`);
    if (entry.qualityTier !== undefined && (!['strict', 'traditional', 'experimental'].includes(entry.qualityTier)
        || (manifest.qualityPolicy === 'strict' && entry.qualityTier !== 'strict')
        || (manifest.qualityPolicy === 'traditional' && entry.qualityTier === 'experimental'))) {
      fail(`${label}.qualityTier is inconsistent with qualityPolicy`);
    }
    if (manifest.qualityPolicy === 'experimental' && entry.qualityTier === undefined) {
      fail(`${label}.qualityTier is required for experimental policy`);
    }
    if (direct && entry.qualityTier !== 'experimental') fail(`${label} direct astrometry must be explicitly experimental`);
    for (const [field, source, reason] of [
      ['magnitude', 'magnitudeSource', 'magnitudeMissingReason'],
      ['radialVelocityKmS', 'radialVelocitySource', 'radialVelocityMissingReason'],
    ]) {
      const value = entry[field];
      if (value === null) nonempty(entry[reason], `${label}.${reason}`);
      else {
        if (typeof value !== 'number' || !Number.isFinite(value)) fail(`${label}.${field} must be finite or null`);
        nonempty(entry[source], `${label}.${source}`);
      }
    }
    return { name, aliases, ...(displayName === undefined ? {} : { displayName }),
      ...(entry.qualityTier === undefined ? {} : { qualityTier: entry.qualityTier }),
      ...(lookupWarnings === undefined ? {} : { lookupWarnings }),
      ...(direct ? {sourceId, astrometry} : {hip:entry.hip}), radialVelocityKmS: entry.radialVelocityKmS, magnitude: entry.magnitude };
  });
  return { stars, hips: [...hips], qualityPolicy: manifest.qualityPolicy, reviewer, astrometrySource, dataUseDecision };
}

/** Audit requested lookup strings against a caller-reviewed manifest snapshot. */
function auditStarCatalogCoverage(manifest, requestedNames) {
  const catalog = prepareStarCatalog(manifest);
  if (!Array.isArray(requestedNames) || requestedNames.length === 0) fail('requestedNames must be a non-empty array');
  const lookup = new Map();
  for (const star of catalog.stars) {
    for (const name of [star.name, ...star.aliases]) lookup.set(name.toLocaleLowerCase('en-US'), star);
  }
  const seen = new Set();
  const matchedHips = new Set();
  const results = requestedNames.map((value, index) => {
    const requestedName = nonempty(value, `requestedNames[${index}]`);
    const key = requestedName.toLocaleLowerCase('en-US');
    if (seen.has(key)) fail(`duplicate requested name: ${requestedName}`);
    seen.add(key);
    const star = lookup.get(key);
    if (!star) return { requestedName, status: 'NOT_IN_MANIFEST' };
    matchedHips.add(star.hip === undefined ? `source:${star.sourceId}` : star.hip);
    return {
      requestedName, status: 'MATCHED_CALLER_ASSERTION', canonicalName: star.name, hip: star.hip,
      magnitude: star.magnitude === null ? 'MISSING' : 'VALUE_SUPPLIED',
      radialVelocity: star.radialVelocityKmS === null ? 'ASSUMED_ZERO' : 'VALUE_SUPPLIED',
    };
  });
  const matched = results.filter((row) => row.status === 'MATCHED_CALLER_ASSERTION');
  return {
    schema: 'virgostars-star-coverage/1', status: 'REVIEW_REQUIRED',
    catalogStarCount: catalog.stars.length, requestedCount: results.length,
    matchedCount: matched.length, unmatchedCount: results.length - matched.length,
    uniqueMatchedStarCount: matchedHips.size,
    missingMagnitudeCount: matched.filter((row) => row.magnitude === 'MISSING').length,
    assumedZeroRadialVelocityCount: matched.filter((row) => row.radialVelocity === 'ASSUMED_ZERO').length,
    qualityPolicy: catalog.qualityPolicy, hip2Eligibility: 'NOT_CHECKED',
    sourceRights: 'NOT_VERIFIED', scientificIdentity: 'NOT_VERIFIED', results,
  };
}

module.exports = { prepareStarCatalog, auditStarCatalogCoverage, validateLookupWarnings, validateExplicitAstrometry };
