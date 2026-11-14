# DE441 record, granule and velocity cross-feature CI — 2026-11-14

**Status: INTEGRATION-ONLY; DO NOT MERGE THIS DUPLICATE BRANCH.**
This review aid combines source and deterministic synthetic tests from original
independently reviewable draft PRs. The original PRs remain the authoritative
units to integrate one at a time after owner review. This branch must not
replace or silently merge those PRs.

## Pinned ancestry and source

- Base snapshot: existing [integration-only PR #198](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/198),
  commit `a1b4bf119763c3e62ffd23f1a5884db44beb5a6f`,
  already carrying #174, #182, #187 and #193 production/test changes.
- Latest [PR #187](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/187)
  test fix: `024d11fface84f519001a5a9a4903ba0ab021fa9`;
  `de441_granule_resolution.rs` has **eight** synthetic test functions.
- [PR #178](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/178):
  `daeb6d16c7c5adee09e2979a139b2ecffcaafc54`;
  carries its conditional derivative scaling and three synthetic velocity
  tests (overflow avoidance, truly unrepresentable velocity, and preserving
  the smallest subnormal when record width is two days).

The previous PR #198 snapshot had a now-obsolete velocity scaling path,
and its copied wide-record test expected exact binary64 equality even where
two mathematically equivalent operation orders differ by a single
subnormal ULP. This integration branch adds the PR #178 runtime fix and
tests, and replaces the copied PR #187 test with the exact 1-ULP-aware
version from its latest pinned head.

The combined synthetic suite now includes **20 original focused tests**:
#174 (2), #182 (4), #187 (8), #193 (3), #178 (3).
This count refers to the five cited PR test files only; additional
workspace tests remain present. This branch contains no real ephemeris
kernel, independent NASA/JPL reference vector or external dependency.

## Combined invariants

1. A classic little-endian JPL DE441 coefficient-record cache never
   allocates above the defensive 16-MiB engineering limit.
2. Declared subgranule TDB Julian-day boundaries are distinguishable in
   binary64 at the first and last principal record.
3. Original TDB JD selects the principal record by direct boundary
   comparison. Exact interior boundary belongs to the following record.
4. TDB JD selects the in-record Chebyshev granule by direct comparison to
   representable boundaries; the final coverage endpoint stays inclusive.
5. Normalized Chebyshev abscissa divides by interval width before doubling,
   avoiding spurious overflow for very wide finite records.
6. Chebyshev derivative scaling preserves finite km/day velocity where
   valid and avoids losing the smallest subnormal at two-day width.

**Scientific interpretation:** input time is one binary64 **Julian Date TDB**;
output is **geometric ICRF Cartesian state** in km and km/day,
**target relative to selected center** (outer planets are system
barycenters, Earth/Moon have explicit mass-ratio reconstruction).
No TT/UTC transformation, aberration, light-time, precession, nutation,
topocentric correction or SPK `.bsp` support is introduced.

## CI verification and provenance

The read-only branch CI should pass on this exact head:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --release
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The repo CI also checks rustdoc, Windows and macOS native Rust tests.
This verifies *compatibility of synthetic arithmetic and parser changes*,
not real-kernel JPL/NAIF/Horizons scientific accuracy. Never report
production readiness from these tests.

No `main`, `dev/ephemeris`, or original draft branch was merged or
rewritten to create this crosscheck. No packages/releases or deployment.
Preserve CPAL-1.0 Exhibit A headers and required **Powered by VirgoStars**
attribution, https://www.virgostars.it/.
