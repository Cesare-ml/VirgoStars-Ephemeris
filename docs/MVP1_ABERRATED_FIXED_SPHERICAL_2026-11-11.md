# MVP 1 — six fixed-frame spherical fields after annual aberration

**2026-11-11 | original Rust; experimental calculation stage, NOT
`swe_calc_ut` or validated apparent ecliptic-of-date astronomy.**

This feature joins the prior numerical stages rather than
reimplementing their astronomy:

```text
externally supplied classic DE441 .eph file
  → Earth barycentric position/velocity/analytic acceleration at TDB reception
  → target emission time and geocentric one-way light-time Cartesian state
  → Earth's special-relativistic annual aberration, including analytic
    derivatives of the observer boost
  → existing fixed-axis Cartesian-to-spherical projection of BOTH
    the aberrated position AND the aberrated reception-time velocity
  → six spherical fields + source/epoch/acceleration metadata
```

The new `src/aberrated_spherical.rs` simply composes existing
`aberration_state::geocentric_light_time_annual_aberration_analytic`
and `coordinates::project_fixed_axes`. This avoids duplicating
the original boost, time-of-flight solver, numerical differentiation
or the spherical velocity equations. The pure
`spherical_from_aberrated_state` accepts an already corrected
`AberratedGeocentricState`, so existing callers can project without
a second correction. `geocentric_aberrated_fixed_spherical` does
the full composition given a `De441<R>` reader and epoch.

**Dependency graph:** this draft branch is based on draft PR
[#76](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/76)
(analytic Chebyshev Earth acceleration), which is based on draft
[#73](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/73)
(analytic aberrated velocity), which depends on draft
[#46](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/46)
(observer aberration). The independent draft
[#70](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/70)
already implements light-time-only fixed-axis **degrees/AU**
six-field conversion. **This change does not copy or replace PR #70**;
when both lines are reviewed/integrated, the common scalar-unit
adapter should be shared, not duplicated. The new interface preserves
the original `SphericalState` rad/km shape specifically to avoid
introducing a second incompatible degree/AU formatter.

## Exact API and scientific contract

```rust
use virgostars_ephemeris_core::aberrated_spherical::{
    geocentric_aberrated_fixed_spherical,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut eph = De441::open("/user-supplied/de441.eph")?;
let result = geocentric_aberrated_fixed_spherical(
    &mut eph,
    Body::Venus,
    JulianDateTdb(2_451_545.0),
    0.0, // zero rotation = ICRF equatorial fixed axes
)?;
println!(
    "{} rad; {} rad/TDB-day",
    result.spherical.longitude_rad,
    result.spherical.longitude_rate_rad_per_day,
);
# Ok(())
# }
```

The function requires a caller-supplied **classic** DE441 binary;
not NAIF SPK `.bsp`, not Swiss `.se1`, and never downloads a file.
Input epoch is **TDB Julian date**, not `swe_calc_ut` Julian UT.
The passive X-tilt is **constant in time**. A nonzero tilt may be
useful for mathematical tests but must **not** be called official
J2000 or true ecliptic of date; no rotating-frame rate term
`dR/dt × r` is added.

`AberratedFixedSpherical.spherical` provides **six actual fields**:
- longitude in **radians [0,2π)**, latitude in radians;
- geocentric distance in **kilometres**;
- longitude and latitude angular speed in **rad/reception-TDB-day**;
- radial distance rate in **km/reception-TDB-day**.

It additionally preserves `Body` target identity (important:
JupiterBarycenter is not Jupiter's center), `Body::Earth` as center,
reception TDB date, target emission TDB date, one-way light time
and its time rate, and the
`ObserverAccelerationSource::AnalyticDe441Chebyshev` provenance.
The pure converter also preserves `CallerProvided` provenance for
already-computed states. Projection uses only the *existing*
validated geometric/singular-direction checks and rejects malformed
time/delay metadata; it cannot authenticate source arrays.

## Tests and result limitations

New `tests/aberrated_spherical.rs` has **seven original mathematical
offline regressions**:

1. independent hand-derived six-component spherical position/velocity
   with zero observer boost, including metadata and Earth center;
2. exact **transverse Lorentz** result
   `longitude=acos(beta)`,
   `dlongitude/dt = -beta_dot/sqrt(1-beta²)`;
3. a caller-chosen constant 90° X-tilt rotating **both** Cartesian
   position and velocity, with independently derived latitude speed;
4. 270° longitude normalization and preserved negative retrograde speed;
5. invalid Earth target, reversed reception/emission epochs and
   invalid time/delay metadata rejected before projection;
6. invalid tilt and pole singularities rejected, no NaN fallback;
7. centered differences of **independently aberrated source
   directions** compared to the analytic spherical longitude rate.

The existing synthetic quadratic `tests/de441_analytic_acceleration.rs`
adds **two** tests, including an eighth regression which runs the
entire existing classic DE441 reader → analytic Earth acceleration →
Newtonian light-time → observer aberration → six fixed spherical fields
and confirms that nonzero observer velocity changes the geometric
source direction. A ninth test exercises the geocentric **Moon**
reconstruction through that same pipeline and rejects the undefined
geocentric Earth-as-target direction.

**These are synthetic/mathematical unit and composition tests, NOT a
full numerical independent JPL/Horizons or Swiss Ephemeris validation
of a real geocentric apparent longitude.** No source file/planetary
scientific fixture was embedded.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test aberrated_spherical
cargo test --workspace --locked --test de441_analytic_acceleration
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Missing work to reach MVP-1 Swiss substitution

The result is the **fixed ICRF after geometric light-time and observer
annual aberration**. It omits solar gravitational light deflection,
Shapiro delay, additional relativistic potential terms, frame
bias/precession, *complete* IAU 2000A nutation, **moving
true-ecliptic-of-date frames and their rates**, UT/TT/TDB conversion,
topocentric observer changes and physical planet centers for the outer
DE441 system barycenters. It has no fully validated speed accuracy
against Swiss 0.5.17 and no Node binding or `swisseph`-shaped
return contract. Only after these gates pass may outputs be described
as truly apparent or Swiss compatible.

**2026-11-11 CPAL-1.0 modification record:** new original
`src/aberrated_spherical.rs`,
`tests/aberrated_spherical.rs`, one additional test in
`tests/de441_analytic_acceleration.rs`, `lib.rs` module export,
this document and a README link. No third-party programs,
official coefficient datasets, JPL binary, dependencies, license
changes, package release, deploy or protected-branch merge. Preserve
CPAL-1.0 Exhibit A notices and **Powered by VirgoStars**
<https://www.virgostars.it/> attribution.
