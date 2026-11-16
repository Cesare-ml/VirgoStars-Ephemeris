// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Caller-owned offline metadata only. An audit is not catalog approval.
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { auditStarCatalogCoverage } = require('../star-catalog.js');

try {
  const [manifestPath, requestsPath, ...extra] = process.argv.slice(2);
  if (!manifestPath || !requestsPath || extra.length) {
    throw new Error('usage: node audit-star-manifest-coverage.mjs <manifest.json> <requested-names.json>');
  }
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const requests = JSON.parse(readFileSync(requestsPath, 'utf8'));
  console.log(JSON.stringify(auditStarCatalogCoverage(manifest, requests), null, 2));
} catch (error) {
  console.error(`Star manifest coverage audit failed: ${error.message}`);
  process.exitCode = 1;
}
