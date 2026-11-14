# SPK type-2-style XYZ cubic interpolation oracle

Created **2026-11-14** as an independent synthetic arithmetic regression for
the public Rust `ChebyshevPositionRecord` API in
`crates/ephemeris-core/src/state.rs`. No new runtime algorithm is introduced.

## Geometry and units

The test record is an **in-memory Chebyshev position record**, not a file
parsed from NAIF SPK, with midpoint **ET = -172800 TDB seconds past J2000**
and radius **43200 TDB seconds**. The normalized coordinate is
`u = (ET - midpoint) / radius` in [-1, 1].

Three independent cubic Cartesian polynomials (in km) are checked against
Rust's Chebyshev evaluation:

- `x(u) = 11 + 2u - 3u² + 0.5u³`
- `y(u) = -5 + 4u + 0.25u² - 2u³`
- `z(u) = 3 - u + 1.5u² + u³`

The expected velocities come from *direct analytic differentiation of these
ordinary polynomials*, divided by the record radius in **seconds**, giving
**km per TDB second**, not km/day. This independently checks the coefficient
mapping, all XYZ components, velocity scaling and signs at seven exact
normalized locations including both endpoints. A separate finite-difference
test measures slopes using positions at ±1 ET second at five interior epochs.
There are also strict out-of-coverage and malformed-coefficient/radius cases.

## Verify

```sh
cargo test --locked -p virgostars-ephemeris-core --test spk2_xyz_cubic_oracle
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

No official JPL kernels, network requests, external datasets, extra crates,
or Swiss Ephemeris source code are used. The test cannot establish NASA/JPL
real-kernel parity, NAIF SPK binary compatibility, center-chain correctness,
time-scale conversion, ICRF frame accuracy, or apparent positions.
Independent reference validation is a **separate release blocker**.

See [project validation gates](VALIDATION.md), [architecture](ADR-0001-RUST-CORE-AND-BINDINGS.md)
and the CPAL-1.0 [LICENSE](../LICENSE) / [NOTICE](../NOTICE), including
*Powered by VirgoStars* obligations.

This source-only addition has no production deployment, package publication
or release. Run the existing approved read-only GitHub Actions at the exact PR
head and record the result before treating the tests as passing.
