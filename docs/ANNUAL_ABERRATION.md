# Experimental geocentric annual aberration stage

**Added 2026-11-10. Status: standalone independently implemented Rust direction transform; NOT a validated complete apparent-position pipeline, ephemeris replacement, or Swiss Ephemeris parity.**

## Purpose and boundaries

`crates/ephemeris-core/src/aberration.rs` exports
`annual_aberration_icrf(source_position_km, earth_barycentric_velocity_km_per_day)`.
This is a **special-relativistic Lorentz boost of the source-pointing photon
direction**, correcting for the Earth's barycentric velocity at the moment of
observation (annual aberration). It operates only on a direction; it does not
produce a position or an angular rate.

Contract:

| Dimension | Required meaning |
|---|---|
| Source | Vector **from Earth's geocenter toward the target**; for planets preferably the separately computed reception-light-time-retarded line of sight, NOT a barycentric target position |
| Observer | Earth geocenter moving with **Earth's barycentric velocity at reception** |
| Input frame | ICRF-oriented fixed barycentric inertial axes for both vectors |
| Epoch | Observer and direction must correspond to the chosen **observation TDB epoch**; no time-scale conversion occurs in this function |
| Units | Source vector in km (distance intentionally discarded); Earth velocity in km per TDB day; `c = 299792.458 km/s` |
| Output | Dimensionless unit vector **towards the aberrated source** in fixed ICRF axes |
| Included | Only observer-velocity special-relativistic light aberration |
| Excluded | Gravitational light deflection and potential, Shapiro delay, nutation/precession/frame bias, geodetic/topocentric observer, polar motion, diurnal aberration, refraction and all angular-rate derivatives |

**Do not** label the returned vector `apparent geocentric true-ecliptic-of-date`,
or feed it directly to the strict `swe_calc_ut` reference comparator.
Other corrections and changing-frame derivatives are still required.
This function does not copy, link or invoke Swiss Ephemeris or NAIF SPICE.

## Independent physical model

In the inertial frame, let `n` be the unit vector pointing from the observer
**towards** the source; let `beta = v/c` be the observer's barycentric velocity
in that same frame and `w = sqrt(1 - |beta|²)`. The direction observed in the
instantaneous comoving inertial frame is

```text
n_observed = [w*n + beta*(1 + dot(n,beta)/(1+w))] / (1 + dot(n,beta))
```

The output is normalized to a unit vector to control roundoff. For a boost
along +X the equivalent scalar one-dimensional relation is

```text
cos(theta_observed) = (cos(theta) + beta) / (1 + beta*cos(theta))
sin(theta_observed) = sqrt(1-beta²)*sin(theta)/(1 + beta*cos(theta))
```

These follow directly from the special-relativistic Lorentz transformation
of light propagation; the sign convention is for direction **to the source**,
not the propagation direction of the arriving photon.

Independent physical references (equations/conventions only; no code reused):
- [Richard Fitzpatrick, University of Texas at Austin, *Relativistic Aberration of Light*](https://farside.ph.utexas.edu/teaching/355/Surveyhtml/node139.html) — angle transformation and terrestrial orbital-speed scale.
- [David W. Hogg, *Special Relativity*, §7.2 stellar aberration](https://cosmo.nyu.edu/hogg/sr/sr.pdf) — source deflection toward the observer's motion.
- [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/manual.html) — eventual independent full-pipeline comparison; none performed for this stage.

This pure special-relativistic model intentionally omits the gravitational
potential and other refinements needed for a fully relativistic IAU astrometry
pipeline; such refinements require separate validation.

## Combining two current Rust stages (illustrative, not Swiss parity)

```rust
use virgostars_ephemeris_core::aberration::annual_aberration_icrf;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::light_time::geocentric_light_time;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut ephemeris = De441::open("/path/to/external/de441.eph")?;
let tdb_observation = JulianDateTdb(2_451_545.0);
let light_time = geocentric_light_time(
    &mut ephemeris,
    Body::MarsBarycenter,
    tdb_observation,
)?;
let earth = ephemeris.state(
    Body::Earth,
    Body::SolarSystemBarycenter,
    tdb_observation,
)?;
let direction_icrf = annual_aberration_icrf(
    light_time.position_km,
    earth.velocity_km_per_day,
)?;
println!("{direction_icrf:?}");
# Ok(())
# }
```

The external DE441 path must be supplied separately. The sample returns a
**light-time + annual-aberration corrected** fixed-ICRF direction. It does NOT
yield a complete apparent position. In particular it does not yield a
Swiss-compatible ecliptic longitude, latitude, or daily rates. Current DE441
outer planets are system barycenters rather than individual planetary centers.

## Deterministic regression tests

`crates/ephemeris-core/tests/annual_aberration.rs` tests:

- Zero observer speed: input direction normalization without deflection.
- Exactly perpendicular boost: `(beta, sqrt(1-beta²), 0)` against the
  independent scalar Lorentz transformation.
- Oblique boost: both sine and cosine relations, and inverse-velocity boost.
- Parallel/anti-parallel source: no change of direction.
- Earth-like transverse speed **29.78 km/s**: **20.4893945 arcseconds** shift;
  this is a physical-scale check, not a real planetary observation.
- Range independence; unit norm; large but subluminal speeds.
- Zero/nonfinite source, nonfinite or superluminal observer speed, and
  unrepresentably large input: explicit errors rather than silent fallback.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test annual_aberration
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

All numerical tests are **analytical/synthetic physics fixtures**. Passing
them validates this Lorentz stage, not the numerical accuracy of NASA/JPL
ephemerides, the light-time implementation, an IAU full apparent model or
Swiss backend output parity. Independent external numeric position checks,
separate aberration-rate derivatives and full apparent-of-date comparison
remain later milestones.

## CPAL-1.0 modification record

2026-11-10 — Added `crates/ephemeris-core/src/aberration.rs`, exposed the
module from `src/lib.rs`, added the analytic regression suite
`tests/annual_aberration.rs` and created this documentation. Source code is
original, written for VirgoStars-Ephemeris under existing CPAL-1.0 with its
source notices and the original VirgoStars attribution retained. No new
dependency, Swiss/NAIF source, ephemeris kernel, binary or private data is
incorporated. No change to `LICENSE`, `NOTICE` or production integrations.
