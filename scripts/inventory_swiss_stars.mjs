// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-17): inventory an external Swiss catalog for local review.
// Names and designations are lookup leads, not verified object identities.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const fold = value => value.toLocaleLowerCase('en-US');
const sha = data => createHash('sha256').update(data).digest('hex');

export function inventorySwissStars(text) {
  if (typeof text !== 'string' || !text.includes('sefstars.txt')) throw new Error('unrecognized catalog header');
  const entries = [];
  const names = new Map();
  const designations = new Map();
  for (const [index, line] of text.split(/\r?\n/).entries()) {
    const content = line.split('#', 1)[0].trim();
    if (!content) continue;
    const fields = content.split(',').map(s => s.trim());
    if (fields.length < 14 || fields.length > 16 || !fields[1] || !['ICRS', '2000', '1950'].includes(fields[2])) {
      throw new Error(`malformed catalog row at line ${index + 1}`);
    }
    if (fields.slice(3, 14).some(value => value === '' || !Number.isFinite(Number(value)))) {
      throw new Error(`invalid numeric field at line ${index + 1}`);
    }
    const entry = { line: index + 1, name: fields[0] || null, designation: fields[1], equinox: fields[2] };
    entries.push(entry);
    for (const [map, key] of [[names, entry.name], [designations, entry.designation]]) {
      if (key === null) continue;
      const k = fold(key);
      const item = map.get(k) ?? { value: key, lines: [] };
      item.lines.push(entry.line);
      map.set(k, item);
    }
  }
  if (!entries.length) throw new Error('empty catalog');
  return {
    schema: 'virgostars-swiss-star-inventory/1', status: 'REVIEW_REQUIRED',
    notes: 'Catalog entries include aliases, duplicates, nonstellar objects and test records. Designations are not proven unique objects.',
    counts: { records: entries.length, namedRecords: entries.filter(e => e.name !== null).length,
      uniqueNames: names.size, uniqueDesignations: designations.size,
      repeatedNames: [...names.values()].filter(v => v.lines.length > 1).length,
      repeatedDesignations: [...designations.values()].filter(v => v.lines.length > 1).length },
    entries, names: [...names.values()], designations: [...designations.values()],
  };
}

function main() {
  const [inputPath, reportPath, namesPath, ...extra] = process.argv.slice(2);
  if (!inputPath || !reportPath || !namesPath || extra.length) throw new Error('usage: node inventory_swiss_stars.mjs <sefstars.txt> <new-report.json> <new-names.json>');
  const paths = [inputPath, reportPath, namesPath].map(p => resolve(p));
  if (new Set(paths).size !== 3) throw new Error('paths must differ');
  if (existsSync(reportPath) || existsSync(namesPath)) throw new Error('output already exists');
  const stat = statSync(inputPath);
  if (!stat.isFile() || stat.size > 8_000_000) throw new Error('invalid or oversized input');
  const bytes = readFileSync(inputPath);
  if (bytes.length > 8_000_000) throw new Error('input exceeded size limit');
  const report = inventorySwissStars(bytes.toString('utf8'));
  report.inputSha256 = sha(bytes);
  writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' });
  writeFileSync(namesPath, `${JSON.stringify(report.names.map(v => v.value), null, 2)}\n`, { flag: 'wx' });
  console.log(JSON.stringify({ sha256: report.inputSha256, ...report.counts }));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`Inventory failed: ${error.message}`); process.exitCode = 1; }
}
