# Geometric altitude crossing hour angles — 2026-11-11

**Status:** experimental original Rust component, proposed to `dev/ephemeris`.
This is an analytical primitive for future rise/set/transit solvers, not an
ephemeris event time, an apparent horizon model, or a Swiss API replacement.

## Motivation and scope

The VirgoStars backend invokes `swe_rise_trans` for rise, set and transit
features. An accurate event search ultimately requires moving DE441-based
planetary directions, Earth orientation, time-scale conversion, topocentric
parallax, chosen atmospheric/limb conventions, and independent timing checks.
However, the algebraic first step can be implemented and tested separately:
for a target held at a **fixed equatorial declination**, determine which
**local hour angles** give a chosen geometric altitude, and when none exist.

The new `altitude_crossings` module is independent of open PRs
#75 (ERA), #78 (GMST), and #79 (local mean sidereal time). It can later
consume hour angles from a consistently modeled orientation pipeline
without duplicating time or planetary calculations. Actual house
systems and chart application logic remain in `astrological-functions`.

## Scientific definition

The local astronomical hour angle is positive **west** of the meridian.
For observer latitude `φ`, equatorial declination `δ` and geometric
center altitude `h`, the spherical identity is:

```text
sin(h) = sin(φ) sin(δ) + cos(φ) cos(δ) cos(H)
cos(H0) = [sin(h0) - sin(φ) sin(δ)] / [cos(φ) cos(δ)]
H_rise = -H0;  H_set = +H0;  0 < H0 < π  (strict crossings)
```

All three inputs are **finite radians in [-π/2, π/2]**:
- Observer latitude: north-positive, geographically conventional.
- Declination: *fixed* equatorial declination, consistently referred
  to the same equator and time/frame as the hour angle supplied by an
  eventual caller.
- Threshold altitude: geometric altitude of the **center** of the body,
  above an ideal mathematical horizon (positive above). A nonzero
  threshold is a caller-supplied **geometric** value, not proof of
  a specific atmosphere, body limb or observer-height correction.

Return `AltitudeCrossings`:
- `Two { rising_hour_angle_rad, setting_hour_angle_rad }`: one
  eastern negative and one western positive angle.
- `AlwaysAbove` / `AlwaysBelow`: no crossing at any hour angle.
- `TangentAtUpperMeridian` / `TangentAtLowerMeridian`: altitude
  merely touches the threshold at H=0 or H=±π, without crossing.
- `ConstantAtThreshold`: at a polar singularity or celestial-pole
  direction the diurnal amplitude vanishes and the altitude equals
  the threshold to within a documented floating-point guard.

The near-pole numerical threshold is `32 * f64::EPSILON`, used
**only** to avoid dividing by a vanishing amplitude. It is not a
physical accuracy bound or an approximation for polar rise/set timing.
Inputs outside their coordinate domains, including NaN and infinity,
are rejected with typed errors; no implicit unit conversion is made.

## Example

```rust
use virgostars_ephemeris_core::altitude_crossings::{
    geometric_altitude_hour_angles, AltitudeCrossings
};

// Ideal geometrical horizon, at the equator, declination 0°.
let result = geometric_altitude_hour_angles(0.0, 0.0, 0.0)?;
match result {
    AltitudeCrossings::Two {
        rising_hour_angle_rad,
        setting_hour_angle_rad,
    } => println!("H(rise)={rising_hour_angle_rad}; H(set)={setting_hour_angle_rad}"),
    other => println!("No pair of crossings: {other:?}"),
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

At the equator and zero declination the ideal geometric horizon
crossings occur at H=−π/2 and H=+π/2. This does **not** mean
a true Sun rises at 06:00 or sets at 18:00 local time.

## What this calculation intentionally does not do

- No observer longitude, sidereal time, UT1/TT/TDB/UTC/calendar
  conversion, conversion of an hour angle to an event timestamp,
  time zones, Delta T or EOP.
- No ephemeris sampling, evolving declination/RA, light-time,
  topocentric parallax, gravitational bending, nutation/precession
  or true-vs-mean equinox conversion.
- No atmospheric refraction, semidiameter, elevation dip, terrain
  horizon, weather or default pressure/temperature assumptions.
- No choice of apparent versus geometric solar/lunar limb and
  **no accurate `swe_rise_trans` output**.
- Circumpolar classifications here assume **fixed declination**.
  Near a grazing event, real motion and refraction can change
  whether a visible rise/set actually occurs.

The original USNO and IMCCE materials distinguish the
spherical identity from time searches and from atmospheric
rise/set conventions:

- [IMCCE, rise and set of a celestial body](https://promenade.imcce.fr/en/pages3/367.html)
- [US Naval Observatory, altitude/azimuth formula](https://aa.usno.navy.mil/faq/alt_az)
- [USNO, rise/set/twilight definitions](https://aa.usno.navy.mil/faq/RST_defs)

## Verification

The Rust integration test
`crates/ephemeris-core/tests/altitude_crossings.rs`
contains eight deterministic mathematical tests:
- Equatorial horizon H=±π/2 and signed rising/setting convention.
- Independent equivalent zero-horizon `cos H=-tan φ tan δ`.
- Custom −6° geometric altitude and expected ±96° angles.
- Northern/southern polar day/night classifications.
- Exactly polar and celestial-pole degenerate geometry.
- Exact tangent contact at both meridians.
- Reinsert crossing angles into the source spherical identity.
- Nonfinite and out-of-range rejections.

**These invariants do not independently validate real solar/lunar
rise/set times.** Future end-to-end acceptance must compare
independently published USNO/IMCCE event times with source,
body, location, date, atmospheric/horizon convention and tolerance.

Expected read-only CI:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

No new third-party dependencies, data, kernel redistribution or
release process is added. The code and tests carry existing CPAL-1.0
notices with the required VirgoStars attribution unchanged.
