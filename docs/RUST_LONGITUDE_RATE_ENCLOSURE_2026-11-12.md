# Experimental Rust longitude enclosure from a declared global rate bound

**Status:** mathematical prototype; **not** real-kernel-validated continuous astronomical coverage.
**Added:** 2026-11-12 (original VirgoStars work; CPAL-1.0 notices retained).
**Branch:** `feat/rust-longitude-rate-envelope-20261112` stacked directly on `dev/ephemeris`.
**Module:** `crates/ephemeris-core/src/longitude_enclosures.rs` exported through `pub mod longitude_enclosures;`.

This is the first *Rust-side mathematical building block* for the proposed unknown-birth-time synastry uncertainty workflow in [Astrological Functions PR #11](https://github.com/Cesare-ml/astrological-functions/pull/11). It does **not** produce certified longitude bounds from a JPL kernel yet, does not implement the proposed JavaScript/WASM provider response interface and does not calculate any extra astrological method.

## Contract and units

`enclose_longitude_from_claim(LongitudeRateBoundClaim)` takes:

- `reference_longitude_deg`: canonical [0°, 360°), in **one fixed declared frame, observer and correction convention** throughout the calculation. The caller must establish and maintain those external metadata; this helper does not know whether the input is geometric, apparent, barycentric, geocentric or topocentric.
- `start_offset_days <= 0` and `end_offset_days >= 0` relative to the known or representative reference instant. The offsets and angular rate share the explicit `UniformTimeScale::Tdb` or `UniformTimeScale::Tt`; UTC is intentionally excluded.
- `max_abs_rate_deg_per_day >= 0`: **a bound asserted for the full continuous interval**, including any direct/retrograde/stationary reversals. An instantaneous velocity, endpoint-only check or discrete sweep is **not** enough.
- `reference_error_deg >= 0`: bound on error of the reference longitude in degrees.
- `asserts_full_interval_rate_bound: true`: explicit acknowledgement of responsibility for the continuous rate bound. This is **not** independent verification.

The output `ClaimedLongitudeEnclosure { start_deg, span_deg, time_scale, coverage }` describes a **single forward circular arc**. Its `coverage` is always `CallerAssertedRateBound` and never `ScientificallyValidated`.

## Mathematical construction

Let `lambda(t0)` be a reference unwrapped longitude and suppose a correctly proven bound `|d lambda / dt| <= L` is valid for every continuous instant of the closed interval `[t_start, t_end]`. For a differentiable longitude track (and its piecewise differentiable continuous extensions), the fundamental theorem of calculus and the triangle inequality give:

```text
|lambda(t) - lambda(t0)| <= L * |t - t0|
R = L * max(t0 - t_start, t_end - t0) + referenceError
```

Thus a conservative **circular** arc starts at `lambda(t0) - R` modulo 360°, with span `2R`. If `R >= 180°`, all directions may be possible and we return a full 360° arc, not a falsely precise subset. Numerical multiplication/addition/subtraction/modulo receive a small positive floating-point outward pad in the finite nonzero case. At `R == 0` the exact point is retained; on finite input arithmetic overflow the result conservatively saturates to the full circle.

**Station handling:** a continuous global absolute speed bound remains valid even when the speed changes sign or reaches zero. Consequently the mathematical enclosure formula is unchanged at stations. **But computing the actual global rate bound across all stations remains an unsolved upstream astronomical task.** The new module does not search them or certify that all were covered.

If the longitude track becomes undefined (e.g. at a polar singularity), the assumption is invalid and the future producer must fail closed rather than call this helper with a guessed finite bound.

## Example (purely synthetic)

```rust
use virgostars_ephemeris_core::longitude_enclosures::{
    enclose_longitude_from_claim, LongitudeRateBoundClaim, UniformTimeScale,
};

let arc = enclose_longitude_from_claim(LongitudeRateBoundClaim {
    reference_longitude_deg: 359.0,
    start_offset_days: -1.0,
    end_offset_days: 1.0,
    max_abs_rate_deg_per_day: 3.0, // SYNTHETIC declared full-interval bound
    reference_error_deg: 0.0,     // only meaningful when independently justified
    time_scale: UniformTimeScale::Tdb,
    asserts_full_interval_rate_bound: true,
})?;
assert!(arc.span_deg >= 6.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

This example makes no claim about the actual daily speed of any planet and performs no real DE441 file access.

## Validation

Authored ten independent Rust unit tests in the module for:

1. wrap across Aries 0°/360° and both directions;
2. a stationary reversal with a known synthetic global speed bound;
3. direct/retrograde linear tracks;
4. zero-length known-time interval;
5. asymmetric time intervals;
6. explicit reference longitude error;
7. >=180° and arithmetic-overflow full-circle saturation;
8. nonfinite/noncanonical/negative/unasserted inputs;
9. zero claimed speed across a long interval;
10. immutable, explicit conditional coverage metadata and TT scale.

Commands to run in a complete Rust checkout:

```sh
cargo fmt --all -- --check
cargo test --workspace longitude_enclosures
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**These Rust commands were not run in the assistant's current environment if Cargo/Rust are unavailable; do not infer a pass from an authored test.** An independent Python geometry sanity check with 150,000 evaluated synthetic trajectory positions in 10,000 randomized configurations is only a **cross-check of the mathematical formula**, not compilation of this Rust source, numerical error certification or a JPL/Horizons parity result.

## Scientific and interface blockers

Before exposing this through the `LongitudeEnclosureProvider` contract proposed in Astrological Functions PR #11:

1. Implement a **genuinely conservative full-interval upper bound** for apparent, Earth-centered, ecliptic-of-date longitude speed (degrees/day) in a declared uniform time scale and supported JPL kernel range. Bounds must cover complete records, frame-rate terms, corrections, time conversions, every station and retrograde loop — **not** just rates seen at finite samples.
2. Independently verify the rate bounds with analytically controlled coefficient/derivative intervals, justified numerical rounding and reference error budgets. Handle undefined directions and missing/invalid kernel coverage as errors.
3. Tie each envelope to provenance (engine version, kernel set, model), UTC→TT/TDB conversion, frame/center/corrections and true full-interval coverage evidence. This pure math helper intentionally does **not** mint those claims.
4. Build a thin Rust-backed WASM response adapter conforming to PR #11, with numerical cross-language and real JPL/Horizons tests; do not implement ephemeris arithmetic again in TypeScript.
5. Only after Cargo gates, independent scientific validation and review consider release/merge decisions; do not deploy to VirgoStars Desktop or replace its calculations.

No kernel binaries, external code, Swiss Ephemeris sources, runtime dependencies, WASM bindings, package release or production changes were added. The original VirgoStars-Ephemeris code remains licensed under **CPAL-1.0**, with the required `Powered by VirgoStars` notice and https://www.virgostars.it/ attribution as provided in the actual LICENSE and NOTICE.
