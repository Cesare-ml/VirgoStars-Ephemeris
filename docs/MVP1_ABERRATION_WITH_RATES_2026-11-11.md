# MVP 1 — annual aberration **with reception-time velocities**

**2026-11-11 | Original Rust, experimental, fixed ICRF.** This branch
is **stacked on draft PR #46**, which contributes the existing
`annual_aberration_icrf` special-relativistic direction law.
**Do not merge this draft until #46 has been reviewed and integrated.**

This module fills a concrete MVP-1 gap: the existing annual aberration
correction returned **direction only**, while actual Swiss-style planet
positions need longitude/latitude **and their time derivatives**.
The new `src/aberration_state.rs` supplies a **Cartesian reception-time
position and velocity** after Newtonian one-way light-time and
special-relativistic annual aberration. The separate draft
[PR #70](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/70)
already projects light-time vectors to six fixed-frame fields, but **has not
yet been merged/integrated with this stage**.

## Public API / physical contract

```rust
use virgostars_ephemeris_core::aberration_state::{
    geocentric_light_time_annual_aberration
};
use virgostars_ephemeris_core::de441::{Body,De441,JulianDateTdb};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut eph = De441::open("/externally/obtained/de441.eph")?;
let corrected = geocentric_light_time_annual_aberration(
    &mut eph,
    Body::Venus,
    JulianDateTdb(2_451_545.0),
    0.02, // caller-chosen centered Earth-acceleration step, TDB days
)?;
println!("{:?}", corrected.velocity_km_per_tdb_day);
# Ok(())
# }
```

The example requires an external legally obtained **classic DE441 binary**.
It has not been executed here against that external file. **Do not**
treat its numerical output as independently validated or as apparent
true-ecliptic-of-date coordinates. No `.bsp` SPK, Swiss `.se1` or
proprietary file is bundled or auto-fetched.

The pure
`aberrate_light_time_state(&GeocentricLightTimeState,
earth_barycentric_velocity_km_per_tdb_day,
earth_barycentric_acceleration_km_per_tdb_day2)` accepts a previously
calculated **Earth-centered reception light-time** vector and the
observer's **barycentric** velocity and acceleration in the same
fixed ICRF frame and at the **same observation TDB epoch**. **These
inputs must NOT be target geocentric speed or acceleration**, and the
arrays carry no self-verifiable frame or epoch identifiers. They remain
a caller responsibility. There is no automatic TT↔TDB/UTC/UT1 conversion.

The one-call
`geocentric_light_time_annual_aberration(&mut De441, target,
epoch_tdb, acceleration_step_tdb_days)`:
1. reads the Earth barycentric velocity at reception;
2. samples the same Earth ephemeris at **t−h** and **t+h**;
3. estimates Earth acceleration in km/TDB-day² by the **centered**
   difference `[v(t+h)-v(t-h)]/(2h)`;
4. calls the existing `light_time::geocentric_light_time` to obtain
   a target emission vector, Earth reception vector and correct
   reception-time geometric derivative;
5. converts that state with the **existing** `annual_aberration_icrf`
   direction and the newly derived analytic time derivative.

The caller **must** supply `0 < h <= 0.5` TDB days and ensure
`t±h` is representable and inside the file. In particular, this is
a **numerical Earth acceleration**, *not an analytic second derivative
of the Chebyshev record*. The error depends on `h`, original file
precision, record boundaries and kinematics and is **not bounded**
by a demonstrated astronomical benchmark in this change.
The output records
`ObserverAccelerationSource::CenteredEphemerisVelocity {step_tdb_days}`
or `CallerProvided`, so downstream code cannot silently ignore this
approximation.

### Derivative definition (original calculus, no third-party source code)

Take the one-way reception light-time vector `r(t)` with already
correctly differentiated geometric velocity `u=dr/dt`, `R=|r|`,
`n=r/R`, and observer velocity `v` and acceleration `a` in
km/TDB-day and km/TDB-day². Let
`beta=v/c`, `beta_dot=a/c`,
`s=sqrt(1-|beta|²)`, `k=n·beta`, and
`q=1+k/(1+s)`. The previously published #46
Lorentz boost yields the aberrated unit direction along the vector

```text
y = s*n + q*beta
n_ab = y/|y|                  (equivalent to the existing normalized boost)
```

The numerator of the full boost also has a strictly positive
denominator `1+n·beta`, which cancels upon normalization. By the
chain rule,

```text
Rdot   = dot(n,u)
ndot   = (u - n*Rdot)/R
sdot   = -dot(beta,beta_dot)/s
kdot   = dot(ndot,beta)+dot(n,beta_dot)
qdot   = kdot/(1+s) - k*sdot/(1+s)^2
ydot   = sdot*n + s*ndot + qdot*beta + q*beta_dot
n_abdot= [ydot - n_ab*dot(n_ab,ydot)] / |y|
r_ab   = R*n_ab
r_abdot= Rdot*n_ab + R*n_abdot
```

**All dotted quantities here are derivatives per observation TDB day.**
The source range and radial speed are preserved by the directional
correction. The implementation **calls** the prior #46
`annual_aberration_icrf` for the actual corrected source direction,
rather than copying the Lorentz direction routine into a new module.
Error checking rejects non-finite inputs, invalid epoch/delay metadata,
undefined or unphysical direction/superluminal observer speed, and
non-finite derivative output.

## Validation: what the tests prove

`tests/aberration_state.rs` contains **9** deterministic offline
mathematical / synthetic regressions:
1. Zero observer velocity/acceleration reproduces the existing
   light-time vector **and all three Cartesian rates**.
2. Independently known transverse Lorentz solution
   `n_ab=(beta,sqrt(1-beta²),0)` **and exact observer-acceleration derivative**.
3. **Centered numerical differences** of the existing independent #46
   direction routine for a curved moving target and accelerating Earth,
   compared against all three analytically calculated velocity components.
4. Source range and **radial rate invariance**, while the source
   direction matches the original #46 transform exactly.
5. No motion/no acceleration produces no artificial speed.
6. Reject NaN/inf observer velocities/accelerations and speeds >=c.
7. Reject Earth-as-target, invalid emission/reception geometry and
   invalid source input.
8. A genuine **offline one-call test through a synthetic classic DE441
   binary**, Earth barycenter, light-time solver, finite-difference Earth
   acceleration and new aberration-rate transform.
9. Reject nonrepresentable, zero or oversized acceleration steps,
   out-of-coverage Earth acceleration samples and undefined Earth
   target without falling back silently.

All those tests are **synthetic mathematical validation**. They are
**not full independent JPL/Horizons, SOFA/ERFA or Swiss reference
parity** for an apparent ecliptic-of-date longitude and velocity.
The existing direction routine's nine original tests must still pass.

## Scientific limitations and integration gates

**Present**: Newtonian reception light-time and its analytic
reception-time derivative; SR Earth-velocity aberration and its
analytic derivative using Earth barycentric acceleration; fixed
ICRF Cartesian position and speed, keeping Earth as center and
model-specific `Body` identifiers including *system barycenters*.

**Missing**: solar gravitational light deflection and
gravitational potential refinements (SOFA's `ab()` explicitly includes
an additional solar potential term), Shapiro delay, rigorous
ICRS/GCRS frame conventions, IAU 2006 bias/precession, IAU 2000A
complete nutation, **rotating-frame `Rdot*r`**, geocentric
true-ecliptic-of-date six-field output, outer planet centers where
DE441 only returns system barycenters, full TT/TDB/UT1 conversion,
and Node/SWIEPH compatibility. See
[SOFA astrometry overview](https://juliaastro.org/SOFA/stable/api/astrometry/)
and the underlying draft
[PR #70](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/70).

**Do not call these results `swe_calc_ut` or complete apparent
positions.** The first scientifically meaningful final comparison will
require an **independently verified true-ecliptic-of-date** pipeline,
JPL source/body selection and a multi-epoch Swiss 0.5.17 parity
harness including all six rates.

### Build/CI

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test aberration_state
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Read-only GitHub Actions must pass Ubuntu, Windows and macOS before
the draft can be considered technically validated; no production
migration or merge is authorized by passing CI.

## CPAL-1.0 modification record — 2026-11-11

Added independently authored `src/aberration_state.rs`,
`tests/aberration_state.rs`, its `src/lib.rs` export, this document
and README link. The existing #46 direction module and its original
tests remain untouched. No SOFA/ERFA/Swiss/NAIF source code, third-party
JPL file, proprietary harmonics/copyrighted table, new dependency
or application production change. Preserve original CPAL-1.0
Exhibit A notices, and required **Powered by VirgoStars**
(https://www.virgostars.it/) attribution.
