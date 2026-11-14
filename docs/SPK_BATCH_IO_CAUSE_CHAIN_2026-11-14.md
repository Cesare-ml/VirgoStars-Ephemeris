# SPK batch I/O failure-chain and rollback regression — 2026-11-14

**CPAL-1.0 §3.3 modification record.** Added a synthetic multi-kernel I/O fault regression to the existing `crates/ephemeris-core/src/spk_batch.rs` unit tests. The original Exhibit A, LICENSE, NOTICE, and **Powered by VirgoStars** attribution are unchanged. No astronomy algorithm or public API was altered.

## Motivation

A multi-body batch can produce valid states for its first pairs and then encounter unreadable Type-2 coefficients in a later, higher-priority SPK kernel. Returning a partial result, falling back to a lower-priority kernel, or dropping the underlying I/O error would invalidate a scientific comparison without making the failure discoverable.

The existing `relative_states_at` implementation promises first-failure semantics: either the **entire** ordered vector of geometric states, or `SpkBatchError::Request` with the first failed tuple's zero-based index, IDs and causal `SpkRelativeError`. This test locks down that promise across the complete error chain when the kernel stream fails **after** successful DAF indexing.

## Deterministic scenario

The test runs twice, reversing little-/big-endian source order:

1. Load a synthetic Type-2 Earth-to-SSB kernel and a synthetic Moon-to-Earth kernel, then compute the original four-pair batch with a duplicate pair.
2. Load a newer Moon-to-Earth kernel with different numerical values. Its header, summary and Type-2 directory remain readable, but reading the selected coefficient region fails with `PermissionDenied`.
3. Assert that the batch rejects on **request index 1**, preserving target 301 and center 399, and the nested `SpkBatchError → SpkRelativeError → SpkStateError::Io → io::Error` chain.
4. Assert that a retry beginning at the failing pair reports **index 0**, and the failed selected kernel stays loaded and takes precedence.
5. Explicitly `unload_last` the broken overlay and verify the entire batch equals the previously computed original results, including ordering and duplication.

The test is synthetic and uses only in-memory DAF/SPK fixtures, not real NASA/JPL or NAIF data. The underlying type-2 position/velocity units remain **km** and **km/s**, the epoch is finite **TDB seconds past J2000**, and the geometric frame is NAIF frame 1 (J2000/ICRF). No UTC/TT conversion, apparent correction, package publication, or production integration is performed.

## Verify

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Require success of the exact-PR-head read-only CI jobs on Linux, Windows and macOS before integration. A passing test proves deterministic fail-closed behavior for these synthetic bytes, **not** independent numerical parity with official NASA/JPL/NAIF/Horizons ephemerides.
