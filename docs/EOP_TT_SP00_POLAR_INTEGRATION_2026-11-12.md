# UTC EOP CSV → TT epoch → conventional SP00 → ITRS/TIRS polar matrix

**Date:** 2026-11-12. **Status:** experimental position-only numerical
adapter, **draft stacked on PR #105** and therefore dependent on the
still-unmerged PR #89 observer orientation components. This document
records software behavior and validation evidence, not scientific
accuracy of actual EOP observations or astronomical chart results.

## Why this integration matters

The standalone original CelesTrak EOP CSV reader in **PR #109**
provides UTC daily **polar-motion** x/y values in radians with an
observed/predicted quality flag. The separate **PR #100** provides
the conventional *approximate* TIO locator s′ from a **TT** split JD;
the draft combined observer/integrated polar branch **PR #105**
provides the 3×3 ITRS→TIRS matrix W(t) from
`xp, yp, s′`.

This branch starts from **PR #105 HEAD
`a94808e0114150958113b910a0a2524fed4c8e54`** and imports
the untouched original PR #109 CSV parser/test/docs Git blobs from
**`daf4be56046362268cf0929a385d5c0b6cac109f`**.
Both original component PRs and branches remain unchanged and
unmerged. The new code adds an **explicit, normal-day time
alignment** and composes these previously independent modules.

## API and intended contract

```rust
use virgostars_ephemeris_core::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopPredictionPolicy,
};
use virgostars_ephemeris_core::eop_polar_orientation::
    approximate_itrs_to_tirs_from_eop_utc;

// The caller is responsible for obtaining a verifiable snapshot,
// checking its IERS/CelesTrak provenance, freshness and licensing.
// This is a historical documentation fixture, not current EOP.
# fn example(csv: &str) -> Result<(), Box<dyn std::error::Error>> {
let catalog = CelestrakEopPolarCatalog::parse_csv(
    "Externally verified CelesTrak EOP historical CSV", csv
)?;
let solution = approximate_itrs_to_tirs_from_eop_utc(
    &catalog, 51544, 0.0, EopPredictionPolicy::ObservedOnly
)?;
println!(
    "TT split JD: {:?}; EOP xp={}; yp={}; s′={}",
    solution.tt_epoch,
    solution.xp_rad,
    solution.yp_rad,
    solution.approximate_polar.locator.sp_rad,
);
let w_itrs_to_tirs = solution.approximate_polar.itrs_to_tirs;
println!("{w_itrs_to_tirs:?}");
# Ok(())
# }
```

Inputs are the **caller-selected** catalog, integer UTC MJD day,
UTC fraction of a normal 86400-SI-second day in **[0,1)**, and an
explicit **ObservedOnly** / **AllowPredicted** policy. No network,
filesystem discovery, system time, time zone, hidden EOP fallback
or automatic ephemeris time is accessed.

For nonzero day fractions, the catalog must contain **two
consecutive daily records**. The selection is in UTC, with linear
`xp/yp` interpolation. Forecast values are rejected unless the
call authorizes them, and the output then records
`EopPolarQuality::IncludesPrediction`. `source_label` is retained
for auditing and does not prove the file is genuine.

The output includes the input UTC MJD and fraction, source label,
observed/predicted quality, **DAT** (TAI−UTC at 0h UTC), a
**TimeScale::Tt** split Julian date, interpolated polar coordinates
`xp_rad`, `yp_rad`, and the conventional approximate
`s′(TT)` plus 3×3 polar-motion matrix. s′ is in **radians**;
its rate is only the mathematical derivative of a linear
approximation, not the observer's inertial velocity.

## Exact time handling for modern, normal UTC days

CelesTrak documents `DATE/MJD` as the UTC date/0h sample and
`DAT` as TAI−UTC in seconds. The defined TAI→TT relationship is
exactly +32.184 seconds. For ordinary **post-1971 UTC days with no
DAT discontinuity**:

```text
TT−UTC (seconds) = (TAI−UTC)_at_0h + 32.184

TT fraction = UTC normal-day fraction
             + (DAT + 32.184)/86400

TT split JD day_number = UTC MJD + 2400001 (+1 on fraction carry)
TT split JD midnight_fraction = normalized TT fraction
TT scale = TimeScale::Tt
```

