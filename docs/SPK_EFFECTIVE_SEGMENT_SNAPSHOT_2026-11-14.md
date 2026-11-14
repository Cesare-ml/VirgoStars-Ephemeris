# Effective SPK segment snapshot at one TDB epoch

**Experimental Rust metadata API · 2026-11-14 · Synthetic tests only**

## Goal

The new Rust function `spk_effective_index::effective_segments_at(&SpkKernelSet<R>, et)` returns a deterministic inventory of **which DAF/SPK segment actually wins** for every declared NAIF target covered at one epoch. This is useful for investigating overlapping SPK kernels, high-priority overlays and coverage gaps before requesting geometric states.

It is a read-only composition of the existing `SpkKernelSet::declared_target_ids` and `SpkKernelSet::selected_segment` methods. It does not implement a second priority algorithm, read coefficient records, calculate coordinates, load additional kernels or download data.

## Public contract

```rust
use std::fs::File;
use virgostars_ephemeris_core::spk_effective_index::effective_segments_at;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

fn example(paths: &[&str], et: f64) -> Result<(), Box<dyn std::error::Error>> {
    let mut kernels = SpkKernelSet::new();
    for path in paths {
        kernels.load(File::open(path)?)?;
    }
    for selection in effective_segments_at(&kernels, et)? {
        println!(
            "target {} kernel {} center {} frame {} type {}",
            selection.target_id,
            selection.kernel_load_index,
            selection.descriptor.center_id,
            selection.descriptor.frame_id,
            selection.descriptor.spk_type,
        );
    }
    Ok(())
}
```

- **ET:** one finite `f64` TDB second offset since J2000 (not UTC, a JD or TT). NaN/infinity are rejected with `SpkStateError::InvalidEpoch`, **even on an empty kernel set**.
- **Output:** zero or one `SpkEffectiveSegment` per covered target, sorted by signed NAIF ID ascending. Each item carries the exact selected descriptor and zero-based kernel load index (0 is the oldest).
- **No coverage:** a declared target without coverage at ET is omitted. To inspect gaps, compare returned target IDs against the set's `declared_target_ids()`.
- **NAIF precedence:** the last loaded file with a matching covering descriptor wins; within it, the last matching segment wins. Inclusive segment start/end boundaries follow the existing selector.
- **Unsupported/corrupt sources:** unsupported frame IDs, SPK types, and unreadable coefficients are *not* filtered or silently replaced with older kernels. The snapshot is only metadata; a returned descriptor is not proof that the SPK state can be evaluated.
- **Frame and units:** segment coverage timestamps are **TDB seconds past J2000**, NAIF IDs are signed integers and DAF addresses are 1-based 8-byte words. No positions, velocities, angular coordinates, frames-of-date, topocentric or apparent corrections are produced.
- **Cost and semantics:** one traversal of declared target inventory plus one ordinary metadata selection per target. No new file I/O after caller loading and no promise of speedup over individual lookups. A later load/unload changes the snapshot as expected.

## Synthetic regressions

Four tests construct entirely original in-memory DAF/SPK metadata in both IEEE byte orders. They verify:

1. Deterministically sorted target results and exact parity with individually selected source descriptors, including **a higher-priority unsupported frame/type** that must remain visible.
2. Negative ET, gaps and inclusive coverage boundaries; a newer overlay at ET=2 and ET=8, with priority returning to an older source at ET=10.
3. Unloading the newest kernel restores the older selection without mutating descriptor order.
4. Empty-set results and explicit nonfinite ET errors.

These are **structural/selection tests**, not independent numerical validation of NASA/JPL SPK files. Authentic kernel acquisition/provenance, separate NAIF/SPICE/Horizons reference vectors and reviewed numerical tolerances remain required for scientific accuracy claims and production integration.

## Verification and attribution

The pull request must pass the repository's existing read-only Rust CI on its exact head:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

Modification dated **2026-11-14**: added `crates/ephemeris-core/src/spk_effective_index.rs`, exported it from `lib.rs`, and documented the API. Original source remains **CPAL-1.0**, with required VirgoStars attribution and existing `LICENSE`/`NOTICE` unchanged. No third-party code or kernel binaries were copied; no new dependencies, privileged CI actions, deployments or releases were introduced.
