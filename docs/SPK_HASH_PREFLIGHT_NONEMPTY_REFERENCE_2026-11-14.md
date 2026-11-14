# Offline SPK SHA-256 preflight: reject empty vector receipts — 2026-11-14

**CPAL-1.0 §3.3 modification record.** Added a preflight completeness guard to `scripts/verify_spk_kernel_hashes.py` and deterministic regression tests to `scripts/test_verify_spk_kernel_hashes.py`. Original CPAL notices, LICENSE, NOTICE and **Powered by VirgoStars** attribution remain unchanged.

## Why

A reference CSV containing only its hash metadata and the required column header previously passed the offline SHA-256 script whenever the supplied SPK kernel bytes matched. Such a receipt provides **zero cases** for the independent NAIF/SPICE versus Rust comparison; calling the hash verification successful without making that missing evidence visible was misleading.

## Change

After reading the metadata and required CSV header, the script now requires at least one nonblank, non-comment data line. Otherwise it fails with:

```text
reference contains no astronomical vector rows
```

This is an **existence check only**. It intentionally leaves numeric parsing, row cardinality, finite-value enforcement, provenance and tolerances to the separate Rust comparator and scientific audit tools. It does not authenticate a purported NAIF reference and does not upgrade kernel hashes to numerical-parity evidence.

Legacy hash-only reference metadata remains supported when there is at least one vector row. Ordered optional kernel basenames and byte sizes remain unchanged. Failure is reported by the existing script CLI with a nonzero exit status; no changes to the public Rust API, astronomical models, kernel downloads or external dependencies.

## Regressions

- Two synthetically constructed receipts with valid SHA-256 metadata but **no vectors** (header only, and whitespace-only tail) must both fail the parser, hash verifier and CLI.
- Restoring a single synthetic vector row must restore the previous successful hash-only preflight.

The fixtures use dummy kernel bytes, not NASA/JPL datasets.

## Verify

```sh
python3 -m unittest discover -s scripts -p 'test_verify_spk_kernel_hashes.py' -v
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Review the exact-head read-only GitHub Actions results. Successful tests certify only fail-closed behavior of this preflight, not Horizons/NAIF numerical accuracy. Final scientific release still requires authentic independently generated reference vectors, pinned kernel SHA-256 hashes, explicit frames/centers/time scales and approved numerical residual thresholds.
