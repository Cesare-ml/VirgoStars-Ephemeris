# MVP-1 integration: civil UTC-offset → approximate TDB → aberrated six-field batch

**Date: 2026-11-12. Experimental, unmerged integration branch; NOT a production Swiss-Ephemeris replacement.**

## Purpose and dependency discipline

Two independently developed draft lines need a precise integration boundary:

- [Draft #96](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/96) provides `CivilTimestamp::to_approximate_tdb_julian_date` using the existing `first_sky::epochs` UTC/TAI/TT/truncated TDB−TT conversion.
- [Draft #107](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/107) provides `geocentric_aberrated_fixed_six_fields_many`, which reuses the Rust DE441/light-time/special-relativistic observer aberration/analytic Chebyshev Earth acceleration pipeline and fixed-axis angular and range rates. It depends on [#77](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/77) → #76 → #73 → #46.

This draft **branches from the exact head of #107**, not from `main` or `dev/ephemeris`. Since #96 is also still unmerged and its module is not yet available on #107, this integration branch includes an **exact source-text snapshot of the 19-line #96 public `CivilTimestamp` delegating method** in `first_sky.rs` (same implementation and docs, verified byte-for-byte against #96). This is a **temporary dependency bridge**, NOT a new time-scale algorithm. The original branches #96 and #107 remain untouched. **Before integrating into dev, first land or explicitly reconcile #96 and #107, then remove any duplicate diff/rebase this bridge. Never merge this combined stack into protected branches by itself.** The intended permanent source of the conversion is #96, not a second implementation.

The only substantive new runtime code is `src/civil_aberrated.rs`, which uses existing, independently named Rust APIs. No JS, TypeScript, Python, Angular, or alternative astronomical formulas are implemented.

## Exact Rust API

```rust
use virgostars_ephemeris_core::civil_aberrated::{
    geocentric_aberrated_fixed_six_fields_many_from_civil,
};
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::first_sky::CivilTimestamp;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut eph = De441::open("/user-supplied/classic-de441.eph")?;
let date = CivilTimestamp::parse("2026-11-10T13:30:00+01:00")?;
let batch = geocentric_aberrated_fixed_six_fields_many_from_civil(
    &mut eph,
    &[Body::Sun, Body::Moon, Body::Mercury, Body::Venus],
    date,
    0.0, // exactly fixed ICRF equatorial axes
)?;
for result in &batch.results {
    println!(
        "{:?}: longitude {} deg, rate {} deg/TDB-day",
        result.fields.target,
        result.fields.longitude_deg,
        result.fields.longitude_speed_deg_per_tdb_day
    );
}
# Ok(())
# }
```

It returns `Result<CivilAberratedSixFieldsBatch, CivilAberratedError>`, including **`reception_epoch: JulianDateTdb` even for an empty target list**, and `results: Vec<AberratedFixedSixFieldsDegreesAu>`. All requested bodies share the same converted reception TDB Julian date, preserving input order and duplicates. The existing batch computation does not silently skip invalid bodies or return a partial result.

The civil time normalization is performed **once** by the existing #96 delegate. The astronomical calculation is delegated to the **same** #107 multi-body function, with no second light-time, observer boost, derivative, spherical projection, or unit formatter.

## Time-scale, frame and scientific contract

- **Input civil time:** proleptic Gregorian with explicit `Z` or `±HH:MM` UTC offset. Unsupported leap-second `:60` labels are rejected. A caller must never pass unspecified local time and assume UTC.
- **UTC coverage:** IERS Bulletin C 72 offset schedule in this experimental implementation supports 1972-01-01 through 2027-06-27 UTC. Outside returns a nested `CivilAberratedError::CivilTime(SkyError)`.
- **TDB conversion:** TT = TAI + 32.184 s; TDB−TT uses the **truncated USNO series**, not a full ephemeris relativistic integral. Output is a **single `f64` approximate Julian date**, so precision is finite at modern epochs. The returned speed rates are derivatives per **reception TDB day**, not UTC or UT1 day.
- **Target/center:** each target is Earth's-center relative, carrying its exact `Body` identity. Outer body variants such as JupiterBarycenter represent **system barycenters**, not physical planets' centers.
- **Reference frame:** `fixed_tilt_rad=0.0` yields fixed ICRF equatorial axes; arbitrary other values are a **constant passive X tilt**, NOT moving mean or true ecliptic-of-date, and add no `dR/dt * r` term.
- **Correction set:** original classic-DE441 target emission epoch with **one-way light time**, annual aberration of observer direction, **analytic Earth Chebyshev observer acceleration** for rates. Results are angular position in degrees, range in IAU-2012 AU and **three signed speed fields** in degrees/TDB-day or AU/TDB-day, plus per-target emission epoch, light-time seconds and rate and observer acceleration provenance.
- **Missing physics:** no independently verified real-DE441 numerical parity; no Sun's gravitational light deflection or Shapiro delay; no complete precession/nutation and moving-date-frame rates; no topocentric effects or EOP; no physical outer-planet centers. **DO NOT call this `swe_calc_ut` equivalence or accurate apparent ecliptic-of-date positions.**

`CivilAberratedError::CivilTime` preserves UTC conversion failures; `CivilAberratedError::Aberrated` nests the original detailed DE441/light-time/projection/unit errors, without hiding scientific failure modes. A request for no targets still validates civil time and tilt, then returns an empty vector with the converted TDB epoch, without claiming any particular DE441 body coverage.

## Deterministic regression tests

`tests/civil_aberrated_batch.rs` builds a fully invented **9 KB classic-DE441** coefficient file in memory, with independent quadratic Mercury, EMB and geocentric Moon series. This exercises the actual DE441 parser, Earth–Moon reconstruction, analytic Earth acceleration, target light-time solver, aberration, spherical velocity projection and unit conversion end-to-end. No proprietary, Swiss, JPL or NAIF data is bundled.

1. Equal UTC instants as `2026-11-10T13:30+01:00` and `12:30Z` produce identical metadata and all fields. The composed wrapper matches the underlying TDB batch **exactly** for `[Moon, Mercury, Moon]`, retaining order, duplicates, target, center and analytic acceleration provenance.
2. Full-pipeline observations at `12:28Z`, `12:30Z`, `12:32Z` yield independent centered numerical differences for all three output speeds, with **signed wrapped** longitude delta and an explicit retrograde example. This checks derivatives without trusting the derivative field as its own oracle.
3. Unsupported UTC dates and missing Sun series are distinguishable through the appropriate nested error variants. After a failure the reader can be reused for a valid Moon result.
4. Empty-body requests preserve the converted reception epoch; nonfinite axis tilt is rejected even when no series is read.

Commands to run when a Rust toolchain is available:

```sh
cargo fmt --all -- --check
cargo test --workspace --test civil_aberrated_batch
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The existing authorized **read-only CI** can run Linux, native Windows and native macOS checks from a temporary validation PR to `dev/ephemeris`, which must be **closed without merge**. These synthetic regressions establish integration behavior **only**, not independent accuracy against actual Horizons or Swiss reference data.

## Attribution, files and review safety

**Modification record (2026-11-12):** new `src/civil_aberrated.rs`, new `tests/civil_aberrated_batch.rs`, one `lib.rs` module export and this English document; exact source-text import of the small approved dependency #96 API method into `src/first_sky.rs`. All original CPAL-1.0 Exhibit A notices are preserved, `LICENSE`/`NOTICE` unchanged. No external data/kernel redistribution or additional dependency. Applicable attribution: **Powered by VirgoStars** — https://www.virgostars.it/.

**Integration sequence:** upstream #96; upstream #46 → #73 → #76 → #77 → #107; then review/reconcile this bridge's duplicated #96 delegate and its test results before a separately authorized integration decision. Nothing here authorizes `main` changes, dev merges, app production updates, releases, external kernel downloads, data migrations or publishing packages.
