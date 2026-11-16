# IAU 2006 Greenwich Mean Sidereal Time correction — 2026-11-11

**Status:** experimental original Rust component on a draft PR; not on
`main` or production. No use of the Swiss Ephemeris implementation.

## Motivation and independent integration boundary

The VirgoStars backend needs correctly modeled Earth orientation and
sidereal time as prerequisites for house calculations and planetary-hour
features. IAU 2000 **ERA** is a function of UT1, while IAU 2006 **GMST**
adds an accumulated-precession polynomial evaluated at TT.

To keep this PR independent of the open ERA implementation **PR #75**, the
public Rust function explicitly accepts a valid IAU 2000 ERA in radians
and an independent TT split Julian date. After PR #75 is reviewed, a
future adapter can call its ERA function and pass the result here.
**Neither this API nor PR #75 alone calculates the houses.**

## Definition, provenance and units

From the **IERS Conventions (2010), TN36, equation (5.32)**:

```text
t_TT = (JD(TT) - 2451545.0) / 36525
GMST_IAU2006 = wrap_2pi[ ERA_IAU2000(UT1) +
  (0.014506 + 4612.156534*t + 1.3915817*t^2
   - 0.00000044*t^3 - 0.000029956*t^4
   - 0.0000000368*t^5) arcseconds ]
```

The implementation evaluates this expression and its analytic derivative
with respect to **TT Julian days**, using the split date rather than a
single-f64 JD. Output values:

- `iau2006_gmst_correction(tt)` returns `GmstCorrection`:
  `offset_rad` in signed radians, and
  `offset_rate_rad_per_tt_day` in rad/TT day.
- `iau2006_gmst_from_era(era_rad, tt)` returns the *mean*
  Greenwich sidereal angle in **[0, 2π)** radians.
- `era_rad` must be finite, normalized and genuinely calculated from
  UT1 using the IAU 2000 convention. The function cannot independently
  prove that provenance.
- `tt` must represent physical TT and be tagged
  `TimeScale::Tt`. This code does **not** convert UTC, UT1, TAI,
  TT, TDB or time zones. The caller must obtain UT1/EOP/Delta-T by
  independently documented policy.

The ERA's angular rate with respect to UT1 and the polynomial's TT rate
are **distinct derivatives**; they cannot be added into one d(GMST)/d(UTC)
without a defined relation between those time scales.

This is not **GAST**, the equation of the equinoxes/origins, local
sidereal time, ARMC, true equinox-of-date, or `swe_houses`. For a
consistent IAU 2006/2000A GAST or house calculation, add the reviewed
nutation model, origin/equinox conversion, EOP and further tests.
The polynomial is defined across astronomical epochs but has not been
independently validated to the whole historical range of VirgoStars.

## Example

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::gmst_iau2006::{
    iau2006_gmst_correction, iau2006_gmst_from_era
};

let tt = JulianDate::new(2_451_545, 0.5, TimeScale::Tt)?;
let era_ut1 = 1.0; // EXAMPLE ONLY: obtain true IAU 2000 ERA from UT1 in production.
let correction = iau2006_gmst_correction(tt)?;
let gmst_mean = iau2006_gmst_from_era(era_ut1, tt)?;
println!("offset {} rad; TT-day rate {} rad/day; GMST {} rad",
    correction.offset_rad,
    correction.offset_rate_rad_per_tt_day,
    gmst_mean);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Verification and independent reference

The regression fixture is from the independent **ERFA `t_gmst06`**
test, `eraGmst06(2400000.5, 53736.0, 2400000.5, 53736.0)`,
whose published output is **1.754174971870091203 rad** with
**1e-12 rad** tolerance. The ERA input corresponding to that UT1
epoch is obtained from the independent IAU 2000 ERA defining formula;
the final expected GMST is not derived from our output.

Additional offline tests cover the J2000 zero-epoch offset and
analytic derivative, comparison of derivative with a symmetric finite
difference, angle wrapping, invalid ERA angles, wrong TT scale labels,
and unrepresentable Julian dates.

References:
- [IERS Conventions Centre (2010), TN36, equation 5.32](https://iers-conventions.obspm.fr/conventions_versions.php)
- [IERS TN36 eq. 5.32 synopsis](https://www.iers.org/iers/en/dataproducts/conventions/conventions)
- [ERFA independent t_gmst06 fixture](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c)

A standalone independent GMST scalar fixture is **not** evidence that
the future combined apparent-sidereal/house pipeline, Swiss ABI, leap
seconds, or planetary positions match their external references.

## Test and integration gate

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The existing `dev/ephemeris` pull-request CI also runs release profile
tests, documentation checks, and native Windows/macOS Rust suites.
No new dependencies or data, publication, deployment or production
changes are part of this PR. The existing CPAL-1.0 source notice and
VirgoStars attribution remain unchanged.
