// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Offline identification workbench for the historical fixed-width IAU-CSN format.
// It does not review identities, data rights, photometry, or catalog quality.
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const fold = (name) => name.trim().toLocaleLowerCase('en-US');
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

export function findCandidates(catalog, requestedNames, sourceUrl) {
  if (typeof catalog !== 'string' || !catalog.includes('#Name/ASCII') || !catalog.includes('Dec(J2000)')) {
    throw new Error('unrecognized IAU-CSN header');
  }
  if (!Array.isArray(requestedNames) || !requestedNames.length || requestedNames.length > 2000) {
    throw new Error('requested names must be a nonempty array of at most 2000 strings');
  }
  if (typeof sourceUrl !== 'string' || !/^https:\/\//.test(sourceUrl)) throw new Error('a source HTTPS URL is required');
  const wanted = new Set();
  for (const name of requestedNames) {
    if (typeof name !== 'string' || !name.trim() || wanted.has(fold(name))) throw new Error('invalid or duplicate requested name');
    wanted.add(fold(name));
  }
  const byName = new Map();
  let recordCount = 0;
  for (const line of catalog.split(/\r?\n/)) {
    if (!line.trim() || line.startsWith('#') || line.startsWith('$')) continue;
    const name = line.slice(0, 18).trim();
    const hip = line.slice(90, 97).trim();
    const mag = line.slice(81, 86).trim();
    const band = line.slice(87, 90).trim();
    const date = line.slice(126, 136).trim();
    if (!name || !/^(_|\d+)$/.test(hip) || !/^(_|[-+]?\d+(?:\.\d+)?)$/.test(mag)
      || !band || !/^\d{4}-\d{2}-\d{2}$/.test(date)) throw new Error(`malformed IAU-CSN record at data row ${recordCount + 1}`);
    recordCount++;
    const candidate = { iauName: name, hip: hip === '_' ? null : Number(hip),
      component: line.slice(65, 70).trim(), magnitude: mag === '_' ? null : Number(mag),
      magnitudeBand: band === '_' ? null : band, iauApprovalDate: date };
    const key = fold(name);
    const matches = byName.get(key) ?? [];
    matches.push(candidate);
    byName.set(key, matches);
  }
  if (!recordCount) throw new Error('IAU-CSN catalog contains no records');
  const results = requestedNames.map((name) => {
    const candidates = byName.get(fold(name)) ?? [];
    return { requestedName: name.trim(), status: candidates.length === 0 ? 'UNMATCHED'
      : candidates.length === 1 ? 'CANDIDATE_REVIEW_REQUIRED' : 'AMBIGUOUS', candidates };
  });
  const selectedByHip = new Map();
  for (const result of results) for (const candidate of result.candidates) {
    if (candidate.hip !== null) {
      const names = selectedByHip.get(candidate.hip) ?? [];
      names.push(result.requestedName);
      selectedByHip.set(candidate.hip, names);
    }
  }
  for (const result of results) for (const candidate of result.candidates) {
    const names = selectedByHip.get(candidate.hip) ?? [];
    candidate.sharedHipWith = names.filter((name) => name !== result.requestedName);
  }
  return { schema: 'virgostars-iau-candidates/1', status: 'REVIEW_REQUIRED', sourceUrl,
    catalogRecordCount: recordCount, requestedCount: results.length,
    exactNameCandidates: results.filter((r) => r.candidates.length > 0).length,
    unmatched: results.filter((r) => r.status === 'UNMATCHED').length, results };
}

function main() {
  const [catalogPath, requestedPath, sourceUrl, outputPath, ...rest] = process.argv.slice(2);
  if (!catalogPath || !requestedPath || !sourceUrl || !outputPath || rest.length) {
    throw new Error('usage: node scripts/iau_star_candidates.mjs <IAU-CSN.txt> <names.json> <https-source-url> <new-output.json>');
  }
  if ([catalogPath, requestedPath].some((p) => resolve(p) === resolve(outputPath))) throw new Error('output must differ from inputs');
  const catalogStat = statSync(catalogPath);
  const namesStat = statSync(requestedPath);
  if (!catalogStat.isFile() || !namesStat.isFile() || catalogStat.size > 4_000_000 || namesStat.size > 1_000_000) {
    throw new Error('input is not a regular file or exceeds size limit');
  }
  const catalogBytes = readFileSync(catalogPath);
  const namesBytes = readFileSync(requestedPath);
  if (catalogBytes.length > 4_000_000 || namesBytes.length > 1_000_000) throw new Error('input exceeded size limit during read');
  const report = findCandidates(catalogBytes.toString('utf8'), JSON.parse(namesBytes.toString('utf8')), sourceUrl);
  report.catalogSha256 = digest(catalogBytes);
  report.requestedNamesSha256 = digest(namesBytes);
  writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' });
  console.log(`${report.exactNameCandidates}/${report.requestedCount} exact-name candidates; ${report.unmatched} unmatched; all require review.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`IAU candidate report failed: ${error.message}`); process.exitCode = 1; }
}
