# Rust full-record Chebyshev longitude-speed bounds (experimental)

**Date:** 2026-11-12. **Branch:** `feat/rust-chebyshev-global-longitude-rate-20261112` stacked on [PR #128](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/128). This code is a mathematical source of a global fixed-frame longitude rate limit, **not** a certified NASA/JPL apparent-planet interval provider.

Module: `crates/ephemeris-core/src/chebyshev_longitude_bounds.rs`, exported in `src/lib.rs` as `chebyshev_longitude_bounds`. The new public function is `bound_full_chebyshev_planar_longitude_rate(x_coefficients, y_coefficients, radius_days, time_scale)`.

## Input assumptions

- `x_coefficients` and `y_coefficients` are Chebyshev `c0*T0 + c1*T1 + ... + cn*Tn` with **unhalved c0**, corresponding to the **same complete** normalized record domain `[-1, 1]`.
- X/Y must represent one continuous planar direction in the **same fixed Cartesian frame** and use the **same length units**, body-center/observer and corrections. This helper itself does not know what these are. It does not construct Earth-centered apparent ecliptic-of-date positions.
- `radius_days` is the half-duration of the complete record in **uniform TT or TDB days**, not civil UTC days. Zero or negative radii are rejected. Matching it to a real DE441 record's interval is the caller's responsibility.
- An actual ephemeris producer must independently establish that its time/frame/observer representation and polynomial coefficients are valid **throughout** the interval (including stationary/retrograde passages); sampled values are insufficient.
- Maximum coefficient count 4096; invalid/unequal/nonfinite values fail. No external kernels or third-party code are bundled.

## Analytical method

For a Chebyshev polynomial `u(tau) = sum(c[k] * T_k(tau))` on `tau in [-1, 1]`:

```text
|T_k(tau)| <= 1
|T'_k(tau)| <= k^2

min_abs_u >= max(0, |c0| - sum_{k>=1} |ck|)
max_abs_du_dt <= sum_{k>=1} k^2 |ck| / radius_days
```

Applying independently to X and Y yields a conservative lower radius `r_xy_min` and an upper planar speed `v_xy_max`. For `longitude = atan2(Y, X)`, when `r_xy_min > 0`:

```text
|d longitude / dt| = |X*Y' - Y*X'| / (X^2 + Y^2)
                  <= hypot(X',Y') / hypot(X,Y)
                  <= v_xy_max / r_xy_min
```

The resulting bound is expressed in **degrees per uniform day**. It applies to the entire mathematical Chebyshev polynomial and is independent of the sign of angular velocity, so stationary reversals do not invalidate it.

### Fail closed rather than infer false precision

- If coefficient bounds cannot **exclude** the planar origin (`r_xy_min <= 0`), return `CannotExcludePlanarOrigin`. This may reject safe tracks because component-wise intervals are deliberately conservative.
- If finite operations overflow (including coefficient sums, derivative sums or calculated rate), return `UnboundedArithmetic` — no finite result is fabricated.
- Coefficient-sum and rate calculations carry explicit small floating-point safety inflation/deflation. **This is not a formally proven directed-rounding interval arithmetic implementation or a real ephemeris error budget.** It must be reviewed before claiming machine-certified containment.

The result includes `CoefficientBoundCoverage::FullMathematicalPolynomialRecord`, **not** proof of correctness of external ephemeris coefficients, observer composition, or astronomical frame transformations.

## Synthetic Rust usage

```rust
use virgostars_ephemeris_core::chebyshev_longitude_bounds::{
    bound_full_chebyshev_planar_longitude_rate,
};
use virgostars_ephemeris_core::longitude_enclosures::UniformTimeScale;

let rate = bound_full_chebyshev_planar_longitude_rate(
    &[10.0, 0.0], // x(tau) = 10
    &[0.0, 1.0],  // y(tau) = tau, -1 <= tau <= 1
    1.0,         // one TDB day per normalized unit
    UniformTimeScale::Tdb,
)?;
assert!(rate.max_abs_rate_deg_per_day > 5.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The mathematical output can feed the previous [PR #128](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/128) function `enclose_longitude_from_claim` for that *same* complete mathematical record. The PR #128 interface still marks the resulting circular arc as `CallerAssertedRateBound`, never scientifically certified.

## Tests and review gates

Eight Rust synthetic unit tests are authored for:
- constant zero-rate nonzero position;
- a linear sweep and every full-record endpoint;
- a `T2`-polynomial station and retrograde reversal;
- refusal when planar origin cannot be excluded;
- malformed input, coefficient mismatch and invalid time radius;
- nonrepresentable derivative sums and excessive degree;
- inverse scaling of rate with radius in days;
- handoff from the analytically obtained rate to PR #128's circular-arc function.

Native checks required on an authenticated Rust checkout:

```sh
cargo fmt --all -- --check
cargo test --workspace chebyshev_longitude_bounds
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

If these tests are **not executed**, do not report them as passing. A separate numerical check in Python can help verify the algebra but is **not** Rust compilation or authoritative reference validation.

## Critical next steps before real-world use

1. Derive a conservative bound for the **actual Earth-relative target position** (including center-chain arithmetic and any record handovers), not simply a single target barycenter record. The observer reference must be fixed and explicit.
2. Add exact frame-rate terms and correction models to derive apparent geocentric ecliptic-of-date longitude and its valid time derivative. An ICRF fixed-frame bound **cannot** be silently labeled an apparent-of-date bound.
3. Implement a documented strategy for piecewise intervals across multiple Chebyshev records and discontinuities, using conservative union, proven numeric error budgets and station coverage.
4. Obtain separately licensed DE441 datasets legally, compare trajectories and conservative envelopes to independent NASA/JPL/NAIF references and document tolerances, provenance, failure modes, and time-scale conversions.
5. Only after these steps expose a thin Rust/WASM provider satisfying the [Astrological Functions PR #11](https://github.com/Cesare-ml/astrological-functions/pull/11) contract, with its separate warning that provider-asserted coverage is not independently certified.

This PR adds no file download, new dependency, Swiss Ephemeris source, Python/TypeScript astronomy implementation, GitHub Actions workflow, release, or production integration. License: **CPAL-1.0**, original VirgoStars source notices retained. Follow `Powered by VirgoStars` and https://www.virgostars.it/ requirements in the official LICENSE/NOTICE.
