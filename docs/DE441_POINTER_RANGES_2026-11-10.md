# DE441 pointer-range validation — 2026-11-10

Modification record (CPAL-1.0 §3.3). Based on the original VirgoStars-Ephemeris source by Initial Developer Cesare-ml (VirgoStars project).

## Change

- Validate that each declared coefficient series occupies a distinct half-open range of 1-based coefficient words.
- Reject overlapping series with `De441Error::InvalidFile("overlapping coefficient pointers")` at header parsing time.
- Preserve valid adjacent series and absent series.
- Add synthetic positive/negative regression tests; update reader documentation.

## Scientific contract and provenance

No change to astronomical interpolation, centers, units (km, km/day), geometric ICRF frame or Julian Date TDB. No kernel files, copied external algorithms, third-party source or dependencies added. CPAL-1.0 and attribution remain unchanged.

## Verification

Local Cargo, Rustc and Rustfmt are unavailable in this execution environment. The code was reviewed statically; the new tests have **not** been run locally. Before merging, execute:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

This tests parser integrity with synthetic data; it is **not** independent numerical verification against NASA/JPL or Horizons. The existing PR #17 contains a separate Rust CI proposal. Avoid merging before CI checks on this branch pass.
