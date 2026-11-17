// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Experimental sky-position candidates; a positional match is not identity proof.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { inventorySwissStars } from './inventory_swiss_stars.mjs';
import { parseCsv } from './extend_star_manifest_nec.mjs';
import { inspectAvailable } from '../bindings/node/scripts/audit-star-quality.mjs';
const require = createRequire(import.meta.url);
const { prepareStarCatalog } = require('../bindings/node/star-catalog.js');
const { inspectHipparcosQuality } = require('../bindings/node/index.js');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const fold = value => value.trim().toLocaleLowerCase('en-US');
const numeric = value => value && /^[-+]?(?:\d+(?:\.\d*)?|\.\d+)$/.test(value) ? Number(value) : NaN;
const rad = Math.PI / 180;
const vector = (ra, dec) => [Math.cos(dec * rad)*Math.cos(ra * rad), Math.cos(dec * rad)*Math.sin(ra * rad), Math.sin(dec * rad)];
function separation(a, b) {
  const x = vector(...a); const y = vector(...b);
  return Math.atan2(Math.hypot(x[1]*y[2]-x[2]*y[1], x[2]*y[0]-x[0]*y[2], x[0]*y[1]-x[1]*y[0]), x[0]*y[0]+x[1]*y[1]+x[2]*y[2]) / rad * 3600;
}

export function extendByPosition({ manifest, swissText, necText, hip2Sha256, hip2Path, inspect = inspectHipparcosQuality, qualityPolicy = 'strict' }) {
  prepareStarCatalog(manifest);
  if (!['strict', 'traditional'].includes(qualityPolicy) || manifest.qualityPolicy !== 'strict') throw new Error('a strict base and explicit strict or traditional policy are required');
  const swissSha256 = hash(Buffer.from(swissText)); const necSha256 = hash(Buffer.from(necText));
  if (manifest.sourceHashes?.hip2Sha256 !== hip2Sha256 || manifest.sourceHashes?.swissCatalogSha256 !== swissSha256
      || manifest.sourceHashes?.necSha256 !== necSha256) throw new Error('source hash mismatch');
  const inventory = inventorySwissStars(swissText); const lines = swissText.split(/\r?\n/);
  const csv = parseCsv(necText.replace(/^\ufeff/, '')); const header = csv.shift();
  if (!header || new Set(header).size !== header.length) throw new Error('invalid NEC header');
  const cols = ['HIP', 'RA2000', 'DE2000', 'Vmag'].map(key => header.indexOf(key));
  if (cols.some(index => index < 0)) throw new Error('invalid NEC header');
  const usedHips = new Set(manifest.stars.map(s => s.hip));
  const usedNames = new Set(manifest.stars.flatMap(s => [s.name, ...s.aliases]).map(fold));
  const nec = [];
  for (const row of csv) {
    if (row.length !== header.length) throw new Error('malformed NEC row');
    const id = /^HIP\s+(\d+)$/.exec(row[cols[0]].trim());
    const [ra, dec, magnitude] = cols.slice(1).map(i => numeric(row[i].trim()));
    if (!id || ![ra, dec, magnitude].every(Number.isFinite) || ra < 0 || ra >= 360 || Math.abs(dec) > 90 || magnitude < -5 || magnitude > 30) continue;
    const hip = Number(id[1]); if (!Number.isSafeInteger(hip) || hip < 1) continue;
    nec.push({ hip, ra, dec, magnitude });
  }
  const groups = new Map(); const nameGroups = new Map();
  for (const entry of inventory.entries) {
    const key = fold(entry.designation); const group = groups.get(key) ?? []; group.push(entry); groups.set(key, group);
    if (entry.name) { const n = fold(entry.name); const set = nameGroups.get(n) ?? new Set(); set.add(key); nameGroups.set(n, set); }
  }
  const matches = [];
  for (const [key, group] of groups) {
    if (usedNames.has(`,${key}`) || group.some(r => r.name && usedNames.has(fold(r.name))) || group.some(r => r.equinox !== 'ICRS')) continue;
    const fields = group.map(r => lines[r.line - 1].split('#', 1)[0].split(',').map(v => v.trim()));
    if (new Set(fields.map(f => JSON.stringify(f.slice(2, 14)))).size !== 1) continue;
    const source = fields[0]; const hms = source.slice(3, 6).map(numeric); const dms = source.slice(6, 9).map(numeric);
    const magnitude = numeric(source[13]);
    if (![...hms, ...dms, magnitude].every(Number.isFinite) || magnitude < -5 || magnitude > 30
        || hms[0] < 0 || hms[0] >= 24 || hms[1] < 0 || hms[1] >= 60 || hms[2] < 0 || hms[2] >= 60
        || Math.abs(dms[0]) > 90 || dms[1] < 0 || dms[1] >= 60 || dms[2] < 0 || dms[2] >= 60) continue;
    const ra = 15 * (hms[0] + hms[1]/60 + hms[2]/3600);
    const dec = (source[6].startsWith('-') ? -1 : 1) * (Math.abs(dms[0]) + dms[1]/60 + dms[2]/3600);
    if (Math.abs(dec) > 90) continue;
    const raBound = Math.min(180, 2 / 3600 / Math.max(0.000001, Math.cos(dec * rad)));
    const possible = [];
    for (const item of nec) {
      const dra = Math.abs(((item.ra - ra + 540) % 360) - 180);
      if (Math.abs(item.dec - dec) > 1/3600 || dra > raBound || Math.abs(item.magnitude - magnitude) > 0.1) continue;
      const arcsec = separation([ra, dec], [item.ra, item.dec]);
      if (arcsec <= 1) possible.push({ ...item, separationArcsec: arcsec, magnitudeDifference: item.magnitude - magnitude });
    }
    if (possible.length !== 1 || usedHips.has(possible[0].hip)) continue;
    const aliases = [...new Map(group.filter(r => r.name && nameGroups.get(fold(r.name)).size === 1)
      .map(r => [fold(r.name), r.name])).values()];
    matches.push({ designation: group[0].designation, aliases, ...possible[0] });
  }
  const countByHip = new Map();
  for (const row of matches) countByHip.set(row.hip, (countByHip.get(row.hip) ?? 0) + 1);
  const unique = matches.filter(row => countByHip.get(row.hip) === 1);
  const checked = inspectAvailable(hip2Path, unique.map(row => row.hip), inspect);
  const inspected = new Set(checked.rows.map(row => row.hip));
  if (inspected.size !== checked.rows.length || checked.rows.some(r => typeof r.strictEligible !== 'boolean')
      || (qualityPolicy === 'traditional' && checked.rows.some(r => typeof r.traditionalEligible !== 'boolean'))
      || checked.rows.length + checked.missing.length !== unique.length
      || unique.some(row => !inspected.has(row.hip) && !checked.missing.includes(row.hip))) throw new Error('inconsistent HIP2 inspection');
  const quality = new Map(checked.rows.map(row => [row.hip, row]));
  const selected = unique.filter(row => qualityPolicy === 'strict' ? quality.get(row.hip)?.strictEligible : quality.get(row.hip)?.traditionalEligible);
  if (qualityPolicy === 'traditional') {
    const original = inspectAvailable(hip2Path, manifest.stars.map(s => s.hip), inspect);
    const originalHips = new Set(original.rows.map(row => row.hip));
    if (original.missing.length || original.rows.length !== manifest.stars.length || originalHips.size !== original.rows.length
        || manifest.stars.some(star => !originalHips.has(star.hip)) || original.rows.some(row => row.strictEligible !== true)) throw new Error('base catalog is not strictly eligible');
  }
  const stars = selected.map(row => ({ name: `,${row.designation}`, aliases: row.aliases, hip: row.hip,
    identitySource: `Swiss-NEC position <=1 arcsec and magnitude <=0.1; local Swiss ${swissSha256}; NEC ${necSha256}; identity unreviewed`,
    qualityDecision: `Local candidate: ${quality.get(row.hip).strictEligible ? 'strict' : 'traditional'} HIP2 filter; no individual scientific review`,
    ...(qualityPolicy === 'traditional' ? { qualityTier: quality.get(row.hip).strictEligible ? 'strict' : 'traditional' } : {}),
    magnitude: row.magnitude, magnitudeSource: `NEC Vmag snapshot SHA256 ${necSha256}; provenance unverified`,
    radialVelocityKmS: null, radialVelocityMissingReason: 'HIP2 has no radial velocity; local candidate assumes zero km/s' }));
  const originalStars = qualityPolicy === 'traditional' ? manifest.stars.map(star => ({ ...star, qualityTier: 'strict' })) : manifest.stars;
  const output = { ...manifest, reviewer: `${manifest.reviewer}; positional additions require individual review`, qualityPolicy, stars: [...originalStars, ...stars] };
  prepareStarCatalog(output);
  return { manifest: output, report: { schema: 'virgostars-positional-candidates/1', status: 'REVIEW_REQUIRED',
    swissSha256, necSha256, hip2Sha256, oneToOneCandidates: unique.length, missingHipIds: checked.missing,
    qualityPolicy, selectedCount: selected.length, selected: selected.map(row => ({ ...row,
      strictEligible: quality.get(row.hip).strictEligible, traditionalEligible: quality.get(row.hip).traditionalEligible ?? null })) } };
}

