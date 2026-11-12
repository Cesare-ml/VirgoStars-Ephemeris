# Overlapping Chebyshev longitude-rate bound without time alignment

**Added 2026-11-12 · Experimental mathematical helper · CPAL-1.0**

This change extends the [aligned-record bound in PR #135](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/135), stacked on #134 and #128. It addresses **two independently timed Chebyshev records whose time domains overlap**, but whose polynomial midpoint, radius and degree may differ.

The original Rust module is `crates/ephemeris-core/src/overlapping_chebyshev_longitude_bounds.rs`. It deliberately **does not** support a multi-record continuous track, simulate astronomy, or claim an independently verified DE441 bound.

## API

```rust
pub fn bound_overlapping_relative_chebyshev_longitude_rate(
    target: IndependentPlanarChebyshevRecord<'_>,
    observer: IndependentPlanarChebyshevRecord<'_>,
    start_tdb_seconds: f64,
    end_tdb_seconds: f64,
) -> Result<OverlappingRelativeLongitudeRateBound, OverlappingRecordError>
```

Each record must declare a distinct `target_naif_id` and the same `common_origin_naif_id`, `FixedCartesianFrame::IcrfJ2000`, and `CartesianLengthUnit::Kilometer`. Its X/Y Chebyshev polynomial coefficients share one midpoint and radius; each record's X/Y degree must agree, but target and observer may have different polynomial degrees. The normalized parameter is **independently** `tau=(TDB_seconds - midpoint_tdb_seconds)/radius_tdb_seconds` per record. Coefficient convention is `sum c_k T_k(tau)`, with unhalved `c_0`.

The requested `[start_tdb_seconds,end_tdb_seconds]` must be a strictly positive, finite interval **wholly inside the intersection** of the two complete record domains. Dates are **uniform TDB seconds**, not UTC, TT, or JD. The calculation neither pads gaps nor extrapolates.

The output contains the claimed full subinterval, body IDs, common origin, conservative lower planar distance, upper planar relative speed, upper longitude rate in degrees per TDB day, and the status `RequestedOverlapOfTwoPolynomials`. It is strictly mathematical coverage, **not** a scientifically verified planetary ephemeris guarantee.

## Derivation, including different midpoint/radius

For the same TDB instant `t`, let `tau_t=(t-m_t)/r_t` for the target and `tau_o=(t-m_o)/r_o` for the observer. These parameters **are not required to match**. Both remain within [-1,1] on the requested intersection.

For each component `u` in X/Y, use the independently valid inequalities `|T_k| <= 1` and `|T'_k| <= k²`:

```text
|u_target(t) - u_observer(t)| >=
    |target_c0 - observer_c0| - sum_{k>=1}|target_ck| - sum_{k>=1}|observer_ck|

|d(u_target-u_observer)/dt| <=
    sum_{k>=1} k²|target_ck| / target_radius_seconds +
    sum_{k>=1} k²|observer_ck| / observer_radius_seconds
```

The independent X/Y component bounds imply a lower bound `r_min` on relative planar distance and an upper bound `v_max` on relative planar speed. Provided `r_min > 0`:

```text
|d atan2(Y_relative, X_relative)/d TDB_day| <=
    v_max_TDB_day / r_min  [radians per day]
```

Converting to degrees yields the output rate. This bound applies to every instant of the mathematical **common subinterval**, including stationary or retrograde reversals, regardless of the two different normalized record parameters.

**No coefficient interpolation or polynomial alignment occurs.** Small floating-point safety guards are included, but these are **not a proof of correctly outward-rounded interval arithmetic**. A true astronomical enclosure additionally needs rigorous numerical and physical model error estimates.

## Failure rules

All of the following are explicit typed errors: empty/invalid polynomial arrays, different X/Y degrees within one record, more than 4096 coefficients, NaN/Infinity, unusable epoch/radius, duplicate body IDs, incompatible common origin/frame/length units, zero-length or inverted requested interval, any uncovered requested instant, inability to exclude relative planar zero, or arithmetic overflow. The module **never falls back** to finite sampled speeds.

### Critical boundary between this PR and future multi-record stitching

This function supports **one target record and one observer record**, regardless of their different domains. It does **not** claim continuity across transitions from one record to another.

To combine a long birth-time interval spanning multiple records, a future component must prove complete temporal coverage **and** appropriately handle endpoint position continuity and numerical mismatch. Taking a maximum of per-record speed bounds alone does not bound a possible discontinuous jump at a record boundary. Until boundary behavior is accounted for, do not use a stitched rate as an unconditional continuous-time bound.

Additional limitations: real DE441 series can have different center chains (Earth vs Earth–Moon barycenter), and the current fixed ICRF geometric bound is **not** apparent geocentric ecliptic-of-date longitude. Light-time, annual aberration, varying frame, UT1/TT/TDB conversions, kernel error and station handling at transitions remain separate review gates. Never treat the result as certified by NASA/JPL or as sufficient for the [Astrological Functions provider in PR #11](https://github.com/Cesare-ml/astrological-functions/pull/11).

## Verification

**12 authored Rust unit tests** cover:
- different time midpoints/radii, exact endpoints and relative motion;
- observer changes on its own time normalization;
- synthetic stationary/retrograde reversal within the common interval;
- different Chebyshev degrees without resampling;
- disjoint domains and extrapolation refusal;
- invalid, inverted and nonfinite requested intervals;
- incompatible common origin and duplicate body identity;
- malformed coefficients/record epochs;
- planar singularity and extreme constant-term overflow;
- compatibility with the previous conditional arc function;
- honest mathematical-only metadata.

An independent **Python 3 + NumPy** check sampled 80 times each in 1,800 randomized synthetic target/observer polynomial pairs, with independent domain midpoints and radii: **144,000 instantaneous rates** stayed within the coefficient-derived bound, no violations. It verifies **sampled mathematics only**, not native Rust execution, formal rounding, or astronomical accuracy.

Execute these in a complete authenticated Rust checkout:

```sh
cargo fmt --all -- --check
cargo test --workspace overlapping_chebyshev_longitude_bounds
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The assistant environment did not have `cargo` or `rustc`, therefore the authored Rust tests and compiler gates were **not executed** here.

No new dependencies, kernels, private application code, Swiss Ephemeris source, WASM bindings, GitHub Actions changes or production migration. Preserve the original CPAL-1.0 source headers/NOTICE and the applicable `Powered by VirgoStars` / https://www.virgostars.it/ attribution.

**Next milestone:** build independently validated piecewise record selection with gap rejection and explicit endpoint-continuity/error handling; later derive and independently compare complete apparent Earth-centered ecliptic-of-date interval bounds using official JPL/NAIF references.