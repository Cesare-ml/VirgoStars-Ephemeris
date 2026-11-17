# Proleptic calendar boundary regression suite

**2026-11-10 — calendar arithmetic validation, not ephemeris validation.**

The Rust integration suite `crates/ephemeris-core/tests/calendar_boundaries.rs`
exercises the existing `calendar` module without modifying its public API.

## Independent civil-date anchors

- **J2000.0** is Julian Date **2451545.0** at **2000-01-01 12:00** in the
  selected time scale. The calendar test uses **TT**, not UTC.
- Gregorian **1582-10-15 00:00** is JD **2299160.5**; Julian
  **1582-10-05 00:00** represents that same instant on the same scale.
  These are standard published Julian-date reference anchors; see the
  [US Naval Observatory Julian Date introduction](https://aa.usno.navy.mil/data/JulianDate)
  and [JPL time specification](https://ssd.jpl.nasa.gov/tools/jdc/).

These checks validate integer calendar-day offsets and noon/midnight JD
conventions. They do not validate a physical UTC↔TT conversion or JPL
planetary interpolation.

## Additional deterministic coverage

- Proleptic Gregorian dates in the historically omitted 1582 civil interval
  round-trip without an implicit reform/cutover.
- Gregorian 400-year century leap rule vs. Julian every-four-years leap rule
  at 1600–2400, including 1700, 1800, 1900 and 2100.
- Astronomical year numbering around BCE/CE (year zero is 1 BCE).
- Preservation of explicit TAI, TT, TDB and UT1 labels without silently
  transforming time scales.
- Invalid leap dates, 24-hour clock values, NaN/infinite fractions and
  out-of-range fractional Julian days return typed errors.

All six tests are synthetic/deterministic except the two published date
anchors. No external datasets, kernels, downloads or dependencies are needed.

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --test calendar_boundaries
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

GitHub Actions on the PR will run these checks as part of the existing
cross-platform CI. An unexecuted command is not evidence of a pass.

## License and provenance

2026-11-10 — Added one original Rust test suite and this documentation
under the repository's existing **CPAL-1.0** license. No copied Swiss
Ephemeris, SOFA, NAIF, or ERFA implementation, no JPL binary redistribution,
no new dependency, and no change to the mandatory **Powered by VirgoStars**
attribution (https://www.virgostars.it/).
