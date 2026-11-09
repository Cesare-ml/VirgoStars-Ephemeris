# Rust core — Phase 1: mathematical foundation

**Branch:** `feat/rust-core-foundation-cpal`
**Status:** implementation draft; **not compiled or tested with Cargo in the authoring environment**; not a scientific validation result.
**Date:** 2026-11-09

## Implemented in this branch

- A minimal Cargo workspace with an experimental, non-published `virgostars-ephemeris-core` crate, pure Rust and no runtime third-party dependencies.
- Explicitly typed TDB Julian day and seconds past J2000. **No UTC/TT/TAI/UT1 conversion**.
- Chebyshev polynomial evaluation and derivative using an extended Clenshaw recurrence; the coefficient convention is `sum(c_k * T_k(x))` without silently halving `c0`.
- A synthetic Type-2-style position record, whose position coefficients are kilometers and whose derivative yields km/s via division by the half-interval in seconds.
- Input validation for invalid or nonfinite epochs, radii, polynomial coefficients, and out-of-record evaluation.
- Synthetic tests for constants, polynomials/derivatives, endpoints, invalid input, XYZ position/velocity and J2000 reference.

This is **not** an SPK parser and is not yet a real planetary ephemeris. It does not load `.bsp`, resolve NAIF IDs, combine kernel segments, or compute apparent / topocentric coordinates.

## Reproduce manually (no GitHub Actions)

Install a stable Rust toolchain supporting edition 2021 (minimum version declared in workspace: Rust 1.75), then at the repository root:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo run -p virgostars-ephemeris-core --example synthetic_record
```

These are **instructions**, not claims they have been successfully run. When performed, record the OS, Rust version, tool versions, pass/fail outcomes and date in a review/PR.

## Required next milestone: safe SPK data access

1. Verify the NAIF DAF/SPK specification and binary parsing rules, including byte order, record alignment, summary/directory indexing and coverage ranges.
2. Implement a separate read-only `ephemeris-spk` crate. Start with documented SPK **Type 2** and **Type 3** records, reject unknown types explicitly rather than guessing.
3. Respect target/observer/frame IDs, barycentric vs planetary-center distinctions, unit contracts and TDB ephemeris time. Position Type 2 derives velocity from position; Type 3 has separate velocity coefficients.
4. Obtain a legal, reproducible NASA/JPL kernel fixture through an explicitly documented external download process; do **not** commit bulk kernels without rights review.
5. Compare computed outputs independently against authoritative JPL Horizons/NAIF results for representative bodies, dates, boundaries and frame conventions. Use declared angular, position and velocity error budgets rather than a broad 'Swiss parity' claim.
6. Add precision-safe time representation and verified time-scale conversions before supporting civil UTC dates or angular horoscope outputs.
7. Only after a validated Rust API exists, add WebAssembly/TypeScript and PyO3/maturin Python adapters with parity tests against that same core. PyPI and npm publishing require separate approval.

## Guardrails

- No Swiss Ephemeris source or licensed SPICE toolkit vendoring, even if behavior looks similar; use original code from documented scientific data formats/algorithms.
- No production VirgoStars or Astrolabio changes; no database changes; no publishing; no GitHub Actions.
- Distinguish 'unit tests on synthetic polynomials' from 'independently validated astronomical ephemerides.'
- Keep astronomical science in the Rust engine; `astrological-functions` remains a separate TypeScript package.
- Original code is covered by the repository's CPAL-1.0 license with Exhibit B VirgoStars attribution; preserve notices in source files and respect network-use and visible-credit requirements. A `publish=false` crate does not imply an already released package.
