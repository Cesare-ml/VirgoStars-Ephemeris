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
