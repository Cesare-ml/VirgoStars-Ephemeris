# Moving-frame Cartesian velocities

**2026-11-10 — Experimental Rust mathematical primitive, not a precession/nutation implementation or Swiss Ephemeris parity.**

## Scientific purpose

The existing `coordinates::project_fixed_axes` implements a constant rotation
of a state: it intentionally holds its axes fixed when differentiating
spherical coordinates. **That is insufficient for rates in a rotating
reference frame** such as the mean/true ecliptic and equator of date.

The new `rotating_frame::RotatingFrame` accepts a caller-supplied, passive,
right-handed 3×3 orthonormal rotation `R(t)` and its derivative
`Rdot(t)=dR/dt`, both referring to **the same reception epoch**.
It transforms a Cartesian state as

```text
r_moving = R(t) * r_inertial
v_moving = R(t) * v_inertial + Rdot(t) * r_inertial
```

The second term is the apparent movement induced by the changing coordinate
axes. Omitting it can give incorrect planetary longitude/latitude rates and
can even hide or create apparent stations/retrogradations. The transform
preserves the **physical distance and radial distance rate** under a valid
orthonormal rotation.

## Input and output contract

- **Source:** Cartesian target-relative-to-center `de441::StateVector`
  in equatorial ICRF-aligned *inertial* axes, position km, velocity km/TDB
  day, epoch explicitly `JulianDateTdb`. The original state can be geometric
  or a separately constructed corrected state as long as its velocity is
  consistently the derivative with respect to observation TDB and frame.
- **Frame:** `R(t)` maps *inertial* components to the required rotated axes.
  `Rdot(t)` is **per TDB observation day** (even if a future physical
  model uses a TT argument internally, its derivative must be converted to
  the declared TDB-day parameter). No time conversion is applied here.
- **Result:** same target, center, epoch, km and km/TDB-day units, but
  position and velocity components now belong to the provided moving axes.
- **Validation:** finite matrices and state, a proper (`det R≈+1`)
  orthonormal rotation (`R Rᵀ≈I`), and a compatible tangent derivative
  `Rdot Rᵀ + R Rdotᵀ≈0`. Invalid reflections, warped matrices,
  incompatible derivatives and arithmetic overflow produce typed errors.
  This checks *internal consistency* and cannot establish the physical
  accuracy of an externally supplied frame.

**Important API limitation:** The preexisting `StateVector` does not carry
an explicit frame label. `rotate_state` returns that same struct for
compatibility with the existing Cartesian/spherical projection, so consumers
**must track the output frame externally** and never pass an already rotated
state to functions requiring ICRF inputs. A future public frame-typed API
should make that contract impossible to violate at compile time.

## Demonstration: rotation about +Z

For a **passive** rotation of axes by angle `θ`:

```text
R = [[ cosθ,  sinθ, 0],   Rdot = θdot*[[-sinθ,  cosθ, 0],
     [-sinθ,  cosθ, 0],                  [-cosθ, -sinθ, 0],
     [    0,     0, 1]]                  [    0,     0, 0]]
```

A stationary inertial source at `[384400,0,0] km` with
`θ=0`, `θdot=+0.1 rad/TDB day` has an **output** velocity of
`[0,-38440,0] km/TDB day` and a longitude rate of
`-0.1 rad/TDB day`. The fixed-axis `R*v` alone incorrectly gives
zero, which is why this stage is essential.

Use with the already existing `coordinates::project_fixed_axes` at
`tilt_rad=0` **only after the rotation has already been applied**; adding
another fixed obliquity would apply a second rotation.

## Regression scope and commands

`crates/ephemeris-core/tests/rotating_frame.rs` contains six synthetic,
deterministic tests covering:

1. Identity axes and metadata preservation.
2. A stationary ICRF target acquiring a nonzero (retrograde) longitude
   rate exclusively from axis rotation.
3. Independent centered position-difference checks of Cartesian and
   spherical rates with **both** source motion and frame rotation.
4. Invariance of range and physical radial speed for multiple rotations.
5. Rejection of reflections, nonorthonormal matrices and inconsistent
   matrix derivatives.
6. Nonfinite matrices/epochs/velocities and arithmetic overflow.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test rotating_frame
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

All tolerances in the tests are numerical *mathematical* regression criteria
on synthetic inputs, not scientific precision guarantees for real planets.

## Explicitly missing

This module **does not calculate** IAU 2006 precession, IAU 2000A
nutation, frame bias, the angle `R(t)`, its physically modeled derivative,
TT/TDB conversion, diurnal rotation, aberration, gravitational deflection,
refraction, or geocentric apparent positions. It is not by itself the true
ecliptic/equinox of date or a substitute for Swiss `swe_calc_ut`.
An authoritative physical frame and its derivative must be implemented and
independently validated before calling the output apparent-of-date.

## CPAL modification record — 2026-11-10

Added `src/rotating_frame.rs`, the `lib.rs` export, a standalone six-test
regression `tests/rotating_frame.rs`, and this scientific document and
README link. Original Rust implementation, no added dependencies or
redistributed JPL/NAIF/Swiss source/data. Existing CPAL-1.0 notices,
`LICENSE`, `NOTICE` and mandatory `Powered by VirgoStars`
attribution are unchanged.
