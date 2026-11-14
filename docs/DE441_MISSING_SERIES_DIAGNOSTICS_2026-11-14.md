# Classic DE441 missing-series diagnostics — 2026-11-14

Modification record under CPAL-1.0 §3.3. Original VirgoStars-Ephemeris source: Initial Developer Cesare-ml (VirgoStars project).

## Summary

The classic little-endian DE441 Rust reader now reports the **specific missing series name** rather than the ambiguous `MissingSeries("body")` fallback for the ten supported planetary/system-barycenter direct series. The special labels for the Earth-Moon barycenter and geocentric Moon are retained. For example, an incomplete synthetic file requesting a Jupiter barycentric state yields `MissingSeries("Jupiter system barycenter")`.

The public `De441Error` variant and its signature are unchanged. Only its `&'static str` diagnostic payload is more informative. Code comparing that string to the prior generic `"body"` will need to use the specific series name; prefer matching the error variant instead. No ephemeris calculations, time scales, axes, precision, input formats, numerical correction models, external data, libraries or production systems are changed.

## Synthetic regression

The new `crates/ephemeris-core/tests/de441_missing_series_diagnostics.rs`:

1. Creates a minimal classic-shaped DE441 header with an internally consistent file length and one coefficient record, but no planetary series.
2. Asserts specific diagnostic names for Mercury, Venus, Earth-Moon barycenter, the seven outer planet system barycenters and Sun.
3. Checks that an Earth barycentric query requires the EMB series and that a Moon-relative-to-Earth query requires the geocentric lunar series.
4. Tests that NaN, ±Infinity, extreme finite TDB Julian days and finite out-of-range days fail with `OutsideCoverage` **even if target equals center**, while the inclusive coverage endpoints permit a zero relative identity.

Epochs are synthetic Julian dates **already on TDB**; output state vectors are geometric in fixed ICRF axes (km, km/day). The test exercises input and diagnostic contracts, **not** numerical agreement with a real NASA/JPL DE441 file.

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --test de441_missing_series_diagnostics
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

This authoring environment has no installed Rust toolchain; the test has not been run locally. The repository's pull-request CI supplies Linux, macOS and Windows checks; do not merge until those checks pass. No licensed third-party data, kernel payload, code or dependencies were introduced.
