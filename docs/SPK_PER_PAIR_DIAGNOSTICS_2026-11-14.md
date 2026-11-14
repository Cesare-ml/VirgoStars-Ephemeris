# Per-pair geometric SPK diagnostics at one TDB epoch

**2026-11-14 · Experimental Rust core · No independent NAIF/JPL accuracy claim**

## Why a separate diagnostic from atomic batch?

The existing `spk_batch::relative_states_at` is deliberately **all-or-error**: the first failed target/center pair aborts the batch without returning earlier successes. This is correct for consumers that require all their calculations to be valid, but inconvenient for **offline kernel coverage investigation**, where one incomplete or unsupported center chain can hide otherwise valid requests.

`spk_pair_diagnostics::probe_relative_states_at(&mut kernels, pairs, et)` therefore evaluates **every** requested pair and returns one `SpkRelativeProbe` per pair, preserving its zero-based request index, IDs and original success/error result. No new interpolation, priority or center-chain algorithm is introduced: every result comes directly from the existing **canonical Rust** `SpkKernelSet::relative_state` method.

## Usage

```rust
use std::fs::File;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;
use virgostars_ephemeris_core::spk_pair_diagnostics::probe_relative_states_at;

fn example(path: &str, et_tdb_seconds: f64) -> Result<(), Box<dyn std::error::Error>> {
    let mut kernels = SpkKernelSet::new();
    kernels.load(File::open(path)?)?; // Explicit external local SPK file
    for row in probe_relative_states_at(
        &mut kernels,
        &[(301, 399), (399, 0), (301, 0)],
        et_tdb_seconds,
    )? {
        match row.result {
            Ok(state) => println!("pair {}: {:?} km", row.request_index, state.position_km),
            Err(error) => eprintln!("pair {} failed: {}", row.request_index, error),
        }
    }
    Ok(())
}
```

## Explicit scientific and error contract

- Epoch is a caller-supplied **finite TDB seconds-past-J2000** value, not UTC, TT or Julian date. Invalid global ET (NaN/±infinity) fails before attempting a pair, even when the requests are empty.
- All successful states are **geometric** target-minus-center in NAIF frame 1 **J2000/ICRF** with **km** position and **km per TDB second** velocity. SPK Type-2/3 and the already-implemented center-chain rules are the only supported evaluators. No light-time, aberration, topocentric or apparent adjustments.
- For finite ET, output length equals request count. Pair order and duplicate requests are preserved; per-pair result errors **do not terminate subsequent probes**.
- A failed pair retains its actual `SpkRelativeError`: disconnected ancestry, cycle, unsupported high-priority SPK type/frame, corrupt coefficient data or arithmetic error. No lower-priority fallback or fabricated vector occurs. A same-body pair returns the existing mathematical zero identity, **not proof of coverage**.
- Only the *outer* `Result` conveys invalid ET. Inspect **every row.result** before treating a diagnostic scan as scientifically usable. **Do not substitute this partial-result API for atomic production batch output**.
- The loaded kernel set is not reloaded, re-indexed or reordered by this API; the canonical evaluator may seek/read its already-loaded caller-provided SPK streams. No remote access or automatic kernel fetching is implemented.

## Test fixtures and limitations

Four Rust tests use original in-memory synthetic DAF/SPK Type-2 fixtures, including little and big endian kernels:

1. Mixed valid–invalid–valid requests, duplicate pairs, source order and component/rate agreement with individual canonical queries.
2. A newer, unsupported frame overriding an otherwise computable older segment: affected pairs fail, unrelated pairs succeed, and unloading the overlay restores valid results.
3. Empty kernel set, mathematical same-body zero results and explicit disconnected pairs.
4. Globally invalid ET for both empty and nonempty request lists.

These tests establish **routing, error isolation and API behavior**, not positional/velocity parity with actual NASA/JPL files. Validation with legally sourced SPK kernels, traceable independent SPICE references and approved quantitative tolerances is still required before scientific certification or production use.

## Verification and provenance

Run the existing nonprivileged CI against the final branch commit:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

New source and modifications dated **2026-11-14** remain under **CPAL-1.0**, preserve its Exhibit A and VirgoStars attribution, and do not introduce external source code, ephemeris binaries, additional dependencies, package publication or privileged Actions. `main` and production integrations are unchanged.
