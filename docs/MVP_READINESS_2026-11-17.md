# MVP readiness checkpoint — 2026-11-17

**Current update (2026-11-18):** the owner selected Swiss ecliptic
compatibility and a 1″ sampled positional tolerance. See the
[finite planetary closure checklist](MVP1_PIANETI_CHIUSURA_2026-11-18.md).
The rest of this document is a dated checkpoint, not the current acceptance decision.

**Priority update:** the owner has paused stellar catalog work. See the
[MVP 1 priorities and acceptance proposal](MVP1_NEXT_PRIORITIES_2026-11-17.md)
and [stellar pause checkpoint](STAR_CATALOG_PAUSED_2026-11-17.md). The ordered
list below is retained as historical context, not the current work order.

**Follow-up:** the previously missing local catalog has been recovered and
extended; see [stellar recovery](STAR_CATALOG_RECOVERY_2026-11-17.md). The
absence reported in the historical table below applied to the earlier checkout
at the time of that review.

**Status: experimental integration candidate; neither a production server
replacement nor a scientifically accepted release.** This checkpoint
distinguishes current source, previous local evidence and missing acceptance;
it does not promote any capability. The exact requirements and tolerances for
an accepted MVP still need a product decision, so a percentage would be
misleading.

## What is implemented and what has actually been checked

| Area | Status and evidence | Remaining limit |
| --- | --- | --- |
| Rust planetary core | Classic externally supplied DE441, apparent geocentric positions and rates; experimental long-term path sampled over astronomical years 0–3000. | Ordinary time/orientation paths have narrower guards. Sampling is not a continuous accuracy guarantee; Mars–Pluto are system barycenters. |
| Planet reference comparisons | [3,010 Horizons ICRF positions and 12,040 Swiss ecliptic seasonal positions](PLANET_VALIDATION_0_3000_2026-11-17.md); Horizons and engine share DE441. [Station signs](STATION_WINDOWS_AND_FUTURE_UT1_2026-11-17.md) were checked at selected times. | Independent review, reference conventions, approved tolerances and reported-rate differences remain open. Approximate TT/TDB conversion and terrestrial-time uncertainty are separate. |
| SPK/NAIF | Real-kernel [type-2](SPK_NAIF_LOCAL_COMPARISON_2026-11-16.md) and [type-3/multi-kernel](SPK_NAIF_TYPE3_MULTI_KERNEL_2026-11-16.md) geometric comparisons were recorded locally against CSPICE. | Limited matrices, not general SPICE compatibility, apparent corrections, all frames or scientific acceptance; source and tolerance review remain. |
| Stars | The caller-supplied manifest and lookup, including explicit source astrometry and warnings, are implemented experimentally. The [last retained checkpoint](STAR_COVERAGE_PHYSICAL_IDENTITIES_2026-11-17.md) records 1,062 candidates, 687/729 names and 1,065/1,113 designations. | That is historical evidence, not current verification: the specific local manifest and saved coverage report were absent from this checkout during this review. The recorded gaps were 42 names and 48 designations; object identity, missing magnitudes, assumed radial velocities, stellar dynamics and observational accuracy need review. |
| Backend | An explicit Node facade and isolated backend smoke/diagnostic work exist; see [integration checkpoint](MVP1_RECONCILIATION_2026-11-16.md). | Representative full backend parity, data provisioning, acceptance and rollout/rollback are open. The owner requires complete Swiss star coverage before replacing the main server. |
| Desktop | The separately inspected Desktop checkout `15d7a4a` contains a native provider, pins core commit `6835e00`, and enables its Cargo feature by default. Its code accepts calendar years 1900–2033, subject to available time data. | The Desktop [provider record](https://github.com/Cesare-ml/virgostars-desktop/blob/15d7a4a7293f1c05cda758c5f064261bd1393c16/docs/VIRGOSTARS_EPHEMERIS_PROVIDER.md) reports prior local tests and an unsigned, unpublished installer; they were not rerun in this checkpoint. The long-term experimental path is not activated there. Source integration is not release. |

Desktop's data-bundle document records an installer tested locally with a
limited DE441 slice. The repository's ordinary Tauri configuration has
`bundle.active=false`; special packaging and the recorded installer must not
be conflated with the ordinary build or with publication.

## Time and long-range scope

DE441 data coverage is not the same as apparent-position, terrestrial-time or
desktop coverage. The year 0–3000 diagnostic uses TDB, with year 0 in
astronomical numbering; historical and future UT1 require an explicit ΔT model
or data. UT1, UTC and historical local time are distinct. The
[future illustration](STATION_WINDOWS_AND_FUTURE_UT1_2026-11-17.md) uses an
older uncertainty scenario, not a modern confidence interval or product
guarantee. Years 3001–5000 have not received the same 0–3000 dense validation.

## Catalog field-readiness diagnostic

`scripts/verify_swiss_coverage.mjs` continues to gate only exact name and
designation coverage. Its additive `fieldCoverage` report lists matched names
and designations with null magnitude or an explicitly assumed-zero radial
velocity, plus object counts. A matched entry with null magnitude cannot
answer `swe_fixstar_mag`; a supplied numeric zero radial velocity is distinct
from a missing value assumed zero. Unmatched strings remain in the separate
missing-lookup lists. The diagnostic neither opens HIP2 nor verifies runtime
loading, identity, data permissions or numerical accuracy; its exit code and
`COMPLETE_LOOKUP_COVERAGE_ONLY` meaning are unchanged.

## Prioritized remaining acceptance work

1. Continue from the recovered local stellar manifest and its updated
   hash-bound coverage/field diagnostic; resolve remaining names, designations
   and nonstellar/ambiguous objects with correct identity.
2. Agree on measurable product tolerances and review the planetary, station,
   SPK and time evidence, including independent references and reported-rate
   convention differences. Validate the target date range rather than infer it
   from DE441 alone.
3. Run representative end-to-end backend comparisons with all required bodies,
   flags, houses, errors and data provisioning; resolve discrepancies before
   any main-server change. Prepare and verify rollback.
4. For Desktop, separately validate the pinned version, supported dates and
   data bundle on each target and complete distribution/signing decisions.
   Do not describe a locally tested unsigned installer as a published release.

The local test runs for a change must be recorded on its pull request; the
scientific evidence above is historical and does not assert a fresh replay.
