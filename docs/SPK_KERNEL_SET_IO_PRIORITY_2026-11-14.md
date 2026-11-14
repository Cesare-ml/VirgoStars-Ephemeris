# Fail-closed SPK kernel-set I/O priority regressions — 2026-11-14

**Modification record, CPAL-1.0 §3.3.** Extended `crates/ephemeris-core/tests/spk_stream_io_faults.rs` with two multi-kernel integration regressions. Original Exhibit A source header, LICENSE, NOTICE and **Powered by VirgoStars** credit remain unchanged.

## Scientific / selection contract

`SpkKernelSet<R: Read + Seek>` selects the newest loaded kernel with a segment covering the requested target and **TDB/ET** epoch; within that kernel it selects the last covering segment. A successful metadata index followed by an unreadable selected coefficient record must return a typed error and **must never** fall back to an older, valid kernel. Otherwise real source corruption could silently produce believable coordinates from the wrong kernel.

The API supports SPK Type 2/3 direct geometric target-minus-center J2000/ICRF vectors; positions are **km**, velocities **km/s**. Relative states compose only the required selected links in those same axes. These tests do not add time conversions, light-time corrections, apparent positions, kernel provenance or astronomical model changes.

## What the tests exercise

1. **Failed load is atomic:** load a valid in-memory kernel, then attempt to load another whose DAF summary record returns `PermissionDenied`. Verify the error chain, kernel count, selected kernel load index and exact previously successful relative state remain unchanged.
2. **High-priority data read fails closed:** load an old valid kernel, then index a higher-priority kernel whose *coefficient bytes*, not header or metadata, return `PermissionDenied`. Verify `selected_segment` reports the newer kernel and both `direct_state` and `relative_state` reject the result (including nested `Error::source`), rather than silently returning data from the older kernel. Explicitly unload the failed overlay and verify the original state is restored.

Both tests run on synthetic little- and big-endian kernels, exercising Type 2 and Type 3 and mixed-endian load order. They extend the already integrated fault-injecting reader and reuse only dummy DAF/SPK bytes, not JPL/NAIF kernel data.

## Reproduce

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --test spk_stream_io_faults
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Require exact-head read-only CI on Linux, Windows and macOS before integration. A passing suite certifies only these deterministic failure and priority rules. It does **not** certify numerical equality with NASA/JPL, the NAIF Toolkit or Horizons; independently pinned kernel hashes, source metadata and quantitative reference-vector comparisons are still required.
