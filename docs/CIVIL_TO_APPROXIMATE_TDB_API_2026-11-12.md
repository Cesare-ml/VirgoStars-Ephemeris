# Civil time → approximate TDB Julian date for Rust consumers

**2026-11-12. Experimental Rust API, scoped for MVP-1 input composition.**

## Motivation

The existing `first_sky::calculate` pipeline already normalizes explicit civil UTC offsets and computes **approximate** TDB Julian dates, but that conversion was private. Draft MVP-1 six-field speed pipelines [#70](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/70) (light-time only) and [#77](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/77) (light-time + annual aberration) accept a caller-supplied `JulianDateTdb` and must not duplicate, assume or silently reinterpret UTC conversion. This minimal change exposes the **existing algorithm once**, without creating new astronomy algorithms, linking the unmerged drafts or modifying their implementation.

`CivilTimestamp::to_approximate_tdb_julian_date(self) -> Result<JulianDateTdb, SkyError>` delegates to the original, still-private `epochs(self)`. It performs no file access and needs no geographic coordinates, dependencies, network calls or new time-scale policy.

## Scientific contract

- Input: proleptic Gregorian local civil date/time with explicit numeric `±HH:MM` UTC offset or `Z`; 24-hour clock. Offset is local minus UTC. Leap-second **labels** (`:60`) are not supported.
- Internal stages: normalize UTC → IERS Bulletin C 72 **TAI−UTC** table → TT (+32.184 SI seconds) → truncated USNO Circular 179 approximation for TDB−TT.
- Output: `de441::JulianDateTdb`, a **single f64 Julian day in approximate TDB**. No implied UT1 / UTC / TT, timezone-naive datetime, or `swe_julday` equivalence. For modern TDB epochs, single-f64 JD limits sub-millisecond precision.
- Strict UTC coverage: **1972-01-01 00:00:00 through 2027-06-27 23:59:59...** under the existing IERS schedule. The UTC instant, **not** the local calendar label, determines validity and the correct TAI−UTC entry. Values outside throw `SkyError::UnsupportedUtc`; invalid input remains a typed `SkyError`.
- No external Earth orientation, ephemeris relativistic TT↔TDB integral, high-precision historical time system or guaranteed absolute TDB accuracy. Do not advertise this as a final astronomical time-scale conversion.

## Rust usage: existing core

```rust
use virgostars_ephemeris_core::first_sky::CivilTimestamp;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let timestamp = CivilTimestamp::parse("2026-11-10T13:30:00+01:00")?;
let tdb = timestamp.to_approximate_tdb_julian_date()?;
println!("Approximate TDB JD: {}", tdb.0);
# Ok(())
# }
```

Once independently validated, the existing draft geocentric pipelines may consume that `JulianDateTdb` value as their *reception TDB epoch*. This **does not** magically complete the true apparent **ecliptic-of-date** chain: moving-frame derivatives, nutation, gravitational deflection, correct Earth-orientation, planet-center models, independent Horizons/Swiss reference parity, and Rust-backed JS/Python bindings remain required for VirgoStars production migration. The classic DE441 reader requires a separately obtained classic JPL file; this adapter does not download one.

## Offline regression tests

`crates/ephemeris-core/tests/civil_tdb_api.rs` checks:
- timezone representations of the **same instant** (`Z`, `+01:00`, `-05:30`, `+14:00`) map to matching epochs;
- the well-known **2017-01-01 UTC JD 2457754.5** and 37 s TAI−UTC give TT+TDB within the declared 3 ms truncated-model correction envelope;
- a 2016→2017 UTC crossing has a **3601-second TAI/TT** separation for **3600 seconds of nominal UTC clock** across the positive leap offset, with correct UTC selection despite misleading local dates;
- first and last supported UTC instants, nearby local offsets, and rejection outside IERS schedule;
- leap-second labels are explicitly unsupported instead of being accepted as `23:59:60`.

Run from workspace root in a Rust-enabled environment:

```sh
cargo fmt --all -- --check
cargo test --workspace --test civil_tdb_api
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

These are **time-contract tests**, not downloaded-kernel or planetary position accuracy validation.

## CPAL-1.0 modification record

2026-11-12: modified `crates/ephemeris-core/src/first_sky.rs` by adding a small original public delegating method without changing internal `epochs`; added one new CPAL-covered Rust test file and this documentation. Preserve the existing Exhibit A notices and license/NOTICE. Applicable attribution: **Powered by VirgoStars** — https://www.virgostars.it/. No third-party code or JPL datasets are included. No PR merge, deployment or release is authorized by this work.
