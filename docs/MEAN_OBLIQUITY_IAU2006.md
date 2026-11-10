# IAU 2006 mean obliquity and its TT-time derivative

**Added 2026-11-10. Status: one validated IAU 2006 scalar precession constituent, NOT a complete precession/nutation model, date-frame matrix or apparent planetary coordinate.**

## Scientific origin

The mean obliquity of the ecliptic of date, `ε_A(T)`, is the **angle between
the mean ecliptic and the mean equator of date**, as defined by the IAU 2006
precession model.

The public [IERS Conventions (2010), Technical Note 36, §5.6.4, Eq. (5.40),
page 65](https://iers-conventions.obspm.fr/content/tn36.pdf#page=65)
provides the polynomial

```text
T = (JD_TT - 2451545.0) / 36525
ε_A(arcsec) = 84381.406
           - 46.836769 T
           - 0.0001831 T²
           + 0.00200340 T³
           - 0.000000576 T⁴
           - 0.0000000434 T⁵
```

The model uses **Julian centuries of 36,525 TT days since J2000 noon**;
`84381.406` arcseconds is the J2000 reference value. A separate analytic
derivative is computed in the Rust core from the same polynomial using paired
Horner recurrences, with the chain rule

```text
dε_A / d(JD_TT) =
    dε_A / dT × (π / (180 × 3600)) / 36525
```

giving **radians per TT day**, not radians per observation-TDB day.
IERS §5.6.4 explains why using TT in practice instead of TDB for these
precession polynomials is appropriate; this API intentionally does NOT perform
time-scale conversions.

## Rust API and units

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::mean_obliquity::mean_obliquity_iau2006;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let tt_epoch = JulianDate::new(2_451_545, 0.5, TimeScale::Tt)?;
let model = mean_obliquity_iau2006(tt_epoch)?;
println!("ε_A = {} rad; dε_A/dTT = {} rad/day",
         model.radians, model.rate_radians_per_tt_day);
# Ok(())
# }
```

The input must already be a **split Julian date labeled TT**. The implementation
rejects TDB, TAI and UT1 rather than relabeling or silently converting them.
It uses the split JD parts to retain sub-second precision near contemporary
epochs; combining the parts to a single large `f64` JD first would discard
information needlessly.

The conservative *engineering* test window is `J2000 ±2 TT Julian centuries`,
approximately years 1800–2200. Values outside that bounded interval fail
explicitly rather than silently extrapolating. **This is our own testing
policy**, not an IAU validity cutoff or a quantified error bound.

## Independent regression evidence and reproducibility

The stand-alone Rust integration test
`crates/ephemeris-core/tests/mean_obliquity.rs` contains seven tests:

- A **numerical reference from ERFA**, not generated from this repository:
  `eraObl06(2400000.5, 54388.0) = 0.4090749229387258204 rad`, matching to
  `1e-14 rad`. Source: [ERFA's own C regression fixture, function
  `t_obl06`](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
  **Only the public numerical test value is reused**; no ERFA/SOFA source code
  or binary is incorporated.
- J2000 `84381.406` arcseconds and `-46.836769` arcseconds per Julian
  century, independently specified by the IERS polynomial.
- Centered finite-difference checks of the analytic TT-day derivative at
  multiple epochs, before and after J2000.
- Retention of a quarter-second TT offset in a split Julian date.
- Rejection of wrong time scales, out-of-window dates, and Julian day parts
  that cannot be safely represented.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test mean_obliquity
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The ERFA fixture is an independent check of the **specific scalar IAU 2006
polynomial**. It does NOT validate DE441 positions, frame bias, precession
matrix orientation, IAU 2000A nutation series, apparent positions or rates
against JPL Horizons or Swiss Ephemeris. It does not prove astronomical
accuracy outside the tested dates or use proprietary ephemeris files.

## Integration boundary

The open draft PR for moving-frame kinematics (PR #47) accepts a **full
orthonormal matrix** `R(t)` and its **time derivative with respect to
observation TDB days**. The mean-obliquity angle computed here is **not**
that matrix: a complete IAU 2006 precession/bias and IAU 2000A nutation model
is required before rotating real ICRF states into true ecliptic/equinox of
date. Moreover, the derivative `dε_A/dTT` cannot be passed as a
`dR/dTDB` rate without an explicit time-parameter conversion and the
other orientation terms. This deliberate separation prevents false Swiss
`swe_calc_ut` parity claims.

## CPAL-1.0 modification record

2026-11-10 — Added `src/mean_obliquity.rs`, exported it from `src/lib.rs`,
introduced `tests/mean_obliquity.rs`, and documented sources and limitations.
Original Rust implementation of published IERS scientific coefficients.
Source headers and project attribution remain under **CPAL-1.0**, including
the existing **Powered by VirgoStars** credit and https://www.virgostars.it/.
No external library/source code, Swiss implementation, NAIF SPICE code,
JPL data, ephemeris binaries, new dependencies, license changes, or
production modifications were introduced.
