# Chebyshev affine invariants — regression scope

Added 2026-11-11 under the repository CPAL-1.0 license and attribution requirements.

The integration test `chebyshev_affine_invariants.rs` exercises two independent algebraic identities for a finite Chebyshev coefficient series `P(x)` and its derivative:

1. Adding a constant `k` to coefficient `c0` changes `P(x)` by `k`, leaving `dP/dx` unchanged.
2. Multiplying every coefficient by a finite factor `a` scales both `P(x)` and `dP/dx` by `a`.

The test uses fixed coefficients, positive and negative factors, interior points, and both endpoints of the supported normalized interval. It does not change the public API, interpolate a real DE441 record, validate velocity units, or demonstrate parity with JPL Horizons.

Run `cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings`. This document does not assert these commands have passed. Independent real-kernel astronomical validation remains an open release gate.

No third-party ephemeris files or code are included. Preserve the existing `NOTICE` and visible VirgoStars attribution requirements when integrating.
