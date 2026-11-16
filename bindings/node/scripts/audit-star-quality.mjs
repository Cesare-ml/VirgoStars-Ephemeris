// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Local, read-only numerical HIP2 quality screening; does not approve stars.
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';

const require = createRequire(import.meta.url);
const { inspectHipparcosQuality } = require('../index.js');
const hash = (data) => createHash('sha256').update(data).digest('hex');

try {
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
  const quality = inspectHipparcosQuality(hip2Path, hips);
  if (hash(readFileSync(hip2Path)) !== before || hash(readFileSync(reportPath)) !== hash(reportBytes)) {
    throw new Error('input changed during inspection');
  }
  const output = { schema: 'virgostars-hip2-quality-audit/1', status: 'REVIEW_REQUIRED',
    candidateReportSha256: hash(reportBytes), hip2Sha256: before,
    note: 'Numerical policy eligibility is not permission to use a star.',
    results: selections.map((r, i) => ({ requestedName: r.requestedName, ...quality[i] })) };
  writeFileSync(outputPath, `${JSON.stringify(output, null, 2)}\n`, { flag: 'wx' });
  console.log(`${quality.length} candidates inspected: ${quality.filter((q) => q.strictEligible).length} strict, ${quality.filter((q) => q.traditionalEligible).length} traditional filter; review required.`);
} catch (error) {
  console.error(`HIP2 quality audit failed: ${error.message}`);
  process.exitCode = 1;
}
