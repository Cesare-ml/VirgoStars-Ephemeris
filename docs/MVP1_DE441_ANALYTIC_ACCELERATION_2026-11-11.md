# MVP-1 exact polynomial acceleration for classic DE441

**2026-11-11 | Original Rust, experimental | NO Swiss parity.**

This feature extends the *existing original Rust* classic-DE441 interpolator
with the **analytic second time derivative of its piecewise Chebyshev
polynomials**. The work is **stacked on draft PR #73** (which in turn depends
on still-draft PR #46): these PRs need scientific review and safe staged
integration before this change may be merged.

## Why this matters for planet speeds

The earlier `geocentric_light_time_annual_aberration` stage on draft #73
estimates the Earth's barycentric acceleration at reception with
**three independent Earth velocity evaluations** at `t`, `t-h`,
and `t+h`. Its accuracy depends on the caller's step `h`,
floating-point cancellation and coverage at both ends of the ephemeris
file. In rapid time-animated chart updates, choosing and checking `h`
adds needless work.

The new method
`De441::state_with_acceleration(target,center,epoch_tdb)`
computes position, velocity and acceleration **within a single
interpolation at the same TDB epoch**. Its result is
`StateVectorWithAcceleration` with independent units
**km, km/TDB day, and km/TDB day²**, source-specific `Body`
and center metadata, and fixed **ICRF-aligned** axes.

The existing `De441::state()` remains unchanged in signature, returns
the same `StateVector`, and intentionally **does not evaluate
second derivatives** unless explicitly requested. The same
`state_components` path handles the file cache, ephemeris time
range, granule lookup, planetary system barycenter and Moon/EMB/Earth
center chains for *both APIs*. Center arithmetic applies the same
coefficients to **all three derivatives**, including the direct
Earth–Moon geocentric shortcut.

## Mathematical contract

For an independently published Chebyshev representation in one
selected DE441 granule of width `w` **TDB days**,

```text
x = 2 * (t - granule_start) / w - 1
r(t) = Σ c_k T_k(x)

T_0 = 1;  T_1 = x
T_k = 2*x*T_(k-1) - T_(k-2)

T'_0 = 0; T'_1 = 1
T'_k = 2*T_(k-1) + 2*x*T'_(k-1) - T'_(k-2)

T''_0 = 0; T''_1 = 0
T''_k = 4*T'_(k-1) + 2*x*T''_(k-1) - T''_(k-2)

v(t) = Σ c_k*T'_k(x) * (2 / w)            [km/TDB day]
a(t) = Σ c_k*T''_k(x) * (2 / w)²          [km/TDB day²]
```

The extra recurrence is evaluated **only if requested**; no numerical
`t±h` is involved in the analytic `state_with_acceleration`
path. The degree-2 identity `T_2(x)=2x²-1` implies that
`d²(c_2 T_2)/dt² = 4*c_2*(2/w)²`. The tests check it against
hand-derived coefficients and independently differentiated
original-API velocities. At a granule/record boundary the returned
second derivative belongs to the selected piecewise Chebyshev
segment and may differ at the boundary due to polynomial fitting;
this method does not promise continuous physical acceleration.

This is **the mathematically exact second derivative of the
interpolating polynomial**, within floating-point limitations; it is
**not a real-kernel accuracy certificate or a guarantee of the
physical acceleration of a planet**. A real DE441 source file and
independent JPL Horizons reference vectors are still required to
establish astronomy-grade tolerances.

## Using the result in annual aberration

New `aberration_state::geocentric_light_time_annual_aberration_analytic`
composes:

1. one Earth/solar-system-barycenter
   `De441::state_with_acceleration` at observation **TDB**;
2. the **existing** `geocentric_light_time` calculation,
   differentiating retarded target emission with respect to reception
   time;
3. the **existing PR #46** special-relativistic aberration direction;
4. the **existing PR #73** analytic derivative of that direction,
   using Earth's actual interpolated velocity **and acceleration**.

The returned `AberratedGeocentricState` explicitly records
`ObserverAccelerationSource::AnalyticDe441Chebyshev` to distinguish
the new fast, no-step-parameter path from
`CenteredEphemerisVelocity { step_tdb_days }`, which remains
available for controlled regression and diagnostics.

```rust
use virgostars_ephemeris_core::aberration_state::{
    geocentric_light_time_annual_aberration_analytic,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut eph = De441::open("/externally/obtained/de441.eph")?;
let result = geocentric_light_time_annual_aberration_analytic(
    &mut eph, Body::Venus, JulianDateTdb(2_451_545.0),
)?;
println!("{:?}", result.velocity_km_per_tdb_day);
# Ok(())
# }
```

No file is bundled, downloaded or accessed implicitly. This
example is **not executed** without a separately obtained real classic
JPL binary. SPK `.bsp` and Swiss `.se1` files are not supported.

### Test evidence and limitations

`tests/de441_analytic_acceleration.rs` introduces **8 independent
offline synthetic regression tests**:

- Degree-2 Mercury, Earth–Moon barycenter and geocentric Moon
  interpolants with **hand-derived exact accelerations**, including
  mass ratio and non-collinear Y-component motion.
- Identical original `De441::state` position and velocity values
  at interior and both file/record endpoints.
- Independently centered differences of original velocities in
  multiple Chebyshev segments.
- Antisymmetric Earth↔Moon and EMB mass-weighted acceleration.
- Zero relative acceleration for identical target and center.
- One-call Earth acceleration → light-time → aberration-rate stage
  versus the earlier finite-difference stage, including explicit
  provenance of the acceleration method.
- File final-epoch support **without a future t+h sample**, unlike
  the FD method.
- Fail-closed out-of-range/nonfinite epochs and absent body series.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test de441_analytic_acceleration
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The read-only Ubuntu, Windows and macOS GitHub CI validates Rust
formatting, compilation, unit tests and lint. Synthetic data **does
not establish** whole-series physical JPL or Swiss parity.

## Remaining MVP-1 integration and safety

Results are still **fixed ICRF** Cartesian observer-target vectors
after Newtonian reception light time and observer special-relativistic
aberration, **NOT** Swiss `swe_calc_ut` apparent **true-ecliptic-of-date**
longitudes, latitudes, distances and rates. Missing major parts:
gravitational light deflection, solar potential/Shapiro delay,
rigorously aligned precession/nutation and moving-date frame rates,
the proper outer *planet centers* where classic DE441 only carries
system barycenters, UT/TT/TDB conversions, real JPL and independent
Swiss speed comparisons, and Node load/binding. The unmerged #70
six-field fixed-axis branch is **not implicitly integrated**.

Another pending draft #72 touches `de441.rs` for batch geometric
states. Coordinate any future merge carefully; do not overwrite its
work or assume this stacked branch contains it. All source code and
tests remain original; no external coefficients/software copied.

**2026-11-11 CPAL-1.0 modification record:** changed the original
`src/de441.rs` and `src/aberration_state.rs`, added original
`tests/de441_analytic_acceleration.rs`, this document and README link.
The license, Exhibit A notices, **Powered by VirgoStars**
<https://www.virgostars.it/> attribution, and existing authorized
read-only CI policy are preserved. No production, release, automated
deploy, external kernel, source data or package dependency changes.