The code refuses dates before **1972-01-01 (MJD 41317)** because
pre-1972 UTC/TAI offsets were not simply whole leap seconds.
A conservative DAT range of 10–100 integer seconds is a
numerical sanity check, **not an IERS/DAT authenticity guarantee**.
TT is formed with split-day arithmetic; the function never first
converts a modern full Julian date to a single lossy `f64`.

For nonzero UTC day fractions, the next daily EOP record must have
the same DAT. If it changes, the code returns
`LeapSecondBoundary`: a leap-second day can have **86401
or 86399 SI seconds** and requires a complete UTC
second-of-day interpretation. The routine must never silently
treat a leap-second day as an ordinary one.
Exact 0h UTC samples are supported even on adjacent leap days
because each daily DAT applies at its recorded midnight.

These checks show consistency **only against caller-supplied data**.
A counterfeit, stale or internally inconsistent DAT snapshot can
pass. A physically validated application needs an independently
authorized leap-second schedule and EOP source, proper UTC
time-of-day semantics and provenance/audit metadata.

**UT1 is not produced.** The CSV reader retains the daily
`UT1−UTC` metadata but the time offset is **not interpolated**
here, especially not across leap-second jumps. A separate
quality-controlled UT1 model will be needed for Earth Rotation Angle
and the GCRS observer transform in PR #89.

## Physical precision boundary

- The 3×3 ITRS→TIRS polar matrix uses the original IERS
  `xp/yp/s′` implementation already validated separately against
  the nine-element SOFA/ERFA `t_pom00` fixture.
- The TIO locator s′ is the **conventional SP00 linear secular
  approximation** `−47 µas per TT Julian century since J2000`,
  not a measurement or complete geophysical model. It is distinct
  from the **celestial CIO locator s**, CIP X/Y, UT1 ERA and
  TDB/ICRF frame transformations.
- The quality flag distinguishes observed vs forecast EOP,
  but does not represent formal uncertainties or accuracy bounds.
- The algorithm does not model UTC leap-second instants,
  TT↔TDB or UT1 conversion, polar-motion derivatives,
  Earth tide, station velocity, atmospheric refraction,
  apparent astrometry, true planetary observer geometry
  or DE441/JPL Horizons scientific parity.
- The mathematical source formula and matrix tests do not
  establish that the catalog's actual EOP coordinates are
  from the same ITRS realization or physical epoch as a
  particular scientific comparison.

## Independent fixtures and QA

The 8 new integration test groups cover:

1. The published [CelesTrak 2000-01-01 CSV example](https://celestrak.org/SpaceData/EOP-format.php):
   UTC MJD 51544, `X=0.043282″`, `Y=0.377909″`,
   `DAT=32` seconds, `O`; derived TT 0h UTC +64.184 s,
   correct arcsecond→radian conversion and independent
   conventional SP00 and matrix products.
2. Synthetic following UTC day, linear observed half-day poles and
   exact split TT position.
3. A normal UTC day near midnight which correctly carries
   the TT fraction into the next Julian-day number.
4. Forecast-day rejection without policy opt-in and preservation
   of `IncludesPrediction` if intentionally allowed.
5. A synthetic 2016-12-31 → 2017-01-01 snapshot with the
   **historically real DAT change 36→37**: deny nonzero
   fractions on the leap-second day, but accept exact midnight.
6. Pre-1972 UTC rejection even when a CSV row has integer DAT.
7. Invalid DAT metadata rejection without inferring an
   external leap-second schedule.
8. Missing consecutive samples and bad UTC fraction failures.

The first record is an independent **published CSV sample**;
the next-day and leap-boundary pole vectors are synthetic.
Separately, the unchanged parent PR #105 tests include published
**SOFA/ERFA SP00 and POM00** fixture values and all nine matrix
components with correct transpose. Neither integration set is
real-dated IERS/JPL astrometric accuracy evidence.

## Review workflow and CPAL

Code is original VirgoStars Rust, with CPAL-1.0 Exhibit A source
headers and mandatory project attribution. No bundled EOP datasets,
third-party code, extra crate dependencies, workflows, GitHub
Actions authoring, release or production migration. The original
PR #105 and PR #109 files remain intact; a new draft PR targets
the #105 integration branch and is not to be merged into
`main` without completed scientific review and owner approval.

Required CI:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

**References:**
[CelesTrak EOP CSV format](https://celestrak.org/SpaceData/EOP-format.php),
[IERS Conventions (2010)](https://iers-conventions.obspm.fr/conventions_material.php),
[ERFA independent orientation regression suite](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
