# SPK stream I/O failure regressions — 2026-11-14

**CPAL-1.0 §3.3 modification record:** a new integration test was added for the canonical Rust `inspect_spk` and `direct_spk_state` APIs. All covered source notices, NOTICE, LICENSE and *Powered by VirgoStars* attribution remain unchanged.

## Failure conditions guarded

A binary DAF/SPK reader must not treat damaged or intermittently failing I/O as a valid segment index or finite astronomical state. The existing Rust implementation returns typed `SpkIndexError::Io` and `SpkStateError::Io` values through standard `std::io::Read + Seek` operations. This additional test suite exercises that contract using a small, original five-record in-memory DAF/SPK fixture.

- **Fragmented and interrupted reads:** simulate seven-byte `Read` fragments and one `Interrupted` error. `read_exact` must retry appropriately and retain the expected single-segment target/center and Cartesian position/velocity.
- **Summary read permission failure:** fail an attempted read in the summary physical record. Verify the index does not return a partial result, and both the typed `PermissionDenied` error and `Error::source` chain remain available.
- **Premature EOF in a summary:** advertise the full file size through `Seek`, but return EOF after a partial summary. Verify `UnexpectedEof` is returned rather than silently indexing incomplete metadata.
- **Selected coefficient record unreadable:** permit the directory footer read but deny its coefficient bytes. Verify `direct_spk_state` returns a typed I/O failure, not any target state.

Every fixture runs with both little- and big-endian IEEE file records; the state-reading cases additionally exercise SPK Types 2 and 3. No production code, API signature or dependency was changed.

## Scientific interpretation and limitations

Synthetic input: NAIF-style DAF/SPK file bytes only, **not** official NASA/JPL kernel data. Epochs are explicit TDB/ET seconds past J2000; Type-2/3 direct states use NAIF frame 1 (J2000/ICRF), target minus declared center, positions in **km**, velocities in **km/s**. These tests establish error propagation and deterministic synthetic state behavior only. They do **not** demonstrate independent Horizons/NAIF parity, binary compatibility with genuine kernels, time-scale correction, apparent states, or readiness for production.

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --test spk_stream_io_faults
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Run through least-privilege, read-only CI on the exact PR head (Linux and native Windows/macOS). No third-party files or network downloads are required; keep the PR isolated from concurrent SPK loader, CLI and numerical validation work.
