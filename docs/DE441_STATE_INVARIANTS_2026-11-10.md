# Synthetic DE441 state invariants — 2026-11-10

This contribution adds `crates/ephemeris-core/tests/state_invariants.rs` on a feature branch based on `dev/ephemeris`, without editing the production reader or the overlapping PR #15.

The tests construct a minimal in-memory **classic little-endian DE441-shaped** binary fixture with a 32-day coefficient record. They exercise: (1) target/center antisymmetry; (2) transitivity of relative geometric vectors; (3) barycentric Earth–Moon mass-weighted reconstruction; and (4) the same invariants for velocity in km/day. Samples include both endpoints and interior epochs.

Scientific contract: epochs are JD TDB; geometric equatorial ICRF vectors, positions in km and velocities in km/day. No light-time, aberration, precession or apparent corrections are tested. The fixture is synthetic and is **not** an independent validation of real NASA/JPL DE441 values.

Verification: tests authored but not executed in this connector-only environment. Before merging, run `cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` on a runner with Cargo. Independent JPL/Horizons comparison remains an open gate.

License/provenance: new Rust test source carries CPAL-1.0 Exhibit A notice. No third-party datasets or algorithms were copied. This document records the 2026-11-10 modification under CPAL §3.3.
