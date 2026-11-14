# Atomic multi-kernel SPK load (experimental Rust API)

**Added 2026-11-14; additive public API, no production integration.**

## Motivation

The existing `SpkKernelSet::load(reader)` validates and appends **one** input. A caller that loads several SPK files in a loop can end up with a partially updated kernel stack if the second or later index is malformed. That can silently change effective segment precedence during an application's error handling.

`SpkKernelSet::load_batch_atomic(readers)` stages the **entire supplied ordered batch** before publishing any new entries:

- Each seekable `Read + Seek` input is inspected using the **existing** canonical `inspect_spk` index parser.
- On full success, append all inputs in their original order; the last loaded file still has highest segment priority.
- If an input's DAF/SPK **index inspection fails**, return `SpkKernelLoadError { input_index, error }`. The index is **zero-based within the attempted batch**, and the original `SpkIndexError` is retained as the error source.
- On failure, do not append any batch member. The **pre-existing loaded stack and its cursors remain unchanged**. Rejected batch readers are consumed/dropped; this API does not return them for retry.
- An empty batch succeeds without changes. No new dependencies, network access, downloads or automatic fallback.

Example (illustrative; caller must supply legally obtained SPK files):

```rust
use std::fs::File;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut kernels = SpkKernelSet::new();
let inputs = vec![
    File::open("/local/first.bsp")?,
    File::open("/local/second.bsp")?,
];
kernels.load_batch_atomic(inputs)?;
assert_eq!(kernels.kernel_count(), 2);
# Ok(())
# }
```

## Scope and limitations

Atomicity applies **only to the in-memory kernel stack update during index loading**. It is not a filesystem transaction, a lock or an immutable snapshot. The SPK coefficient records are read on demand, so successfully indexed kernels can still fail at later state evaluation (unsupported frame/SPK type, corrupt record, changed file, I/O error). The implementation does not validate numerical ephemeris accuracy or external dataset provenance.

The existing experimental geometric contract is unchanged: **ET/TDB seconds past J2000**, NAIF **J2000/ICRF frame ID 1**, **km** positions and **km/TDB-second** velocities for supported SPK Type-2/3 center chains. No light-time, aberration, topocentric conversions or independent NASA/JPL comparison is added.

## Regression coverage

Two deterministic, entirely synthetic tests inside `spk_kernel_set.rs` verify:

1. With an already-loaded valid kernel, a valid proposed overlay followed by an invalid second file fails with input index 1 and a preserved source error; the original selected descriptor, direct state and relative state remain identical.
2. An empty batch is harmless; a successful mixed-endian Type-2/3 batch retains later-loaded priority, and `unload_last` restores the earlier state in order.

Exact-head checks:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

Read-only native Linux, macOS and Windows CI verifies implementation behavior. Synthetic fixtures are **not** evidence of parity with NAIF/SPICE/JPL, and third-party SPK kernels must be separately sourced and reviewed before use or redistribution. Original source remains **CPAL-1.0**; existing attribution, notices and project credit requirements are unchanged.

## Replacing the entire stack without a partial reload

**Added 2026-11-14.** `replace_all_atomic(new_sources)` is the companion to
`load_batch_atomic`: it checks every replacement DAF index in a temporary
`SpkKernelSet` before swapping the complete new ordered stack into place.

```rust
use std::fs::File;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut kernels = SpkKernelSet::new();
kernels.load(File::open("/local/old.bsp")?)?;
let replacements = vec![
    File::open("/local/new-first.bsp")?,
    File::open("/local/new-second.bsp")?,
];
let retired_readers = kernels.replace_all_atomic(replacements)?;
// The new stack is active; old reader ownership is returned in load order.
// Explicitly drop old files when they are no longer needed.
drop(retired_readers);
# Ok(())
# }
```

On **any index-inspection failure**, the old stack is unchanged, including
the seek positions of already loaded readers. The error's `input_index`
identifies the invalid replacement file within the new batch, starting at
zero; all attempted replacement readers are consumed, even on error.
On **success**, the previous readers are returned in their original load
order, so callers can release them or inspect them separately. Unlike
`load_batch_atomic([])`, calling `replace_all_atomic([])` **deliberately
clears** the active stack and returns its previous readers. This explicit
difference matters when replacing external configurations.

This operation is **not thread synchronization or a filesystem transaction**,
and it does not certify SPK coefficient records, physical accuracy or live file
immutability. It is the in-memory index-level all-or-nothing transition only.
Regression tests cover failures in both first and later replacement inputs,
preservation of direct/relative queries, cross-endian precedence, old reader
return, unload priority, and explicit empty replacement.
