// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Local, read-only numerical HIP2 quality screening; does not approve stars.
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const { inspectHipparcosQuality } = require('../index.js');
const hash = (data) => createHash('sha256').update(data).digest('hex');

// Missing individual HIP entries are isolated without hiding other file errors.
export function inspectAvailable(path, hips, inspect) {
  if (!hips.length) return { rows: [], missing: [] };
  try {
    return { rows: inspect(path, hips), missing: [] };
  } catch (error) {
    if (error.code !== 'E_CATALOG_FILE' || !String(error.message).includes('hip2 field HIP not found is out of range')) throw error;
    if (hips.length === 1) return { rows: [], missing: hips };
    const middle = Math.floor(hips.length / 2);
    const left = inspectAvailable(path, hips.slice(0, middle), inspect);
    const right = inspectAvailable(path, hips.slice(middle), inspect);
    return { rows: [...left.rows, ...right.rows], missing: [...left.missing, ...right.missing] };
  }
}

function main() {
  const [reportPath, hip2Path, outputPath, ...extra] = process.argv.slice(2);
  if (!reportPath || !hip2Path || !outputPath || extra.length) {
    throw new Error('usage: node audit-star-quality.mjs <candidate-report.json> <hip2.dat> <new-output.json>');
  }
  if ([reportPath, hip2Path].some((p) => resolve(p) === resolve(outputPath))) throw new Error('output must differ from inputs');
  const reportStat = statSync(reportPath);
  const hip2Stat = statSync(hip2Path);
  if (!reportStat.isFile() || !hip2Stat.isFile() || reportStat.size > 4_000_000 || hip2Stat.size > 100_000_000) {
    throw new Error('input is not a regular file or exceeds size limit');
  }
  const reportBytes = readFileSync(reportPath);
  if (reportBytes.length > 4_000_000) throw new Error('candidate report exceeds size limit');
  const report = JSON.parse(reportBytes.toString('utf8'));
  if (report.schema !== 'virgostars-iau-candidates/1' || report.status !== 'REVIEW_REQUIRED' || !Array.isArray(report.results)) {
    throw new Error('invalid candidate report');
  }
  const selections = [];
  for (const row of report.results) {
    if (!row || typeof row.requestedName !== 'string' || !row.requestedName.trim() || !Array.isArray(row.candidates)) {
      throw new Error('malformed candidate report row');
    }
    if (row.status === 'UNMATCHED' && row.candidates.length === 0) continue;
    if (row.status === 'AMBIGUOUS' && row.candidates.length > 1) continue;
    if (row.status !== 'CANDIDATE_REVIEW_REQUIRED' || row.candidates.length !== 1 || !row.candidates[0]) {
      throw new Error('inconsistent candidate status');
    }
    const hip = row.candidates[0].hip;
    if (hip === null) continue;
    if (!Number.isSafeInteger(hip) || hip <= 0 || hip > 0xffffffff) throw new Error('invalid HIP candidate');
    selections.push(row);
  }
  const hips = selections.map((r) => r.candidates[0].hip);
  if (!hips.length || new Set(hips).size !== hips.length) throw new Error('no unique HIP candidates; resolve duplicates first');
  const before = hash(readFileSync(hip2Path));
  const { rows: quality, missing } = inspectAvailable(hip2Path, hips, inspectHipparcosQuality);
  const byHip = new Map(quality.map(row => [row.hip, row]));
  if (byHip.size !== quality.length || quality.length + missing.length !== hips.length
      || hips.some(hip => !byHip.has(hip) && !missing.includes(hip))) throw new Error('inconsistent HIP2 inspection results');
  if (hash(readFileSync(hip2Path)) !== before || hash(readFileSync(reportPath)) !== hash(reportBytes)) {
    throw new Error('input changed during inspection');
  }
  const output = { schema: 'virgostars-hip2-quality-audit/1', status: 'REVIEW_REQUIRED',
    candidateReportSha256: hash(reportBytes), hip2Sha256: before,
    note: 'Numerical policy eligibility is not permission to use a star.', missingHipIds: missing,
    results: selections.filter(r => byHip.has(r.candidates[0].hip))
      .map(r => ({ requestedName: r.requestedName, ...byHip.get(r.candidates[0].hip) })) };
  writeFileSync(outputPath, `${JSON.stringify(output, null, 2)}\n`, { flag: 'wx' });
  console.log(`${quality.length} candidates inspected, ${missing.length} absent from HIP2: ${quality.filter((q) => q.strictEligible).length} strict, ${quality.filter((q) => q.traditionalEligible).length} traditional filter; review required.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(`HIP2 quality audit failed: ${error.message}`); process.exitCode = 1; }
}
