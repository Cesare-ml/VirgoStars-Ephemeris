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

/** Prepare a reviewed-by-the-caller manifest for loadHipparcos and the facade. */
function prepareStarCatalog(manifest) {
  if (!manifest || typeof manifest !== 'object' || manifest.schema !== 'virgostars-star-manifest/1') {
    fail('expected schema virgostars-star-manifest/1');
  }
  const reviewer = nonempty(manifest.reviewer, 'reviewer');
  const astrometrySource = nonempty(manifest.astrometrySource, 'astrometrySource');
  const dataUseDecision = nonempty(manifest.dataUseDecision, 'dataUseDecision');
  if (!['strict', 'traditional'].includes(manifest.qualityPolicy)) fail('qualityPolicy must be strict or traditional');
  if (!Array.isArray(manifest.stars) || manifest.stars.length === 0) fail('stars must be a non-empty array');
  const names = new Set();
  const hips = new Set();
  const stars = manifest.stars.map((entry, index) => {
    const label = `stars[${index}]`;
    if (!entry || typeof entry !== 'object') fail(`${label} must be an object`);
    const name = nonempty(entry.name, `${label}.name`);
    const displayName = entry.displayName === undefined ? undefined : nonempty(entry.displayName, `${label}.displayName`);
    if (displayName !== undefined) nonempty(entry.displayNameSource, `${label}.displayNameSource`);
    if (!Number.isSafeInteger(entry.hip) || entry.hip <= 0 || entry.hip > 0xffffffff || hips.has(entry.hip)) {
      fail(`${label}.hip must be unique and within the u32 range`);
    }
    hips.add(entry.hip);
    if (!Array.isArray(entry.aliases)) fail(`${label}.aliases must be an array`);
    const aliases = entry.aliases.map((alias) => nonempty(alias, `${label}.alias`));
    for (const alias of [name, ...aliases]) {
      const key = alias.toLocaleLowerCase('en-US');
      if (names.has(key)) fail(`duplicate star name or alias: ${alias}`);
      names.add(key);
    }
    nonempty(entry.identitySource, `${label}.identitySource`);
    nonempty(entry.qualityDecision, `${label}.qualityDecision`);
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
    return { name, aliases, ...(displayName === undefined ? {} : { displayName }), hip: entry.hip, radialVelocityKmS: entry.radialVelocityKmS, magnitude: entry.magnitude };
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
    matchedHips.add(star.hip);
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

module.exports = { prepareStarCatalog, auditStarCatalogCoverage };
