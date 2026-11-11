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

## Follow-up — geocentric moving-axis spherical composition (2026-11-11)

**New original module:** `crates/ephemeris-core/src/moving_geocentric.rs`.
The previous `RotatingFrame::rotate_state` correctly implements the
mathematical rate equation but only returned Cartesian vectors.
Now the added `moving_spherical_from_light_time` connects that
**existing rotation** to the **existing spherical projector** and
returns the six quantities required by chart consumers:

- `longitude_rad`, `latitude_rad`, `distance_km`;
- `longitude_rate_rad_per_day`, `latitude_rate_rad_per_day`,
  `distance_rate_km_per_day`.

This is `R(t) r(t)` and **`R(t) v(t) + Rdot(t) r(t)`**, then
the project's original spherical geometric projection, called with
`tilt_rad = 0`: **zero additional constant X-axis rotation after
the general moving-frame transform**, NOT an assertion that the
output is still fixed ICRF. The `MovingGeocentricSpherical`
wrapper deliberately communicates the **moving-axis** semantics and
retains `SphericalState` together with the original emission TDB epoch,
one-way light-time and its derivative. It also preserves the body's
exact DE441 target variant (including system barycenters),
`Body::Earth` as observer and reception TDB epoch.

The companion `geocentric_light_time_moving_spherical` is a
**single composed offline Rust call**:

```text
external classic DE441 .eph → geocentric_light_time (Earth reception,
target emission and reception-time rates) → caller-provided RotatingFrame
(R, Rdot) → existing fixed-axis spherical projector (zero extra tilt)
→ six moving-frame spherical fields and provenance
```

```rust
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::moving_geocentric::{
    geocentric_light_time_moving_spherical,
};
use virgostars_ephemeris_core::rotating_frame::RotatingFrame;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut eph = De441::open("/user-supplied/de441.eph")?;
let identity = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];
let frame = RotatingFrame::new(identity, [[0.0; 3]; 3])?;
let sample = geocentric_light_time_moving_spherical(
    &mut eph, Body::Venus, JulianDateTdb(2_451_545.0), &frame,
)?;
println!("{}", sample.spherical.longitude_rate_rad_per_day);
# Ok(())
# }
```

The example is **not** an IAU reference-frame model or a verified
astrological longitude; without an external file the numerical
sample cannot be run. The caller must supply **R and dR/dTDB** for
**the same reception epoch** in fixed ICRF input axes. The
`RotatingFrame` type has no embedded time-scale or model metadata
and cannot verify that the matrix is actually IAU 2006, 2000A,
or geocentric true-ecliptic-of-date. **Do not feed raw derivatives
from draft PRs #48/#51 (per TT day) directly to this per-TDB-day API
without correctly validated TT→TDB chain rule and epoch agreement.**
No automatic UT↔TT↔TDB conversion is implemented. A constant
`RotatingFrame` with zero `Rdot` is a valid fixed-axis control
case; it is not a substitute for a date-dependent frame.

### New synthetic validation

`tests/moving_geocentric.rs` includes **9 deterministic offline
mathematics/integration tests**:

1. identity frame reproduces all six original fixed-axis fields,
   original Earth center, target and light-time metadata;
2. stationary ICRF source acquires **negative moving longitude speed**
   in rotating Z axes (the `Rdot*r` contribution);
3. rotating X axes create nonzero **latitude** velocity even if a
   source is stationary in ICRF;
4. independent centered differences of source motion **and**
   moving-axis orientation confirm all three angular/radial rates;
5. invariant radial distance and radial speed for several rotating
   orientations;
6. fail-closed invalid reception/emission metadata, undefined Earth
   target, nonfinite state/time data;
7. singular polar direction and explicit Jovian **system barycenter**
   identity preserved, no fake planet center;
8. actual **synthetic classic-DE441 in-memory parser → reception
   light time → rotating frame → six spherical fields**;
9. errors from an undefined Earth geocentric target or absent Sun
   series propagate rather than inventing positions.

All tests are **synthetic math/structural**, NOT independent
NASA/JPL Horizons planet positions, valid IAU date-frame matrices,
actual Swiss `swe_calc_ut` 6-value compatibility, or proof
of real physical precession/nutation accuracy.

### Coordination and remaining gates

This is an additive enhancement to **existing draft PR #47**, so
its independently authored moving frame is reused, not copied.
The newer draft #77 uses a separate unmerged branch with
**one-way light-time + observer annual aberration and all three rates**.
After appropriate PR review/integration, the same moving-frame
projection can accept those **aberrated** Cartesian vectors via
an adapter, avoiding double light-time or double aberration.
That latter integration is **not yet implemented**, and this module
only applies geometric light-time to the input before rotation.
The new output remains in radians/km and TDB-day derivatives; a
separate draft #77 already owns a pure **degrees/AU** formatter
for fixed-axis spherical states, to be shared rather than duplicated
when the code paths converge.

To reach physical apparent-of-date results we still need the
complete scientifically verified precession/nutation matrix and
**dR/dTDB** with correct TT/TDB chain rule, solar relativistic
deflection, correct planetary centers, robust UTC/TT/TDB conversion,
actual independent JPL/SOFA/Swiss comparison fixtures and Node
load/binding parity. No current consumer can claim this work
alone replaces Swiss.

**CPAL-1.0 modification record — 2026-11-11:** added original
`src/moving_geocentric.rs`, `tests/moving_geocentric.rs`,
one `lib.rs` module export and this documentation section/README
link. Preserve source Exhibit A notices and **Powered by VirgoStars**
(https://www.virgostars.it/) attribution. No external astronomy
datasets, JPL files, third-party program code, new dependencies,
production change, merge, release or deploy.
