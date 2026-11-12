# Aligned target–observer Chebyshev longitude-rate bounds (Rust)

**Added 2026-11-12. Status: experimental mathematical helper, NOT astronomically validated.**

This change is a draft stacked on [PR #134](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/134), itself stacked on [PR #128](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/128). It advances the future unknown-birth-time synastry pipeline by bounding the **relative** direction of a target and observer, not just an individual barycentric position.

## API and strict alignment

The new Rust module `crates/ephemeris-core/src/relative_chebyshev_longitude_bounds.rs` exports:

```rust
pub fn bound_aligned_relative_chebyshev_longitude_rate(
    target: AlignedPlanarChebyshevRecord<'_>,
    observer: AlignedPlanarChebyshevRecord<'_>,
) -> Result<RelativeChebyshevLongitudeRateBound, RelativeRateBoundError>
```

Each `AlignedPlanarChebyshevRecord` explicitly supplies:
- A distinct `target_naif_id`, plus an **identical** `common_origin_naif_id` for target and observer.
- `FixedCartesianFrame::IcrfJ2000` and `CartesianLengthUnit::Kilometer` (fixed equatorial ICRF axes only). No implicit frame conversion.
- Exactly equal `midpoint_tdb_seconds` and `radius_tdb_seconds` for target and observer, both finite/resolvable, with radius positive. The normalized record is the full `tau in [-1,1]` interval.
- Explicit finite X/Y Chebyshev coefficients, with the **same X/Y degree within each record** and possible different degrees across the two records. Missing higher coefficients across records are treated as zero, which is exact for algebraically aligned polynomials.

**This method is not applicable merely because two records cover overlapping dates**: equal normalized epoch/radius and shared frame/origin are required. No interpolation, polynomial reparameterization, record-chain composition, or Earth–Moon barycenter reconstruction is performed. It is the caller's responsibility to guarantee that the supplied X/Y coefficients are physically valid for the same reference center and full TDB interval.

## Mathematical bound

The exact mathematical relative planar polynomial is

```text
X_rel(tau) = Σ_k (X_target,k - X_observer,k) T_k(tau)
Y_rel(tau) = Σ_k (Y_target,k - Y_observer,k) T_k(tau)
```

Rather than relying on rounded coefficient subtraction to prove the bound, the method uses the triangle inequality, for each planar component `u`:

```text
|u_rel(tau)| >= |c_target,0 - c_observer,0| -
                Σ_{k>=1} (|c_target,k| + |c_observer,k|)

|d u_rel / d tau| <= Σ_{k>=1} k² (|c_target,k| + |c_observer,k|)
```

The lower component bounds yield a positive planar-distance lower bound only if they can exclude the relative origin. Otherwise, the algorithm **fails closed** with `CannotExcludeRelativePlanarOrigin` even for some trajectories whose true distance may remain positive.

Dividing derivative bounds by the record's radius in **TDB days** and then by the planar-distance lower bound produces the global bound

```text
|d atan2(Y_rel, X_rel) / d TDB_day|
    <= max_hypot(dX_rel/dTDB_day, dY_rel/dTDB_day) / min_hypot(X_rel,Y_rel)
```

This is independent of the sign of angular velocity and includes synthetic stationary/retrograde reversals within the **same mathematical polynomial record**. A safety margin is applied to floating-point coefficient sums and cancellation in the constant term, but **it is not a formally verified directed-rounding calculation**. Never infer a measured astronomical error budget or precision guarantee.

### Error/metadata contract

Invalid or unresolved conditions (empty/mismatched within-record axes, invalid/undistinguishable time interval, different common origins, duplicate target/observer ID, nonfinite coefficients, overflow, too-large degree or possible planar origin) return typed errors. The result has:
- `max_abs_rate_deg_per_tdb_day`
- `min_relative_planar_distance_km`
- `max_relative_planar_speed_km_per_tdb_day`
- both body NAIF IDs, common origin ID and exact TDB record center/radius
- `time_scale: Tdb` and `coverage: FullAlignedMathematicalRecord`.

The coverage enum **does not claim scientific or real-kernel verification**.

## Synthetic example

```rust
use virgostars_ephemeris_core::relative_chebyshev_longitude_bounds::{
    bound_aligned_relative_chebyshev_longitude_rate,
    AlignedPlanarChebyshevRecord, FixedCartesianFrame, CartesianLengthUnit,
};

let target = AlignedPlanarChebyshevRecord {
    target_naif_id: 499, common_origin_naif_id: 0,
    frame: FixedCartesianFrame::IcrfJ2000,
    length_unit: CartesianLengthUnit::Kilometer,
    midpoint_tdb_seconds: 0.0,
    radius_tdb_seconds: 86_400.0,
    x_coefficients: &[100.0, 0.0],
    y_coefficients: &[0.0, 1.0],
};
let observer = AlignedPlanarChebyshevRecord {
    target_naif_id: 399,
    x_coefficients: &[90.0, 0.0],
    y_coefficients: &[0.0, 0.0],
    ..target
};
let bound = bound_aligned_relative_chebyshev_longitude_rate(target, observer)?;
assert!(bound.max_abs_rate_deg_per_tdb_day > 5.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

All positions here are **synthetic** in kilometers; the example does not assert that a real Mars/Earth barycentric difference is adequately represented by those coefficients.

## Verification

**Authored 10 Rust unit tests**, covering aligned subtraction, unequal polynomial degrees, a stationary reversal, transfer to PR #128's circular enclosure, strict TDB interval matching, frame/origin/body identity, planar-origin rejection, invalid values, extreme cancellation/overflow and conditional metadata.

Independent synthetic geometry cross-check: **145,800** evaluated relative longitude rates over **1,800** randomized planar Chebyshev target/observer configurations (NumPy independent evaluator and derivative) remained below the same coefficient-based upper-bound formula. This is *not* a native Rust test, machine rounding proof, NASA/JPL comparison or complete scientific validation.

Required native Rust commands (not executed in the assistant runtime, where `cargo`/`rustc` are absent):

```sh
cargo fmt --all -- --check
cargo test --workspace relative_chebyshev_longitude_bounds
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Outstanding scientific integration

1. Apply this to **genuinely aligned whole-record Earth-relative position polynomials** from verified DE441/target–center chains. Most actual DE441 target and Earth series have different native record partitions; this code **rejects** those differences and requires a separately validated common-domain transformation, not a silent overlap assumption.
2. Handle real Earth vs Earth–Moon barycenter offsets, planetary barycenters vs planet centers, all missing targets and record boundaries with independent reference vectors. The current module only knows caller-supplied IDs, not actual orbital state provenance.
3. Extend the mathematical bound to the **true apparent Earth-centered ecliptic-of-date** quantity, including time-scale conversions, frame derivatives, light-time, aberration and correction error budgets. This is not an automatic relabeling of ICRF geometry.
4. Address numerically directed-rounded interval bounds or independently verified floating error estimates before claiming certified containment.
5. Prove conservative record-by-record union over a birth-time interval, including record transitions and station passages; compare against official JPL/Horizons/NAIF examples. Then consider a thin Rust/WASM provider compatible with [Astrological Functions PR #11](https://github.com/Cesare-ml/astrological-functions/pull/11).

No changes to production, GitHub Actions, license, data redistribution, privacy-sensitive data, Swiss Ephemeris, or root package releases. Original VirgoStars code remains subject to **CPAL-1.0** (preserve the original Exhibit A headers), with `Powered by VirgoStars` and https://www.virgostars.it/ per the actual LICENSE/NOTICE.