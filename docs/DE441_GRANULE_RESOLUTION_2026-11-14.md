# Classic DE441: representable Chebyshev granule boundaries

**Modification:** 2026-11-14 (original Rust reader; CPAL-1.0).

## Problem and fail-closed preflight

The classic JPL DE441 reader operates on a **single binary64 TDB Julian date**.
The header declares a positive principal-record step (TDB days) and
a number of Chebyshev granules per series. A finite, positive step does
**not** guarantee that all its subrecord boundaries can be represented by
distinct binary64 Julian dates at the declared epoch. Adjacent granules
may then map to the same timestamp, making granule selection ambiguous.

`De441::new` now checks every nonempty series' granule boundaries at
the earliest and latest principal records against strictly increasing
binary64 dates. A zero-width granule after division, a collapsed
neighboring boundary, or nonfinite boundary returns
`De441Error::InvalidFile("coefficient granule dates exceed f64 JD resolution")`.
The original declared file-length, pointer and record-date checks remain.

**Runtime granule selection:** division followed by `floor()` can round
a date that is exactly equal to a representable inner boundary to the
*previous* index, for example near `JD = 1 + 1/3` with three granules
per day. The reader now binary-searches the same representable inner
boundaries as the constructor preflight. An exact inner boundary selects
the following granule; the inclusive final coverage date selects the
final granule. The search needs at most seven comparisons for the
supported 100 granules per record and does not read additional bytes.

**Wide finite record windows:** normalization now divides the
TDB-day offset by its positive, representable granule width before
multiplying by two. This avoids spurious intermediate overflow in
`2 * (jd - granule_start) / width - 1` when `width` is very large.
It does not change the interpolation polynomial or the physical
units. This is an IEEE-754 robustness regression, not a claim that
the official DE441 kernel contains such extreme intervals.

**Integration coordination (2026-11-14):** draft
[PR #193](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/193)
independently fixes selection of *principal coefficient records*
against the original TDB Julian date. This PR #187 fixes selection of
*granules inside a selected record* and checks granule resolution
during header parsing. Both edit `de441.rs`; integrate them serially
onto a current `dev/ephemeris` and re-run exact-head CI so neither
boundary-selection fix is lost. Draft
[PR #178](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/178)
independently fixes large-derivative **velocity** normalization;
preserve that change too. No automatic merge is performed.

The check is cheap (at most 13 series × 2 records × 100 granules).
It does not examine every internal principal record or validate
coefficient payloads, file authenticity, physical accuracy, or all
possible interpolation-roundoff effects. This is a **necessary
representability guard**, not a proof of exact epoch arithmetic.

## Synthetic tests

`cargo test --workspace --test de441_granule_resolution` exercises:

- A positive minimum subnormal TDB-day step with two granules, whose
  subrecord width underflows to zero.
- A two-ULP interval near J2000 split into three granules: two adjacent
  internal boundaries collapse to one binary64 date.
- Valid two-granule layouts at two-ULP and 32-day scales, with finite
  geometric Mercury-relative-to-SSB state samples.
- A valid single-granule one-ULP interval.
- A two-record TDB JD grid spanning 2^20, where the first record's
  three-granule partition is representable but the second record's
  two inner boundaries coincide after binary64 rounding. The guard
  rejects the combined file without incorrectly rejecting the first
  record on its own.
- Public `De441::state` regression with distinct constant coefficients:
  samples just before an internal boundary select the preceding
  granule, the exact boundary selects the next, and the final
  coverage endpoint selects the final granule.
- A seventh public API regression compares rounded internal boundaries
  against their immediately preceding representable TDB dates for
  3, 9 and 97 granules, positive and negative epochs, and inclusive
  final endpoints. It exposes the old division/floor off-by-one bug
  without depending on an external ephemeris file.
- An eighth synthetic public API regression checks a one-granule
  `1e308`-day record with constant and linear Chebyshev coefficients.
  At the inclusive endpoint the old `2 * day_offset / width`
  expression overflowed before division even though the normalized
  coordinate is exactly `+1`; dividing before doubling preserves
  finite analytical positions and km/day velocities.

No external ephemeris kernels are used. These fixtures prove only
deterministic structural checks, not independent NASA/JPL/Horizons vector
parity or SPK `.bsp` support.

## Scientific scope

- Epoch: **Julian Date TDB** (not UTC, TT or UT1).
- Format: **classic little-endian DE441 binary**; not NAIF SPK or
  Astrodienst `.se1`.
- No center, frame or correction changes: the existing reader emits
  geometric ICRF XYZ position (km) and velocity (km/day).
- This change may reject previously accepted structurally unresolvable
  synthetic or nonstandard files. Ordinary DE441 granules measured in
  hours/days are not expected to be affected; real-file verification
  is still required.

See [DE441.md](DE441.md), [VALIDATION.md](VALIDATION.md), and
[AGENTS.md](../AGENTS.md). Source code remains CPAL-1.0;
required **Powered by VirgoStars** attribution is unchanged.
