# 3D reception light-time kinematic regression

**Modification date:** 2026-11-14  
**Initial developer:** Cesare-ml (VirgoStars project). Original source is CPAL-1.0 (see [LICENSE](../LICENSE), [NOTICE](../NOTICE) and [LICENSING_AND_CREDITS.md](LICENSING_AND_CREDITS.md)).

## Scope

Two deterministic, network-free unit regressions were added to the existing Rust `light_time::tests` module. The implementation and public interface of `geocentric_light_time` / `solve` have **not** changed.

- **Moving observer and target in XYZ:** the test fixture supplies independent linear barycentric velocities with nonzero components on all axes (km/second, converted to the core's km/TDB-day). It evaluates the solver at TDB reception epochs `J2000 - 600 s`, `J2000`, and `J2000 + 600 s`. A centered difference of the returned retarded *position* independently estimates its derivative with respect to reception time. The test compares this with the returned corrected velocity on all three axes. A centered difference of one-way delay (seconds) independently estimates the returned dimensionless light-time rate.
- **Stationary non-axis-aligned geometry:** an explicitly constructed 3:4:12 3D vector has known Euclidean length, equal to the exact SI speed of light multiplied by 260 seconds. A nonzero positive delay and *exact* zero kinematic rates are required for both stationary barycentric bodies.

## Numerical contracts and scientific limits

- Inputs: TDB Julian days at reception and emission, **geometric** ICRF-aligned barycentric Cartesian state vectors, position in km, input velocity in km/TDB day.
- Outputs: retarded Earth-centered position in km, reception-time derivative in km/TDB day, one-way delay in SI seconds, delay rate in seconds per second.
- Finite-difference span: ±600 s around JD 2451545.0. The denominator is the **actual represented interval in TDB seconds**, rather than assuming the requested interval survived single-`f64` JD rounding exactly.
- Maximum absolute residual: `2e-5 km/s` for each derivative component and `1e-8 s/s` for delay rate. These generous synthetic regression tolerances accommodate JD rounding and centered-difference truncation; they are **not scientific accuracy claims**.
- Stationary delay tolerance: `1e-10 s`; Cartesian velocities and delay rate must equal zero exactly.
- The 3D paths are synthetic. These checks do **not** validate authentic JPL/NAIF kernels or Horizons positions; they do not imply aberration, gravitational deflection, rotating date-frame rates, complete apparent positions or Swiss Ephemeris equivalence.

## Reproduction

On the feature branch, with Rust installed:

```bash
cargo fmt --all -- --check
cargo test --workspace light_time::tests::three_axis_light_time_velocity_and_delay_rate_match_centered_differences
cargo test --workspace light_time::tests::stationary_three_axis_light_time_preserves_exact_zero_rates
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Use the GitHub Actions run for the **exact PR head** as evidence of successful checks; do not assume these commands passed until reported. Local JPL files, credentials, downloads, extra dependencies and production changes are not required.
