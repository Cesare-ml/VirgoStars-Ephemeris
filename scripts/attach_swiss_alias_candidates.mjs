// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification 2026-11-17: local candidate aliases, not identity approval.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { inventorySwissStars } from './inventory_swiss_stars.mjs';

const fold = value => value.trim().toLocaleLowerCase('en-US');
const digest = bytes => createHash('sha256').update(bytes).digest('hex');

export function attachAliases(manifest, swissText, swissHash) {
  if (typeof swissText !== 'string' || !/^[a-f0-9]{64}$/.test(swissHash)
      || digest(Buffer.from(swissText, 'utf8')) !== swissHash) throw new Error('Swiss catalog hash mismatch');
  const require = createRequire(import.meta.url);
  require('../bindings/node/star-catalog.js').prepareStarCatalog(manifest);
  const inventory = inventorySwissStars(swissText);
  const lines = swissText.split(/\r?\n/);
  const entries = inventory.entries.map(entry => {
    const fields = lines[entry.line - 1].split('#', 1)[0].split(',').map(s => s.trim());
    // Require exact agreement of frame and all astrometric and magnitude fields.
    return { ...entry, signature: JSON.stringify(fields.slice(2, 14)) };
  });
  const byName = new Map(); const byDesignation = new Map();
  const add = (map, key, entry) => { const k = fold(key); const rows = map.get(k) ?? []; rows.push(entry); map.set(k, rows); };
  for (const entry of entries) {
    if (entry.name) add(byName, entry.name, entry);
    add(byDesignation, entry.designation, entry);
  }
  const occupied = new Set(manifest.stars.flatMap(s => [s.name, ...s.aliases]).map(fold));
  const added = []; const ambiguous = [];
  const stars = manifest.stars.map(star => {
    const rows = byName.get(fold(star.name)) ?? [];
    const designations = new Set(rows.map(row => fold(row.designation)));
    if (designations.size !== 1) { ambiguous.push(star.name); return { ...star }; }
    const group = byDesignation.get([...designations][0]);
    const signatures = new Set(group.map(row => row.signature));
    if (signatures.size !== 1) { ambiguous.push(star.name); return { ...star }; }
    const candidates = [`,${group[0].designation}`, ...group.flatMap(row => row.name ? [row.name] : [])];
    const aliases = [...star.aliases];
    for (const value of candidates) {
      const key = fold(value);
      // A traditional name appearing under several designations is unsafe.
      if (!value.startsWith(',') && new Set((byName.get(key) ?? []).map(row => fold(row.designation))).size !== 1) continue;
      if (occupied.has(key)) continue;
      occupied.add(key); aliases.push(value); added.push({ alias: value, canonicalName: star.name, hip: star.hip });
    }
    return { ...star, aliases, ...(aliases.length > star.aliases.length ? {
      aliasSource: `Swiss catalog exact-row candidate grouping; SHA256 ${swissHash}; identity not independently verified`,
    } : {}) };
  });
  const output = { ...manifest, reviewer: `${manifest.reviewer}; Swiss alias identity review pending`,
    sourceHashes: { ...manifest.sourceHashes, swissCatalogSha256: swissHash }, stars };
  require('../bindings/node/star-catalog.js').prepareStarCatalog(output);
  return { manifest: output, report: { schema: 'virgostars-swiss-alias-candidates/1', status: 'REVIEW_REQUIRED',
    swissCatalogSha256: swissHash, addedCount: added.length, ambiguousCount: ambiguous.length, added, ambiguous } };
}

function main() {
  const [manifestPath, swissPath, outputPath, reportPath, ...extra] = process.argv.slice(2);
  if (!manifestPath || !swissPath || !outputPath || !reportPath || extra.length) throw new Error('usage: node attach_swiss_alias_candidates.mjs <manifest.json> <sefstars.txt> <new-manifest.json> <new-report.json>');
  if (new Set([manifestPath, swissPath, outputPath, reportPath].map(p => resolve(p))).size !== 4) throw new Error('paths must differ');
  if (existsSync(outputPath) || existsSync(reportPath)) throw new Error('output already exists');
  const manifestBytes = readFileSync(manifestPath); const swissBytes = readFileSync(swissPath);
  if (manifestBytes.length > 8_000_000 || swissBytes.length > 8_000_000) throw new Error('input exceeds size limit');
  const result = attachAliases(JSON.parse(manifestBytes.toString('utf8')), swissBytes.toString('utf8'), digest(swissBytes));
  if (digest(readFileSync(manifestPath)) !== digest(manifestBytes) || digest(readFileSync(swissPath)) !== digest(swissBytes)) throw new Error('input changed');
  writeFileSync(outputPath, `${JSON.stringify(result.manifest, null, 2)}\n`, { flag: 'wx' });
  writeFileSync(reportPath, `${JSON.stringify(result.report, null, 2)}\n`, { flag: 'wx' });
  console.log(`${result.report.addedCount} candidate aliases added; ${result.report.ambiguousCount} canonical names need review.`);
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`Alias preparation failed: ${error.message}`); process.exitCode = 1; }
}
