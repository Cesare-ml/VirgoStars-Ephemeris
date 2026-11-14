# Same-epoch SPK batch relative states — 2026-11-14

## Purpose

The `spk_batch::relative_states_at` Rust API evaluates several geometric target-minus-center states for **one shared finite TDB/ET epoch** through a caller-loaded `SpkKernelSet`. This is a thin, deterministic orchestration layer over the *existing authoritative* Rust SPK solver, not a second astronomy implementation.

For a chart with multiple bodies, callers can submit all body/center pairs once rather than managing multiple individual calls and collating potentially mismatched epochs themselves. This first implementation **does not cache state evaluations or assert a speedup**: it performs one canonical state resolution per requested pair. It also does not load, download or redistribute kernels.

## API and units

```rust
use std::fs::File;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;
use virgostars_ephemeris_core::spk_batch::relative_states_at;

fn sample(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut kernels = SpkKernelSet::new();
    kernels.load(File::open(path)?)?; // external, locally stored reviewed SPK
    let results = relative_states_at(
        &mut kernels,
        &[(301, 399), (399, 0), (301, 0)],
        0.0, // ET: TDB seconds past J2000; not UTC or a Julian day
    )?;
    assert_eq!(results.len(), 3);
    Ok(())
}
```

- Request pairs: ordered `(target_naif_id, center_naif_id)` signed 32-bit IDs; duplicate pairs remain duplicate results. The `SpkRelativeState` result preserves the target, center, common ancestor and input epoch.
- ET: **TDB seconds since J2000**, supplied explicitly as one finite `f64` to every pair. Nonfinite ET always fails, even for no requests.
- Frame: fixed NAIF frame 1, **J2000/ICRF**. Position: **km**; velocity: **km per TDB second**. Results are **geometric**, not apparent or tropical-of-date.
- Supported kernel geometry: SPK types 2/3 in frame 1 for each required center-chain link; latest loaded matching source wins. An unsupported later kernel, corrupt selected coefficients, disconnected chain or cycle fails with no substitution.
- Empty request slice with finite ET returns an empty vector; identical target/center returns exact zero per the core state API, which **does not prove body coverage** in the external kernels.

## All-or-error contract

Returns `Ok(Vec<SpkRelativeState>)` with the **same length and order** as the request list only after **every** lookup succeeds. The first failure returns either:
- `SpkBatchError::InvalidEpoch` when ET is not finite; or
- `SpkBatchError::Request { index, target_id, center_id, cause }` with the zero-based failing index, exact NAIF IDs and the original `SpkRelativeError`.

The `source()` of a failed request exposes the underlying Rust error chain for diagnostics. No partial result vector is returned, although underlying stream positions may have advanced during attempted reads; callers may retry safely through the same kernel set.

## Regression tests

Four entirely synthetic regressions use in-memory DAF/SPK file layouts with independently encoded mixed IEEE byte orders:

1. Same-ET cross-file Moon → Earth → SSB vectors, rate units and exact identity with individually resolved states; duplicates and original request order preserved.
2. A missing center-chain link at the second request reports correct index and underlying cause; subsequent valid queries remain functional.
3. A higher-priority unsupported frame masks an older supported segment even during a batch; unloading restores older behavior.
4. Invalid `NaN`/`infinite` ET is rejected, including empty batches; finite empty batches and same-body zero follow documented behavior.

Run read-only CI on the exact head:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

**Scientific limitation:** Passing synthetic tests and multi-platform CI does not establish independent NASA/JPL/NAIF SPICE numerical parity or source provenance. A real, hashed, externally obtained kernel and independently computed reference vectors are still required before production use.

## Source license and modification record

2026-11-14: added `crates/ephemeris-core/src/spk_batch.rs`, exported it from `src/lib.rs`, and documented the new public API. Original VirgoStars source remains under **CPAL-1.0** and retains the original developer's header/attribution and related `NOTICE` terms. No external kernels, third-party code, new dependencies, UI migrations, releases, protected-branch modifications or privileged workflows were added.
