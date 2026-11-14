# SPK fixed-axis spherical coordinates — 2026-11-14

## Purpose

A new public Rust adapter, `coordinates::project_spk_fixed_axes`, converts a **geometric SPK target-minus-center state** (`SpkRelativeState`) from fixed J2000/ICRF Cartesian axes into spherical coordinates with angular and radial rates.

Unlike the classic `project_fixed_axes` adapter for DE441, whose input velocities are km per TDB **day**, SPK velocities are km per TDB **second**. This distinction is crucial for correct angular rates and retrograde motion detection downstream. Both adapters now invoke the **same internal mathematical implementation**, so the astronomy is not duplicated.

## Public contract

```rust
use virgostars_ephemeris_core::coordinates::project_spk_fixed_axes;
use virgostars_ephemeris_core::spk_relative_state::SpkRelativeState;

// A caller-supplied state from SpkKernelSet::relative_state(...)
fn project(state: &SpkRelativeState) -> Result<f64, Box<dyn std::error::Error>> {
    let sphere = project_spk_fixed_axes(state, 0.0)?;
    Ok(sphere.longitude_rad)
}
```

- Input `SpkRelativeState` is **frame 1**, geometric J2000/ICRF; relative to the explicit NAIF center ID, with target/center/shared-ancestor identity retained.
- ET is TDB **seconds** since J2000, and is propagated unchanged into `SpkSphericalState`.
- Angle parameters and outputs are **radians**. Longitude is normalized into `[0, 2π)`, latitude into `[-π/2, π/2]`. Angle derivatives are **rad/TDB-second**, radial speed **km/TDB-second**, distance **km**.
- `tilt_rad` is an **explicit fixed passive X-axis rotation**, not a model of the ecliptic of date. Zero is original ICRF equatorial axes. Treat caller-provided obliquity as a constant illustrative tilt, not precise tropical-of-date longitude.
- Origin, polar axes (longitude undefined), nonfinite times, components, tilt and results return typed `ProjectionError` values; there are **no silent substitutes or approximations**.
- No light time, aberration, deflection, precession, nutation, frame bias, Earth orientation, UTC-to-TDB conversion, planetary topocenter, sidereal zodiac or Swiss-equivalent output is implied.

## Validation strategy

One new standalone integration test file exercises:

1. Analytical equatorial circular motion, negative longitude-rate / retrograde sign and radial derivative in km/s.
2. Cross-format mathematical parity between SPK and classic DE441 adapters, using exactly the same position/velocity expressed as km/s and km/day respectively, over multiple fixed tilts.
3. A centered finite-difference oracle for longitude, latitude and range rates in a tilted frame, with time steps measured in TDB **seconds**.
4. Singular direction, nonfinite inputs/epoch and numerical overflow rejection.

These are **synthetic mathematical/unit consistency** checks. They neither verify real SPK binary kernels nor establish agreement with independent JPL Horizons or NAIF/SPICE vectors. Real-kernel and scientific parity review remain independent prerelease gates.

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## License / modification notice

Modification dated 2026-11-14: refactored existing `coordinates.rs` projection into one shared internal numerical helper; preserved the original public `SphericalState` / `project_fixed_axes` API and its km/day output, added SPK-specific wrapper/output and a focused regression file. Original source remains under **CPAL-1.0**, with required VirgoStars attribution unchanged. No third-party datasets, dependencies, files from Swiss Ephemeris or SPICE source, public API removal, publishing or production integration.
