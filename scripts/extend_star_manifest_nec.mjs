// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Local NEC exact-name + position screening; the result remains a candidate.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { inventorySwissStars } from './inventory_swiss_stars.mjs';
import { inspectAvailable } from '../bindings/node/scripts/audit-star-quality.mjs';
const require = createRequire(import.meta.url);
const { prepareStarCatalog } = require('../bindings/node/star-catalog.js');
const { inspectHipparcosQuality } = require('../bindings/node/index.js');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const fold = text => text.trim().toLocaleLowerCase('en-US');

// Strict RFC-style CSV handling, including quoted commas, quotes and newlines.
export function parseCsv(text) {
  const records = []; let record = []; let field = ''; let quoted = false; let closed = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quoted) {
      if (c === '"' && text[i + 1] === '"') { field += '"'; i++; }
      else if (c === '"') { quoted = false; closed = true; }
      else field += c;
    } else if (c === '"') {
      if (field || closed) throw new Error('malformed CSV quote');
      quoted = true;
    } else if (c === ',' || c === '\n' || c === '\r') {
      record.push(field); field = ''; closed = false;
      if (c !== ',') { records.push(record); record = []; if (c === '\r' && text[i + 1] === '\n') i++; }
    } else { if (closed) throw new Error('trailing text after CSV quote'); field += c; }
  }
  if (quoted) throw new Error('unterminated CSV quote');
  if (field || record.length || closed) { record.push(field); records.push(record); }
  return records;
}

const number = text => text && /^[-+]?(?:\d+(?:\.\d*)?|\.\d+)$/.test(text) ? Number(text) : NaN;
const vec = (ra, dec) => { const r = ra * Math.PI / 180; const d = dec * Math.PI / 180; return [Math.cos(d)*Math.cos(r), Math.cos(d)*Math.sin(r), Math.sin(d)]; };
function separation(a, b) {
  const u = vec(...a); const v = vec(...b);
  const c = [u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0]];
  return Math.atan2(Math.hypot(...c), u[0]*v[0]+u[1]*v[1]+u[2]*v[2]) * 180 / Math.PI * 3600;
}

