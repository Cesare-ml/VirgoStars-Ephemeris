# IERS terrestrial intermediate origin s′(TT): bounded secular approximation

**Date:** 2026-11-12. **Status:** experimental original Rust primitive;
draft PR targeting `dev/ephemeris`. **Not** a validated full Earth
orientation model or a real EOP time series.

## Why the TIO locator matters

The IERS Earth-orientation chain, in its CIO convention, needs
polar motion `W(t)` to rotate between Earth-fixed ITRS
coordinates and terrestrial intermediate TIRS coordinates.
That polar-motion matrix uses **xp**, **yp** (observed polar motion)
and the **terrestrial intermediate origin locator s′**.

These three inputs are different quantities:

- `xp` and `yp`: actual polar coordinates from an appropriate
  Earth-orientation-parameters (EOP) series.
- **s′ (s-prime)**: the terrestrial origin locator. A conventional
  secular approximation is available from the **TT** epoch.
- **s (without prime)**: the *celestial* CIO locator used in the
  GCRS↔CIRS orientation. It is NOT this s′ function and must not
  be interchanged with it.

The separate draft polar-motion PR #84 already accepts an explicit
s′ in radians, but does not compute it. This feature is a
**standalone optional approximate supplier of s′**, with no
dependency on that unmerged PR.

## Formula and input

For TT Julian centuries since J2000:

```text
T_TT = (JD_TT - 2451545.0) / 36525

s′_approx [arcseconds] = -47e-6 * T_TT

s′_approx [radians] = -47e-6 * π / (180*3600) * T_TT
```

The secular drift is −47 microarcseconds per Julian **century**,
not per Julian day. Its mathematical model derivative is

```text
ds′_approx / dTT_day = -47e-6 * π / (180*3600) / 36525
```

This is the conventional secular **approximation**, not an EOP
measurement or an exact geophysical law. The TIO locator is
physically influenced by observed polar motion and may contain
unpredictable departures. Use measured/modelled EOP quantities
and the appropriate IERS standard for high-accuracy applications.

The Rust API is:

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::tio_locator::approximate_tio_locator_sp00;

let tt = JulianDate::new(2_452_542, 0.0, TimeScale::Tt)?;
let approximation = approximate_tio_locator_sp00(tt)?;
println!("TIO s′ = {} rad", approximation.sp_rad);
println!(
    "Secular rate = {} rad / TT day",
    approximation.secular_rate_rad_per_tt_day
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The `JulianDate` must have `TimeScale::Tt`. The implementation
does not treat UT1 or TDB as TT and does not implement UTC→TAI→TT
conversion. It calculates centuries using the **split** integer
Julian-day number and fractional day to avoid collapsing an epoch
to a coarse single-f64 JD before subtracting J2000.

**Deliberate usage boundary:** TT epochs more than ±2 Julian
centuries from J2000 are rejected with a typed
`BeyondApproximationRange` error. This limits unbounded
secular extrapolation; it is **not** an accuracy guarantee inside
1800–2200. An application requiring scientifically defensible
precision must supply the appropriate model and validate it
against reference data.

## Independent reference

The independently published `t_sp00` test in
[ERFA's numerical test suite](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c)
uses:

```text
TT = 2400000.5 + 52541.0 = JD 2452541.5

expected s′ = -0.6216698469981019309e-11 rad
```

An independent Rust test compares the module's result with that
published value to an absolute tolerance of `1e-21` rad.
Seven other test groups check the J2000 anchor, −47 µas/century
sign and units, analytical rate via finite differences, split-JD
sub-day precision, rejection of UT1/TDB/TAI labels, ±2-century
bounds, and extreme integer Julian days.

These tests validate the **specific SP00 approximation**, not the
full IERS polar-motion matrix with real EOP input, CIP/CIO series,
the observer position or real JPL/Horizons parity.

## Integration plan and pending science gates

- In the separate #84 ITRS↔TIRS polar-motion API, the caller can
  *explicitly choose* to pass `approximation.sp_rad` instead
  of a separately sourced measured TIO value. Do **not** silently
  substitute this approximation without recording that choice.
- Existing #75 ERA(UT1), #82 WGS84, #83 Earth spin, #86 CIP/CIO,
  #87 matrix composer, and #89 observer integration remain
  separate draft contributions. This module does not merge or
  change their branches.
- The complete observer pipeline still requires TT/UT1/TDB
  temporal consistency, xp/yp EOP, CIP X/Y and CIO s, a physical
  GCRS/DE441 alignment, observer velocity, actual DE441/Horizons
  checks and appropriate apparent corrections.
- Derivative of the simple secular s′ formula is **not** an
  observer velocity. It is not a substitute for `dW/dTT`,
  `dERA/dUT1` or `dC/dTT`.

## QA and legal terms

Native read-only Rust CI for the draft PR to `dev/ephemeris`:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

The new Rust module and test have the project's existing CPAL-1.0
Exhibit A notice and preserve required VirgoStars attribution.
This is original simple mathematical implementation based on the
public IERS definition, using only independent ERFA **test values**,
not SOFA/ERFA/Swiss source code. No new dependencies, releases,
data downloads or production deployments.

**References:** [IERS Conventions 2010](https://iers-conventions.obspm.fr/conventions_material.php);
[ERFA `t_sp00` reference test](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c);
[SOFA documentation for secular s′ approximation](https://marcomaggi.github.io/docs/iausofa.html).
