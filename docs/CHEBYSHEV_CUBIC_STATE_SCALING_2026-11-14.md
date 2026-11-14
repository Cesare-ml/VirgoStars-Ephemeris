# Three-axis Chebyshev state time-scaling regression — 2026-11-14

Modification record (CPAL-1.0 §3.3). Original source copyright (c) 2026 Cesare-ml (VirgoStars project); new deterministic integration tests are contributed under the repository's CPAL-1.0 license.

## Purpose

The stand-alone Rust `ChebyshevPositionRecord` evaluates three Chebyshev series for geometric Cartesian **position (km)** and their analytical **velocity (km/s)** at an input epoch in **TDB seconds past J2000**. This records a dedicated mathematical check that differentiates its coefficients correctly and scales the derivative by the **record radius in seconds**.

## New regression file

`crates/ephemeris-core/tests/chebyshev_cubic_state_scaling.rs` uses three distinct degree-three coefficient series, compared with independently expanded ordinary-power cubic expressions and derivatives.

- Check 7 normalized times (including ±1) across both a 25-second and 50-second record radius.
- Verify the independent polynomial position and its first derivative in each of the three Cartesian axes.
- Confirm doubling the radius keeps the position at the same normalized time unchanged but halves the km/s velocity.
- Reject epochs outside both radii with `EphemerisError::OutsideRecord`.

No external kernels, astronomy datasets, network calls, dependency changes, public API changes or published packages.

## Scientific limits and verification

Only synthetic polynomial/time-unit behavior is tested. This does **not** establish numerical DE441 parity with JPL Horizons, SPK conformance, physical corrections or real-sky precision. Tolerance for numerical assertions: absolute 2e-12 in the relevant units.

Required commands:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --test chebyshev_cubic_state_scaling
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Rust is not installed in the authoring environment: tests have not been run locally. CI on Linux, Windows and macOS must be checked before merge.