export function prepareNecExtension({ manifest, swissText, necText, hip2Sha256, hip2Path, inspect = inspectHipparcosQuality }) {
  prepareStarCatalog(manifest);
  if (manifest.sourceHashes?.hip2Sha256 !== hip2Sha256 || !/^[a-f0-9]{64}$/.test(hip2Sha256)) throw new Error('HIP2 source mismatch');
  const inventory = inventorySwissStars(swissText); const swissLines = swissText.split(/\r?\n/);
  const headersAndRows = parseCsv(necText.replace(/^\ufeff/, '')); const header = headersAndRows.shift();
  if (!header) throw new Error('empty NEC catalog');
  const cols = ['Name', 'HIP', 'RA2000', 'DE2000', 'Vmag'].map(key => header.indexOf(key));
  if (new Set(header).size !== header.length || cols.some(index => index < 0)) throw new Error('unrecognized NEC header');
  const rowsByName = new Map();
  for (const row of headersAndRows) {
    if (row.length !== header.length) throw new Error('malformed NEC row');
    const name = row[cols[0]].trim(); if (!name) continue;
    const key = fold(name); const values = rowsByName.get(key) ?? []; values.push(row); rowsByName.set(key, values);
  }
  const swissByName = new Map();
  for (const row of inventory.entries) if (row.name) {
    const key = fold(row.name); const values = swissByName.get(key) ?? []; values.push(row); swissByName.set(key, values);
  }
  const usedNames = new Set(manifest.stars.flatMap(s => [s.name, ...s.aliases]).map(fold));
  const usedHips = new Set(manifest.stars.map(s => s.hip)); const candidates = [];
  for (const [key, records] of swissByName) {
    if (usedNames.has(key) || !rowsByName.has(key) || rowsByName.get(key).length !== 1
        || new Set(records.map(r => fold(r.designation))).size !== 1 || records.some(r => r.equinox !== 'ICRS')) continue;
    const fields = records.map(r => swissLines[r.line - 1].split('#', 1)[0].split(',').map(v => v.trim()));
    if (new Set(fields.map(f => JSON.stringify(f.slice(2, 14)))).size !== 1) continue;
    const swiss = fields[0]; const row = rowsByName.get(key)[0]; const hipMatch = /^HIP\s+(\d+)$/.exec(row[cols[1]].trim());
    const ra = number(row[cols[2]].trim()); const dec = number(row[cols[3]].trim()); const magnitude = number(row[cols[4]].trim());
    if (!hipMatch || ![ra, dec, magnitude].every(Number.isFinite) || ra < 0 || ra >= 360 || Math.abs(dec) > 90) continue;
    const hip = Number(hipMatch[1]); if (!Number.isSafeInteger(hip) || hip < 1 || usedHips.has(hip)) continue;
    const hms = swiss.slice(3, 6).map(number); const dms = swiss.slice(6, 9).map(number);
    if (![...hms, ...dms].every(Number.isFinite)) continue;
    const sra = 15 * (hms[0] + hms[1] / 60 + hms[2] / 3600);
    const sdec = (swiss[6].startsWith('-') ? -1 : 1) * (Math.abs(dms[0]) + dms[1] / 60 + dms[2] / 3600);
    const arcsec = separation([ra, dec], [sra, sdec]);
    if (!Number.isFinite(arcsec) || arcsec > 30) continue;
    candidates.push({ name: records[0].name, hip, magnitude, separationArcsec: arcsec });
  }
  const multiplicity = new Map();
  for (const row of candidates) multiplicity.set(row.hip, (multiplicity.get(row.hip) ?? 0) + 1);
  const unique = candidates.filter(row => multiplicity.get(row.hip) === 1);
  const { rows, missing } = inspectAvailable(hip2Path, unique.map(row => row.hip), inspect);
  const inspected = new Set(rows.map(row => row.hip));
  if (inspected.size !== rows.length || rows.some(row => typeof row.strictEligible !== 'boolean')
      || rows.length + missing.length !== unique.length
      || unique.some(row => !inspected.has(row.hip) && !missing.includes(row.hip))) throw new Error('inconsistent HIP2 inspection');
  const strict = new Set(rows.filter(row => row.strictEligible).map(row => row.hip));
  const selected = unique.filter(row => strict.has(row.hip));
  const necSha256 = sha(Buffer.from(necText)); const swissSha256 = sha(Buffer.from(swissText));
  if (manifest.sourceHashes?.swissCatalogSha256 && manifest.sourceHashes.swissCatalogSha256 !== swissSha256) throw new Error('Swiss catalog source mismatch');
  const stars = selected.map(row => ({ name: row.name, aliases: [], hip: row.hip,
    identitySource: `Exact NEC name/HIP and Swiss coordinate candidate within 30 arcsec; NEC SHA256 ${necSha256}; Swiss SHA256 ${swissSha256}; identity unreviewed`,
    qualityDecision: 'Local candidate: strict HIP2 filter; no individual scientific review',
    magnitude: row.magnitude, magnitudeSource: `NEC Vmag, local snapshot SHA256 ${necSha256}; provenance unverified`,
    radialVelocityKmS: null, radialVelocityMissingReason: 'HIP2 has no radial velocity; local candidate assumes zero km/s' }));
  const output = { ...manifest, reviewer: `${manifest.reviewer}; NEC additions require individual review`,
    sourceHashes: { ...manifest.sourceHashes, swissCatalogSha256: swissSha256, necSha256 }, stars: [...manifest.stars, ...stars] };
  prepareStarCatalog(output);
  return { manifest: output, report: { schema: 'virgostars-nec-extension/1', status: 'REVIEW_REQUIRED',
    swissSha256, necSha256, hip2Sha256, candidates: unique.length, missingHipIds: missing,
    selectedCount: selected.length, selected } };
}

async function main() {
  const [manifestPath, swissPath, necPath, hipPath, outputPath, reportPath, ...extra] = process.argv.slice(2);
  if (![manifestPath, swissPath, necPath, hipPath, outputPath, reportPath].every(Boolean) || extra.length) throw new Error('usage: <manifest> <sefstars> <nec.csv> <hip2> <new-manifest> <new-report>');
  if (new Set([manifestPath, swissPath, necPath, hipPath, outputPath, reportPath].map(p => resolve(p))).size !== 6) throw new Error('paths must differ');
  if (existsSync(outputPath) || existsSync(reportPath)) throw new Error('output already exists');
  const inputs = [manifestPath, swissPath, necPath, hipPath].map(path => readFileSync(path));
  if (inputs.some((b, i) => b.length > (i === 3 ? 100_000_000 : 8_000_000))) throw new Error('input exceeds size limit');
  if ([1, 2].some(i => !Buffer.from(inputs[i].toString('utf8'), 'utf8').equals(inputs[i]))) throw new Error('catalog input is not UTF-8');
  const result = prepareNecExtension({ manifest: JSON.parse(inputs[0]), swissText: inputs[1].toString('utf8'),
    necText: inputs[2].toString('utf8'), hip2Sha256: sha(inputs[3]), hip2Path: hipPath });
  for (let i = 0; i < inputs.length; i++) if (sha(readFileSync([manifestPath, swissPath, necPath, hipPath][i])) !== sha(inputs[i])) throw new Error('input changed');
  writeFileSync(outputPath, `${JSON.stringify(result.manifest, null, 2)}\n`, { flag: 'wx' });
  writeFileSync(reportPath, `${JSON.stringify(result.report, null, 2)}\n`, { flag: 'wx' });
  console.log(`${result.report.selectedCount} strict NEC candidates added; all require identity review.`);
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main().catch(error => { console.error(`NEC extension failed: ${error.message}`); process.exitCode = 1; });
}