async function main() {
  const [manifestPath, swissPath, necPath, hipPath, outputPath, reportPath, qualityPolicy = 'strict', ...extra] = process.argv.slice(2);
  const paths = [manifestPath, swissPath, necPath, hipPath, outputPath, reportPath];
  if (!paths.every(Boolean) || extra.length || new Set(paths.map(p => resolve(p))).size !== paths.length) throw new Error('usage: <manifest> <sefstars> <nec.csv> <hip2> <new-manifest> <new-report> [strict|traditional]');
  if (existsSync(outputPath) || existsSync(reportPath)) throw new Error('output already exists');
  const inputs = paths.slice(0, 4).map(path => readFileSync(path));
  if (inputs.some((b, i) => b.length > (i === 3 ? 100_000_000 : 8_000_000))) throw new Error('input exceeds size limit');
  if ([1, 2].some(i => !Buffer.from(inputs[i].toString('utf8'), 'utf8').equals(inputs[i]))) throw new Error('catalog input is not UTF-8');
  const result = extendByPosition({ manifest: JSON.parse(inputs[0]), swissText: inputs[1].toString('utf8'),
    necText: inputs[2].toString('utf8'), hip2Sha256: hash(inputs[3]), hip2Path: hipPath, qualityPolicy });
  for (let i = 0; i < inputs.length; i++) if (hash(readFileSync(paths[i])) !== hash(inputs[i])) throw new Error('input changed');
  writeFileSync(outputPath, `${JSON.stringify(result.manifest, null, 2)}\n`, { flag: 'wx' });
  writeFileSync(reportPath, `${JSON.stringify(result.report, null, 2)}\n`, { flag: 'wx' });
  console.log(`${result.report.selectedCount}/${result.report.oneToOneCandidates} ${qualityPolicy} positional candidates added; identity review required.`);
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main().catch(error => { console.error(`Positional extension failed: ${error.message}`); process.exitCode = 1; });
}
