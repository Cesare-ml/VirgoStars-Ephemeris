# Independent interior Chebyshev interpolation regressions — 2026-11-14

Modification record (CPAL-1.0 §3.3): original VirgoStars-Ephemeris code is licensed under CPAL-1.0 by the Initial Developer Cesare-ml (VirgoStars project).

## Scope

The new Rust integration test `crates/ephemeris-core/tests/chebyshev_trigonometric_reference.rs` checks the public `evaluate_chebyshev` numerical primitive against the mathematical identities

- `T_n(cos θ) = cos(n θ)`
- `dT_n/dx = n sin(n θ) / sin θ` for `x = cos θ` and `-1 < x < 1`.

This is an independently expressed **trigonometric** oracle; the implementation under test uses the extended Clenshaw recurrence. The fixtures check degrees 0, 1, 2, 3, 4, 8, 16, 32 and 64 across seven interior normalized times and a mixed 25-coefficient series at five interior points. Derivative units are **per normalized Chebyshev coordinate**, not per second or per day. Tolerances are 2e-12 for values and 2e-9/5e-10 for derivatives (absolute, dimensioned by the supplied coefficients).

## Boundaries and provenance

Only deterministic mathematical identities are involved; there is no external reference source code, ephemeris kernel, redistribution, or runtime dependency. These checks are **not** independent scientific validation of DE441 vs JPL/Horizons, SPK parsing, time scales or sky coordinates. The existing explicit endpoint regression remains separate because the trigonometric derivative identity divides by `sin θ` at endpoints.

## Verification

Run on an environment with a Rust toolchain:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --test chebyshev_trigonometric_reference
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The PR CI covers Linux, Windows and macOS. This authoring environment lacks `cargo` and `rustc`, so no local Rust test execution is claimed; check the PR's CI result before merging.
