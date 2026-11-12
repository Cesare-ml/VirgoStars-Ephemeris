# Civil UTC-offset → approximate TDB → DE441 → six geocentric fields

**2026-11-12 | Experimental MVP-1 integration adapter.**

## Purpose

This deliberately small **new** Rust adapter provides one operation from a civil timestamp to six **geocentric, light-time-only, fixed-axis** spherical position/rate fields using the existing core; it **does not implement a new astronomical algorithm**:

```text
CivilTimestamp (proleptic Gregorian, explicit UTC offset)
  → CivilTimestamp::to_approximate_tdb_julian_date() [PR #96]
  → geocentric_light_time() [existing classic-DE441 core]
  → project_fixed_axes() [existing six-field position/rate math]
  → CivilGeocentricFixedSpherical
```

The API `civil_geocentric::geocentric_fixed_spherical_from_civil(&mut De441<R>, Body, CivilTimestamp, fixed_tilt_rad)` accepts a caller-owned **classic binary JPL DE441** reader and never downloads data or accesses the network. Use `fixed_tilt_rad=0.0` to keep the output in the ICRF equatorial axes. Any other tilt is a *constant, caller-supplied* passive X-axis rotation, **not** an official IAU moving ecliptic-of-date transform.

This branch stacks on **draft PR #96** (the bounded, explicitly approximate civil→TDB adapter). It does not touch or import draft [#70](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/70) (DE441 light-time six fields in degrees/AU), [#94](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/94) (six-field rate regression), or [#77](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/77) (annual-aberration six-field pipeline). **This is a civil-input integration boundary in raw rad/km units**, not a parallel ephemeris engine or a claim those pending PRs have merged. Once #70 and #77 pass scientific review, the wrappers can share the same underlying light-time stage and common unit formatter, avoiding any duplicate scientific logic.

## Rust usage

```rust
use virgostars_ephemeris_core::civil_geocentric::geocentric_fixed_spherical_from_civil;
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::first_sky::CivilTimestamp;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut eph = De441::open("/user-provided/classic-de441.eph")?;
let civil = CivilTimestamp::parse("2026-11-10T13:30:00+01:00")?;
let result = geocentric_fixed_spherical_from_civil(
    &mut eph,
    Body::Mercury,
    civil,
    0.0, // fixed ICRF equatorial axes
)?;
println!("Longitude (radians): {}", result.spherical.longitude_rad);
println!(
    "Longitude rate (rad/reception TDB day): {}",
    result.spherical.longitude_rate_rad_per_day
);
# Ok(())
# }
```

## Output and provenance contract

`CivilGeocentricFixedSpherical.spherical` is the existing `coordinates::SphericalState`:
- `target`, `center=Body::Earth`, `epoch` = **reception approximate TDB Julian date**.
- `longitude_rad` in [0, 2π), `latitude_rad` in [−π/2, π/2], `distance_km` positive.
- `longitude_rate_rad_per_day`, `latitude_rate_rad_per_day`, and `distance_rate_km_per_day` are **signed** first derivatives with respect to **reception TDB day**, including the existing one-way light-time derivative; retrograde motion is not rectified.
- `tilt_rad` identifies the supplied **constant** X-axis rotation. For tilted axes, both position and rate are rotated by the same matrix; **there is no moving-frame dR/dt term**.
- The outer planet variants may designate **planet-system barycenters**, not physical centers.

The wrapper also returns the target `emission_epoch` (TDB JD), `light_time_seconds`, and `light_time_rate` (s/s). The civil input and model come from #96: proleptic Gregorian clock with explicit `Z` or numeric offset, UTC coverage **1972-01-01 through 2027-06-27**, limited IERS Bulletin C 72 leap-second schedule; leap-second *labels* (`:60`) are not accepted, TDB−TT is approximated by the documented truncated USNO series, and a single-`f64` JD cannot retain arbitrary sub-millisecond detail.

Three explicit typed error layers are preserved:
- `CivilGeocentricError::CivilTime(SkyError)` for invalid / out-of-supported-range civil timestamps;
- `CivilGeocentricError::LightTime(LightTimeError)` for missing target/center series, undefined Earth target, unavailable reception/emission coverage, or invalid numeric geometry;
- `CivilGeocentricError::Projection(ProjectionError)` for invalid constant tilt or undefined spherical coordinates.

## What is *not* included

This is **geocentric**, not WGS84 topocentric; **one-way light-time**, not annual aberration or relativistic deflection; **fixed ICRF**, not IAU true ecliptic of date; **TDB** rates, not UT1 or sidereal derivatives. It is **not** a replacement for `swe_calc` / `swe_calc_ut`. It performs no precession, nutation, Earth-orientation transform, moving-date-frame derivative, solar deflection, gravitational delay, atmospheric refraction, physical outer-planet center reconstruction, or cross-language Node/Python binding.

This stage can be reused by future, separately reviewed consumer APIs. Until independently validated with an actual official JPL kernel and matching Horizons/Swiss reference configurations, the output is **experimental**, not certified astronomical or astrological precision.

## Synthetic integration validation

`crates/ephemeris-core/tests/civil_geocentric_pipeline.rs` builds a deterministic **9 KB fictional classic-DE441** record entirely in memory. Its independent invariants cover:

1. Equal UTC instants expressed as `+01:00` and `Z` yield consistent reception epochs, positions and speeds; a linearly departing target matches the **closed-form retarded range and radial speed** `r=(r0+vt)/(1+v/c_day)`, `dr/dt=v/(1+v/c_day)`.
2. A non-collinear moving target has latitude, longitude and radial derivatives consistent with **numerically centered differences of complete civil-input pipeline outputs** at neighboring timestamps; longitude differences are wrapped as signed angular separations.
3. Negative transverse motion retains **negative retrograde** angular speed. Explicit fixed-axis tilt preserves distance and radial speed.
4. Invalid UTC coverage, Earth target, missing body series, nonfinite tilt and insufficient retarded *emission* coverage produce distinct typed errors; no silent fallback.

Commands with a Rust toolchain installed:

```bash
cargo fmt --all -- --check
cargo test --workspace --test civil_geocentric_pipeline
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

No external DE441 kernel, Swiss source, Horizons query, or downloadable dataset is required for these synthetic regressions. **Passing them does not establish parity against real JPL/Horizons or Swiss.**

## Licensing and modification record

2026-11-12: new CPAL-1.0 source file `src/civil_geocentric.rs`, new synthetic test file `tests/civil_geocentric_pipeline.rs`, one module export in `src/lib.rs`, and this documentation file. Existing license/NOTICE/Exhibit A source notices remain intact. No third-party code, scientific coefficients or planetary data are redistributed. Required attribution for applicable Larger Works remains **Powered by VirgoStars** — https://www.virgostars.it/.

**Integration order:** Review and integrate #96 first, then this **stacked** change; no merge to `main` or `dev/ephemeris` without explicit owner decision and scientific review, and no release/deployment.
