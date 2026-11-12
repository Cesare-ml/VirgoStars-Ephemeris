# MVP-1: batch civil geocentric six-field states

**2026-11-12. Experimental stacked addition; draft #101 is the prerequisite.**

This change adds an **offline, dependency-free batch API** to the Rust astronomical core. It uses the **same** existing classic-DE441 light-time solver and spherical projection as the scalar civil adapter. It introduces **no** new astronomy algorithm or separate source of ephemeris truth.

## Rust interface

```rust
use virgostars_ephemeris_core::civil_geocentric::geocentric_fixed_spherical_many_from_civil;
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::first_sky::CivilTimestamp;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut eph = De441::open("/path/to/your/classic-de441.eph")?;
let civil = CivilTimestamp::parse("2026-11-10T13:30:00+01:00")?;
let results = geocentric_fixed_spherical_many_from_civil(
    &mut eph,
    &[Body::Sun, Body::Moon, Body::Mercury, Body::Venus],
    civil,
    0.0, // fixed ICRF equatorial axes
)?;
for result in &results {
    println!(
        "{:?}: lon={} rad, lon speed={} rad/TDB day",
        result.spherical.target,
        result.spherical.longitude_rad,
        result.spherical.longitude_rate_rad_per_day
    );
}
# Ok(())
# }
```

`geocentric_fixed_spherical_many_from_civil(&mut De441<R>, &[Body], CivilTimestamp, fixed_tilt_rad)` returns `Result<Vec<CivilGeocentricFixedSpherical>, CivilGeocentricError>`.

- **Time scales:** parse a proleptic Gregorian local time with explicit `Z`/`±HH:MM` offset. Compute **one** approximate TDB Julian date for all requested bodies via the existing `CivilTimestamp::to_approximate_tdb_julian_date` (draft #96), using the limited IERS Bulletin C 72 UTC schedule (1972-01-01 to 2027-06-27) and truncated USNO TDB−TT approximation. This is not a high-accuracy relativistic TT↔TDB solution.
- **Frame and observer:** all targets are relative to `Body::Earth` (the **geocenter**), in fixed ICRF equatorial axes if `fixed_tilt_rad = 0.0`, or a caller-chosen **constant** passive X rotation. A constant rotation is NOT a mean/true ecliptic of date.
- **Quantity and units:** geometric, with *one-way reception light-time* only. Longitude/latitude are **radians**, distance **km**; three signed rates are rad/reception **TDB day** and km/reception **TDB day**. Target emission TDB Julian epoch, light-time seconds and dimensionless light-time rate are retained per target. Exact system-barycenter body identifiers (e.g. JupiterBarycenter) are not relabeled as physical planetary centers.
- **Ordering:** preserves the caller's requested order and duplicates. Each target uses the scalar API's existing internal calculation, while the batch shares the **civil→TDB conversion**.
- **Errors:** all-or-error; missing coefficient series, invalid target/center (including geocentric Earth itself), out-of-coverage retarded emission time, undefined directions, bad tilt and time-scale errors fail the **whole** batch. The `Vec` is not returned partially. The original typed `CivilGeocentricError` distinctions are retained.
- **Empty targets:** still validate civil time and finite axis tilt but return an empty vector without querying ephemeris body data or asserting per-target kernel coverage. No hidden default target list, auto-download or internet call.
- **I/O/cache:** uses the original `De441<R>` reader and its normal coefficient-record cache; batching is an ergonomic API and **not** a measured performance improvement. No new lock, global state, JS, Python or application runtime dependency.

## Source-level design

The scalar `geocentric_fixed_spherical_from_civil` now delegates to a **private, shared** `spherical_at_reception_tdb` helper. The batch reuses it, avoiding duplicated light-time solution, per-coordinate derivative formulas, or second ephemeris implementation. The old public scalar signature and returned fields are unchanged.

This batch does **not** supersede draft #70's degrees/AU adapter, #77's aberration-corrected six-field path, or #89/#92/#97's observer orientation and frame work. Those require separate scientific and integration review.

## Verification

Four additional, **synthetic-only** regression tests in the existing `tests/civil_geocentric_pipeline.rs`:
1. Real two-body paths with a fictional 9 KB classic-DE441 binary (Mercury + Venus), preservation of input ordering and duplicates, and **exact equality** between every batch result and the scalar API result for the same epoch/body.
2. Empty requests validate UTC coverage and reject nonfinite fixed-axis tilt even without any body series.
3. A missing series in the middle of a batch fails the full operation with its original typed error; the reader remains reusable for a subsequent valid call, and Earth-as-target is still explicitly rejected.
4. UTC-offset equivalence `+01:00`/`Z` and **negative retrograde** rate preserved for the moving body, while a stationary body retains zero rate.

Run with a Rust toolchain:

```sh
cargo fmt --all -- --check
cargo test --workspace --test civil_geocentric_pipeline
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The test kernel is **fictional mathematical data**, not official NASA/JPL/Horizons input. Passing does **not** establish numerical planetary accuracy, `swe_calc_ut` parity, body-center completeness, relativistic deflection, annual aberration, precession/nutation, topocentric correction, moving-frame speeds or a production-ready Swiss replacement. The reader accepts separately supplied **classic DE441 binary**, not SPK `.bsp` or Astrodienst `.se1` files.

## License and branch integration

**2026-11-12 CPAL-1.0 modification record:** modified original `src/civil_geocentric.rs` to add an original batch API and factor the shared private function, extended `tests/civil_geocentric_pipeline.rs` with four synthetic regression cases, added this document. Existing Exhibit A source headers, `LICENSE` and `NOTICE` remain intact. Applicable attribution: **Powered by VirgoStars** — https://www.virgostars.it/. No external source code or dataset redistribution.

**Stack order:** draft #96 (civil→TDB), draft #101 (scalar civil→geocentric six fields), then this batch PR. Keep each separate for review; do not merge any of them into `dev/ephemeris` or `main` without explicit owner approval, and do not publish a package or deploy.
