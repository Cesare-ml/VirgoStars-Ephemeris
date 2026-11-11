# IERS TIRS → CIRS Earth rotation and spin velocity — 2026-11-12

**Status:** experimental, independently implemented Rust primitive proposed as
a draft PR against `dev/ephemeris`, not available on `main` or in production.

## Position in the VirgoStars physical pipeline

The intended complete observer pipeline must distinguish reference frames:

```text
WGS84 geodetic site
   │  PR #82: position in Earth-fixed XYZ, meters → kilometers
   ▼
ITRS / terrestrial ECEF
   │  W(t) polar motion and TIO locator s′: NOT IMPLEMENTED HERE
   ▼
TIRS (terrestrial intermediate, km)
   │  R3(-ERA): THIS PR, from IAU 2000 ERA(UT1) in PR #75
   ▼
CIRS (celestial intermediate, km; includes only Earth spin in derivative)
   │  Q(t) CIP / CIO orientation, precession/nutation: NOT IMPLEMENTED HERE
   ▼
GCRS / near-ICRF, compatible-frame geocentric and topocentric geometry
   │  observer/target state, ephemeris and apparent corrections separately
   ▼
apparent sky / horizon / chart functions
```

A WGS84 ECEF XYZ from PR #82 is **not** strictly a TIRS vector without
polar motion W(t), even at the same epoch. A CIRS position is
**not** a GCRS or JPL DE441 ICRF position. The missing polar-motion,
precession-nutation/CIP/CIO, geocenter translation (where needed),
time-scale and ephemeris stages must be implemented, verified and
composed separately. Do **not** subtract the output of this PR from
DE441 vectors as though both were in ICRF.

## Scientific definition and input contract

IERS Conventions (2010) Technical Note 36, section 5.4.2,
equation 5.5, expresses the TIRS→CIRS Earth rotation step as

```text
R(t) = R3(-ERA(UT1))       [IERS/SOFA passive rotation convention]

x_CIRS = cos(ERA) x_TIRS - sin(ERA) y_TIRS
y_CIRS = sin(ERA) x_TIRS + cos(ERA) y_TIRS
z_CIRS = z_TIRS
```

The sign convention is important: positive ERA moves a terrestrial
+X vector toward +Y in the CIRS column-vector components. The inverse
position-only map uses the transpose rotation.

Inputs:
- `position_tirs_km: [f64;3]`: **finite kilometers**, relative to
  Earth's center in an already-realized TIRS frame, with
  the TIO as x-axis and CIP as z-axis. Do not feed raw ITRS without
  W(t) polar motion unless this approximation has been separately
  selected and documented.
- `era_rad: f64`: finite **IAU 2000** ERA in **[0,2π)** radians,
  calculated at a particular UT1 epoch from an explicitly obtained
  UT1 time. The function checks format but cannot prove provenance.
- `era_rate_rad_per_ut1_day: f64`: positive finite spin angular
  rate in **radians per UT1 day**, from the same ERA model
  (IAU 2000 constant `2π × 1.00273781191135448` rad/UT1 day).
  The code does not independently convert UT1 or compute the rate.

Outputs:
- `CirsEarthSpinState.position_km`: rotated CIRS intermediate-frame
  position in kilometers.
- `CirsEarthSpinState.spin_velocity_km_per_ut1_day`: **only**
  derivative of the CIRS position with respect to a changing ERA,
  holding the TIRS position **fixed**:

```text
v_spin = omega_UT1 cross position_CIRS
       = [-omega*y_CIRS, omega*x_CIRS, 0] km/UT1 day
```

This is not a complete physical observer velocity: changes in W(t),
Q(t), plate deformation, other observer motion, EOP, and any time-scale
derivatives are **excluded**. In particular it is not a complete
d(position_ICRF)/d(TDB) velocity. The function
`cirs_to_tirs_position` inverts the **position-only** R3 step;
a velocity requires an appropriately differentiated frame transform.

Input validation distinguishes non-finite position, missing/wrong ERA
normalization, invalid/nonpositive rate and a non-finite derived output.
Both routines do **no** network calls or data access.

## Rust example

```rust
use std::f64::consts::TAU;
use virgostars_ephemeris_core::tirs_cirs_era::{
    cirs_to_tirs_position, tirs_to_cirs_earth_spin
};

// Illustrative TIRS coordinate only; not raw WGS84/ITRS.
// ERA must be computed from a genuine UT1 epoch upstream.
let position_tirs_km = [6378.137, 0.0, 0.0];
let era = 1.0;
let omega_per_ut1_day = TAU * 1.002_737_811_911_354_6;
let state = tirs_to_cirs_earth_spin(
    position_tirs_km,
    era,
    omega_per_ut1_day,
)?;
let recovered_tirs = cirs_to_tirs_position(state.position_km, era)?;
println!("CIRS: {:?}, spin: {:?}, reverse: {:?}",
    state.position_km, state.spin_velocity_km_per_ut1_day, recovered_tirs);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Verification

Eight deterministic Rust integration test groups exercise:

1. ERA=0 and the correct eastward spin direction.
2. ERA=π/2 sign of the IERS/SOFA R3(-ERA) mapping.
3. Fixed terrestrial spin-axis position has zero spin-only velocity.
4. Roundtrip inverse at varied terrestrial positions and ERA values.
5. Rotation preserves Euclidean length; spin velocity is perpendicular
   to the spin circle.
6. Analytic spin rate agrees with the symmetric finite difference
   in an ERA value progressing with its UT1-day rate.
7. Ideal WGS84 equatorial radius plus IAU 2000 spin rate gives
   about **465.101094 m/s** at the idealized equator.
8. Strict rejection of invalid positions, ERA, rates and overflow.

These are mathematical reference-convention and numerical-invariant
tests, **not** independent ITRS→GCRS, DE441, JPL/Horizons, exact
observer-velocity, ephemeris or Swiss-parity validation.

Scientific sources:
- [IERS Conventions (2010), Chapter 5, Eq. 5.5](https://iers-conventions.obspm.fr/conventions_material.php)
- [SOFA C2T/C2I documentation for R3(ERA) and RPOM](https://juliaastro.org/SOFA/stable/api/precession/)
- [NGA WGS84 geodetic standards](https://earth-info.nga.mil/index.php?dir=wgs84&action=wgs84)

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

No new dependencies, third-party code/data, JPL kernels, package
publishing, protected-main merge or production changes. The original
CPAL-1.0 notice and VirgoStars attribution remain unchanged.
