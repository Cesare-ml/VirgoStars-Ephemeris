# Local MVP star catalog and backend bridge — 2026-11-17

**Status: working local MVP candidate, not a production release.** The project
owner requested a smaller MVP catalog from the already tested stars. This
checkpoint supplies a reproducible 40-star local manifest and connects it to
the experimental backend branch. It does not imply that the full historical
106-name list is supported or that the star catalog has been scientifically
certified.

## Local catalog

`scripts/build_mvp_star_manifest.mjs` reconstructs the exact-match candidates
from the historical 2022 IAU-CSN snapshot, verifies the audit and source hashes,
replays the strict HIP2 quality filter and builds a `virgostars-star-manifest/1`.
This is the same 40-name strict subset used in the earlier
[120-case backend diagnostic](MVP1_STAR_BACKEND_COMPARISON_2026-11-16.md).
Each has a V magnitude from that snapshot. HIP2 contains no radial velocity;
the manifest explicitly assumes **0 km/s for all 40**. That assumption is
exposed in each experimental backend response. Variable or multiple stars,
long date ranges and missing photometry remain scientific limitations.

The local result covers 40 of the backend's 106 requested names; the remaining
66 are not silently invented or substituted. The source snapshot, HIP2 file and
generated manifest remain external to Git. Their source and hash are recorded;
local preparation does not establish redistribution rights or individual
scientific approval. The manifest records this as an MVP evaluation, not as an
independent review.

```powershell
node scripts/build_mvp_star_manifest.mjs <iau-candidates.json> <iau-quality.json> `
  <iau-snapshot.txt> <hip2.dat> <new-manifest.json>
node bindings/node/scripts/audit-star-manifest-coverage.mjs <new-manifest.json> <requested-names.json>
```

The tool requires explicit caller-supplied files and will not overwrite the
output. The local manifest is
`VirgoStarsData/star-catalog-review/2026-11-17/mvp1-star-manifest-strict-v2.json`,
SHA-256 `8399e5a2e5443d80bc11cf48cb4541ba5acc08f5d7d91a8d615223041837a550`.
Its embedded source hashes bind it to the local HIP2, snapshot and reports;
hashes establish byte identity, not scientific correctness or source permission.

## Backend and verification

Backend branch `codex/ephemeris-house-diagnostics-20261116` can explicitly load
the manifest and HIP2 file, check their hashes, and serve star and natal-with-
stars requests through its loopback-only experimental server. It does not mount
the experimental routes in the existing public server. Detailed startup and
request examples are in the backend branch's
`docs/backend/EPHEMERIS_NATAL_BRIDGE_2026-11-16.md`.

On Windows with the real local DE441, HIP2, manifest, addon and house modules,
20 focused backend tests passed with no skips. These include HTTP requests for
all 40 stars, a single batch containing all 40, a combined natal chart with 40
stars, the invalid-input boundaries, and natal polar diagnostics. Three manifest
tests passed, including byte-identical reconstruction and refusal to overwrite.
These are local execution and wiring checks, not independent scientific
validation, CI, staging or production deployment.

The backend runtime suite passed 79/79. Its full astrology suite is not green:
the candidate and its baseline had the same 57 failing test names; the local
candidate also skipped six optional tests when the real-data environment was
not configured for that suite. The focused real-data tests above had no skips.

Subsequent backend work adds an explicit extended profile: 15 kernel elements
including Chiron and four derived opposite points, plus the same legacy star
conjunction service with the 40-star subset. The existing `/stelle` controller
can receive this provider explicitly and has been tested locally with a fake
COW repository and logger. The production server and real PostgreSQL path have
not been switched or verified. See the backend branch's bridge documentation
for startup, coverage, tests and limitations. Broader parity, packaging and
release review remain open.
