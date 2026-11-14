# Geometric equatorial-to-horizontal projection — 2026-11-11

**Status:** experimental Rust core primitive, proposed to `dev/ephemeris`;
not a Swiss Ephemeris replacement and not integrated in production.

## Why this is useful

VirgoStars currently calls `swe_azalt` and `swe_rise_trans`. A verified
horizontal-coordinate transform is an intermediate requirement for their
event and sky-display geometry. The initial `first_sky.rs` prototype
has a **private** vector-based horizontal transformation tied to
an explicitly approximate model; the new public
`equatorial_horizontal` module accepts a mathematically explicit
equatorial direction and makes its angular conventions and error cases
testable without ephemeris files or a clock.

This PR does **not** duplicate Earth rotation (PR #75), IAU 2006 GMST
(PR #78), local mean sidereal time and hour angle (PR #79), or
fixed-declination altitude crossings (PR #80). After independent
review these primitives may be composed, with a shared model/time
scale and independent numerical validation.

## Exact mathematical contract

Input:
- `hour_angle_rad`: signed local hour angle H in **[-π, π)** radians,
  **positive west** of the meridian.
- `declination_rad`: equatorial declination δ in **[-π/2, π/2]** radians.
- `latitude_rad`: observer adopted astronomical latitude φ,
  **positive north**, in **[-π/2, π/2]** radians. Geographic latitude
  may be an approximation; polar motion, vertical deflection and
  topocentric adjustments require explicit upstream modeling.

All angles must be finite; out-of-range values are **errors** rather
than silently normalized. The hour angle and declination **must be in
the same adopted equatorial frame and equinox**, for the same physical
direction and epoch. Mixing mean-of-date H with apparent-of-date
declination or RA is not supported.

The local Cartesian components of the unit direction are:

```text
east  = -cos(δ) sin(H)
north =  sin(δ) cos(φ) - cos(δ) sin(φ) cos(H)
up    =  sin(δ) sin(φ) + cos(δ) cos(φ) cos(H)
```

Outputs:
- `altitude_rad = atan2(up, hypot(east, north))`, in [-π/2, π/2].
- `azimuth_rad = atan2(east, north) mod 2π`, in [0, 2π);
  **north 0°, east 90°, south 180°, west 270°**.
- `azimuth_rad = None` at geometric zenith/nadir, where the
  horizon-projected vector has negligible length, or at the **exact
  geographic poles**, where a unique local north bearing is not
  physically defined. The tiny projected-length guard of
  `32*f64::EPSILON` is numerical only, not a physical accuracy claim.
- Non-finite or invalid inputs and results raise typed
  `HorizontalError` values.

The convention of azimuth from **north through east** is the opposite
reference direction from the Swiss `swe_azalt` convention of south
through west; a future compatibility layer must explicitly apply and
test the required 180° convention conversion rather than silently
relabeling these angles.

## Rust usage

```rust
use virgostars_ephemeris_core::equatorial_horizontal::{
    equatorial_to_horizontal
};

// Input is an illustrative *already-computed* equatorial direction.
let horizontal = equatorial_to_horizontal(
    1.1, // signed local hour angle, rad (west positive)
    1.2, // equatorial declination, rad
    0.3, // adopted observer latitude, rad
)?;
println!("Geometric altitude={} rad, azimuth={:?}",
    horizontal.altitude_rad, horizontal.azimuth_rad);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Independent verification

The Rust test `equatorial_horizontal.rs` includes the **independently
published** SOFA/ERFA `t_hd2ae` regression:

```text
eraHd2ae(hour_angle=1.1, declination=1.2, latitude=0.3)
expected azimuth  = 5.916889243730066194 rad (tolerance 1e-13)
expected altitude = 0.4472186304990486228 rad (tolerance 1e-14)
```

This is a directly recorded SOFA/ERFA horizontal conversion reference,
not a value derived from the old VirgoStars code. Other tests verify
east/west horizon directions, northern/southern meridian cases,
zenith/nadir and exact-pole singularities, the source sine-altitude
equation, signed hour-angle symmetry and rejection of non-finite
or nonnormalized inputs.

Scientific references:
- [USNO, Computing Altitude and Azimuth from GAST](https://aa.usno.navy.mil/faq/alt_az)
- [ERFA `t_hd2ae` independent expected values](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c)
- [SOFA `hd2ae` angular conventions](https://juliaastro.org/SOFA/stable/api/equatorial/)

### Deliberate omissions / later gates

This does **not** infer local sidereal time or a time instant, and does
not apply geocentric-to-topocentric parallax, light-time, aberration,
precession, nutation, EOP/UT1, refraction, observer height, horizon
dip, body semidiameter or meteorological defaults. It does not compute
rising or setting times, identify `swe_azalt` input/output flags, or
validate apparent planetary positions. The independent ERFA scalar
establishes conversion math for one case, **not** numerical agreement
of a complete orbital or chart pipeline.

### CI and license

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Original Rust implementation and tests preserve the existing CPAL-1.0
source notice. No Swiss/ERFA source is copied or linked into the build;
the reference is an independently published test **output**. No
new dependencies, third-party data, large kernels, release, automatic
deployment or production changes are introduced. The new code is a
reviewable draft PR, not on protected `main`.
