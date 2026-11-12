# Piecewise relative-longitude enclosures with explicit boundary jumps (Rust)

**2026-11-12 · Experimental mathematical implementation · NOT JPL/Horizons validated**

This draft is stacked on [PR #140](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/140), itself stacked on #135, #134 and #128. It adds the first **multi-segment** composition for target–observer Chebyshev angular-motion bounds in a fixed ICRF/J2000 Cartesian frame. The result **never claims continuous physical motion across records**, and never silently assumes that adjacent independently evaluated polynomials have identical endpoints.

## API

```rust
use virgostars_ephemeris_core::piecewise_chebyshev_enclosures::{
    enclose_piecewise_relative_chebyshev_longitude,
    PiecewiseRelativeChebyshevSegment,
};

pub fn enclose_piecewise_relative_chebyshev_longitude(
    pieces: &[PiecewiseRelativeChebyshevSegment<'_>],
    reference_error_deg: f64,
) -> Result<PiecewiseLongitudeEnclosure, PiecewiseEnclosureError>
```

Each piece contains two `IndependentPlanarChebyshevRecord` values accepted by the [PR #140 overlap-rate helper](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/140), an explicit `start_tdb_seconds/end_tdb_seconds` interval and a `max_boundary_jump_deg` value.

- **Piece 0:** `max_boundary_jump_deg: None`.
- **Every subsequent piece:** `max_boundary_jump_deg: Some(limit)`, with a finite caller-asserted angular jump upper bound in [0°,180°] for the boundary from the preceding piece. A zero limit means the caller asserts **zero jump**; it does not mean the library independently proved continuity.
- All pieces are required in **strict chronological order** with `end_i == start_(i+1)` as exact TDB-second floating-point values. No gaps, overlaps, resampling, implicit gap padding, rounding of coverage endpoints, or date conversion are permitted.
- Every piece must be mathematically covered by **both its own** target and observer Chebyshev records. No extrapolation beyond either normalized [-1,1] domain.
- Every piece must retain the same target and observer body IDs, common physical origin ID, fixed ICRF axes and kilometer units. One consistent angular-frame convention is required for the entire sequence. It is the caller's responsibility to ensure these identifiers match real JPL data; they are not proof of dataset provenance.
- Maximum 256 segments, finite nonnegative `reference_error_deg`, typed errors on invalid identity/coverage/singularity/bounds, no fallback to instantaneous-sampled speeds.

## Why a maximum piecewise speed is NOT sufficient

Let `L_i` be the previous PR #140 **global mathematical angular-speed bound for piece i**, and `dt_i` its duration in uniform TDB days. Let `J_i` be the caller-asserted **upper bound** (degrees) of the angular direction change when switching from record `i-1` to record `i`. Then:

```text
variation <= Σ_i (L_i * dt_i) + Σ_{i=1..n-1} J_i

reference radius = variation + reference_error_deg
```

This bound is a **path-variation budget**, *not* a continuous derivative bound: if any `J_i > 0`, the mathematical piecewise path can jump at a boundary, with an undefined ordinary derivative there. Simply using `max_i L_i` as an all-day speed without accounting for jumps would be unsound.

The implementation:
1. Calls the existing PR #140 full-record overlap-rate bound for each exact piece.
2. Checks the strict TDB coverage and metadata continuity.
3. Evaluates the **left and right synthetic relative longitudes** at each record transition using the existing Rust Chebyshev evaluator. If the *observed* finite-precision angular discrepancy exceeds the caller's declared upper bound, rejects the request; this check catches contradictions but **does not certify the claimed bound**.
4. Adds the **declared limit** `J_i` (not merely the observed value) to the variation.
5. Computes the reference longitude at the start of the first piece, then returns a symmetric circular arc with the derived total variation radius and an explicitly documented small floating-point padding. If the finite radius reaches 180° or above, returns a full-circle `{start_deg:0,span_deg:360}`. Nonfinite arithmetic and invalid records fail closed.

The return value includes the full requested time window, both body IDs and common origin, `total_declared_variation_deg`, the resulting arc, and frozen-by-value segment summaries (new Rust value vector) with per-piece rate, travel budget, declared boundary jump and diagnostically evaluated boundary jump.

**Crucially, `coverage: CallerAssertedBoundaryJumpsUnverified` is mandatory.** It is not `VerifiedContinuousEphemeris`, nor an assertion that `L_i` and `J_i` are correctly bounded for a real NASA/JPL ephemeris dataset. The diagnostic boundary evaluation does not establish rounding error limits or physical endpoint continuity.

## Synthetic behavior

Two constant segments with one first relative direction at 0° and another at approximately 5.71° have zero derivative within each segment. Even so, their **nonzero junction jump** must be budgeted; with `Some(6.0)`, the final arc expands by 6°. With `Some(1.0)`, the module rejects the claim rather than fabricating a continuous speed. If the segments have mismatched TDB endpoints, it rejects the data even when each Chebyshev pair is internally valid.

This output is only a mathematical **conditional enclosure**. Do not feed an equivalent `L=total_variation/duration` back into PR #128's smooth-rate API as if it represented a derivative valid at every instant.

## Validation evidence

**12 authored Rust unit tests** cover contiguous two-/three-piece composition, nonzero and understated jump limits, gaps and overlaps, a full-circle saturation case, first/missing boundary contracts, body-reference consistency, NaN/Infinity/negative inputs, singularity rejection, and one-piece zero-variation behavior. These tests are authored on GitHub but **not executed in the assistant's environment** if `cargo` and `rustc` remain unavailable.

An independent Python 3 / NumPy synthetic check generated **1,200 three-piece target/observer scenarios** with independent Chebyshev midpoint/radius per piece; **219,600 sampled positions** were all inside the conservatively constructed variation arcs (850 scenarios saturated at the full circle). This is **only a sampled mathematical cross-check**, not Rust compilation, a formal bound on machine rounding, independent JPL/NAIF validation, or proof of real-world continuity.

Required native commands for an authenticated checkout:

```sh
cargo fmt --all -- --check
cargo test --workspace piecewise_chebyshev_enclosures
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Run the complete stack's Cargo gates before any integration. Synthetic tests must not be described as independent physical ephemeris accuracy evidence.

## Scientific gates before actual VirgoStars integration

- **Record selection/coverage:** consume official DE441 ephemeris data under its separate terms; resolve real target/observer origins and Earth–Moon barycenter vs Earth center, planetary barycenters vs centers, record priorities and exact TDB coverage.
- **Boundary error budgets:** independently justify the caller-provided `J_i` from actual kernel-record endpoint discrepancy and numerical uncertainty; use formally controlled arithmetic rather than trusting observed floating-point values. A provider should reject unsupported continuity claims and must never silently assert `Some(0)`.
- **Frame and physical corrections:** this code only handles fixed ICRF geometric XY. The future consumer [Astrological Functions PR #11](https://github.com/Cesare-ml/astrological-functions/pull/11) requires **Earth-centered apparent ecliptic-of-date** longitudes. Implement verified TDB/TT/UTC conversions, Earth observer, light-time, aberration, frame derivatives and their interval bounds in the authoritative Rust core.
- **Independent reference validation:** compare actual states/rates and interval enclosures with official NASA/JPL/NAIF/Horizons numerical fixtures, across record transitions, retrograde stations, and realistic epochs; record full source/provenance, exact correction settings and tolerances.
- **Thin cross-language interface:** only then expose the finalized Rust-derived enclosure through a WASM/TypeScript provider without duplicating astronomy in JavaScript.

This PR adds no third-party source, data files, kernels, new dependencies, CI workflow, production deployment, package release, license change or `main` merge. The original VirgoStars code remains **CPAL-1.0**, with full Exhibit A notices and the `Powered by VirgoStars` attribution per LICENSE and NOTICE.
