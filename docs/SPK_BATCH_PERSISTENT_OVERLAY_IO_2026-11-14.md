# SPK batch persistent selected-source I/O and explicit unload — 2026-11-14

**CPAL-1.0 §3.3 modification record.** Added a multi-kernel batch regression to `crates/ephemeris-core/tests/spk_stream_io_faults.rs`; no production module or API changed. The existing CPAL Exhibit A, LICENSE, NOTICE and **Powered by VirgoStars** attribution remain unchanged.

## Difference from the existing transient I/O regression (#252)

PR #252 already establishes that a later batch request aborted by a **temporary** I/O error preserves the error chain and can be retried once the same source becomes readable again. This additional test verifies the **distinct persistent-source failure case**: a newer, highest-priority kernel has valid index metadata and a readable Type-2/3 record directory, but its selected Chebyshev coefficient bytes remain inaccessible. Calling `relative_states_at` must not substitute older kernel data; reissuing the request must continue to fail until the client explicitly unloads that overlay.

## Synthetic test contract

For both little- and big-endian ordering and each SPK Type 2 and Type 3:

1. Load a valid Earth-to-SSB (NAIF 399→0) kernel and a valid Moon-to-Earth (301→399) kernel.
2. Compute a four-request reference batch, including `(399,0)`, `(301,399)`, `(301,0)` and a duplicate `(301,399)`.
3. Load a newer opposite-endian Moon-to-Earth kernel whose metadata and directory are readable but whose coefficient record returns `PermissionDenied`.
4. Verify the entire batch fails on **zero-based index 1**, with exact IDs (301, 399) and chain `SpkBatchError → SpkRelativeError::Segment → SpkStateError::Io → io::Error(PermissionDenied)`.
5. Verify the broken newest kernel remains selected and a shortened batch beginning at the failing tuple now reports failure at **index 0**.
6. Explicitly unload the overlay and verify the complete original ordered batch and duplicate results are exactly restored.

Fixture data are entirely synthetic; no NASA/JPL kernel bytes, NAIF/SPICE vectors, or Horizons results are bundled, downloaded, or numerically compared.

## Scientific and operational boundaries

- Time: explicit finite **TDB/ET seconds since J2000**, not UTC or TT.
- Frame: fixed NAIF frame 1 **J2000/ICRF**; geometric target-minus-center vectors only.
- Units: positions **km**, velocities **km/s**.
- No changes to astronomical algorithms, kernel load precedence, time conversion, production applications or public interfaces.
- Passing this test is evidence of deterministic failure/rollback semantics, **not** independent numerical accuracy.

## Verify

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --test spk_stream_io_faults
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Require successful exact-head read-only CI on Linux, Windows and macOS before integration. This additive file change avoids the already merged `spk_batch.rs` transient-error test.
