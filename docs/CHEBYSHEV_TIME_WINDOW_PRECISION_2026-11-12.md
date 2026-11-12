# Chebyshev position-record time-window precision

**Change date:** 2026-11-12  
**Scope:** Rust numerical foundation, `ChebyshevPositionRecord` constructor and unit tests.

## Problem and contract

A Chebyshev position record uses a finite midpoint (TDB seconds past J2000) and a positive finite radius (seconds). Previously, the constructor checked for finite values and finite nominal boundaries, but accepted a radius so small relative to the midpoint that IEEE-754 binary64 rounding made one or both interval boundaries **identical to the midpoint**.

For example, at midpoint `2^53` seconds a radius of `1` second is not distinguishable on both sides. At `1e20` seconds, a `1`-second radius is rounded away entirely. This creates a nominal interpolation window which is not reliably representable by the API's single-`f64` epoch type.

The constructor now requires both boundary values to remain distinct:

```text
start = midpoint - radius
end   = midpoint + radius

start < midpoint < end
```

A finite positive radius which collapses an endpoint returns the existing `EphemerisError::InvalidRadius` variant. An arithmetic overflow when computing a boundary continues to return `EphemerisError::ArithmeticOverflow`. No new public types or dependencies are introduced.

## Deterministic test cases

The built-in Rust unit tests cover:

- Positive and negative `2^53`-second midpoints with `1`-second radius: reject the unresolvable boundary.
- Positive and negative `1e20`-second midpoints with `1`-second radius: reject collapsed intervals.
- A resolvable `2^53`-second midpoint with `4`-second radius: evaluate positions and analytically differentiated velocities at both endpoints and the midpoint.
- An epoch beyond the same resolvable interval still returns `OutsideRecord`.

These are **synthetic numerical and error-contract regressions**, not JPL or NAIF vector accuracy claims. The record is a SPK-type-2-style mathematical abstraction, **not** an SPK binary parser. The test time magnitudes are artificial stress cases, not claimed supported physical eras.

## Verification and scientific limitations

From the Rust workspace root:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The change does not modify DE441 file parsing, planetary center selection, Earth–Moon reconstruction, TDB/TT/UTC conversions, coordinate frames, apparent corrections, units (positions in km; velocities in km per TDB second), or production VirgoStars. Single-`f64` epochs still have finite precision; this guard **does not** make such epochs suitable for arbitrary sub-second precision at extreme Julian dates. Other scientific acceptance gates remain outstanding.

## Licensing and source modification record

The two original source files changed on 2026-11-12 are `crates/ephemeris-core/src/state.rs` (constructor validation and unit tests) and `crates/ephemeris-core/src/error.rs` (diagnostic wording). Existing CPAL-1.0 Exhibit A notices and `NOTICE` remain intact; no third-party data/code is copied. The applicable attribution remains **Powered by VirgoStars** — https://www.virgostars.it/.
