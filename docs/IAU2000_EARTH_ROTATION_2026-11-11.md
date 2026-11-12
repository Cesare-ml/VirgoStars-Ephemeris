# IAU 2000 Earth Rotation Angle (ERA) — 2026-11-11

**Status:** experimental, original Rust astronomical primitive. This change
does not deliver Greenwich sidereal time or a full house calculation.

## Why this belongs to MVP1

The VirgoStars backend uses sidereal/earth-orientation quantities when
constructing astronomical angles for houses and current-sky features.
The core must provide explicit Earth-rotation quantities independently of
calendar conversion and ephemeris interpolation. This PR adds one foundational
primitive without touching in-progress nutation, precession or planetary PRs.

## Physical and mathematical contract

The Earth Rotation Angle (ERA) measures the rotation around the celestial
intermediate pole between the celestial and terrestrial intermediate origins,
following **IAU 2000 Resolution B1.8** and **IERS Conventions (2010)**
Chapter 5, equations (5.14) and (5.15):

```text
t  = JD(UT1) - 2451545.0          [UT1 days]
ERA = 2π [0.7790572732640 + 1.00273781191135448 × t] mod 2π
dERA/d(UT1 day) = 2π × 1.00273781191135448  [radians / UT1 day]
```

An equivalent numerically better implementation splits the linear term
into the integral rotations plus the fractional-day portion:

```text
ERA/(2π) = [0.7790572732640
           + (midnight_fraction - 0.5)
           + 0.00273781191135448 ×
             (day_number - 2451545 + midnight_fraction - 0.5)] mod 1
```

This retains the split-Julian-date fraction instead of first adding it to
a large single-f64 Julian date. Result `angle_rad` is in **[0, 2π)**.
`rate_rad_per_ut1_day` is a **constant IAU 2000 defined derivative**.
The caller must provide a valid `JulianDate` tagged `TimeScale::Ut1`
that actually represents UT1. Input outside the split-date arithmetic's
representable range returns an error; TAI, TT and TDB labels are rejected.

No UTC-to-UT1/EOP correction, leap seconds, Delta-T, precession, nutation,
polar motion or true/mean equinox transformation are performed.
ERA **must not be mislabeled GMST, GAST, local sidereal time, ARMC or
a complete Earth-to-sky coordinate transform**. In particular it is **not
sufficient by itself** to calculate Swiss `swe_houses` or `swe_houses_pos`.

## Rust usage

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::earth_rotation::earth_rotation_angle;

let epoch_ut1 = JulianDate::new(2_451_545, 0.5, TimeScale::Ut1)?;
let result = earth_rotation_angle(epoch_ut1)?;
println!("ERA = {} rad, angular speed = {} rad/UT1 day",
         result.angle_rad, result.rate_rad_per_ut1_day);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Scientific references and test provenance

1. [IAU Resolution B1.8 note 3](https://iau-a3.gitlab.io/res.html):
   defining relationship between UT1 and the Earth Rotation Angle.
2. [IERS Conventions (2010), Technical Note 36, §5.5.3](https://iers-conventions.obspm.fr/conventions_material.php):
   equations 5.14–5.15, including the split-Julian-day arithmetic.
3. [Independent SOFA/ERFA `eraEra00` regression](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c):
   the published input `(2400000.5, 54388.0)` yields the reference
   **0.4022837240028158102 radians** (published tolerance 1e-12).

Rust tests (`crates/ephemeris-core/tests/earth_rotation.rs`):
- Independently published ERFA value with an absolute `1e-12` rad tolerance.
- Defining J2000 noon reference, next-day offset and analytical rate.
- Microsecond split-date precision versus single-f64 JD quantization.
- Continuity at midnight, ancient/negative Julian days, wrapping, and rate.
- Reject non-UT1 time-scale labels and invalid date magnitude.

These are deterministic analytical tests **plus one independent ERA
reference case**, not validation of a complete sidereal/ephemeris API or
numerical parity of apparent planetary coordinates with Swiss or JPL.

Expected standard Rust check commands:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Scope, licensing and next step

Four files are changed: new `earth_rotation.rs` and tests, a single
module export in `lib.rs`, plus this documentation. Original implementation
and tests follow the repository's CPAL-1.0 source notice and modified-file
requirements. **No external implementation is copied**, no dependency,
dataset or license is introduced, and no production use is enabled.

Follow-up work: explicit EOP/UT1 policy, CIO/equinox-of-date pipeline,
IAU 2006 consistent sidereal-time model and independent reference cases
for house axes. All of those require separate review and should not be
implicitly provided by this primitive.
