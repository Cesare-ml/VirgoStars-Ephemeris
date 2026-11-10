# DE441 Chebyshev velocity finite-difference regression — 2026-11-10

This change adds a deterministic numerical cross-check in `crates/ephemeris-core/tests/de441.rs`: for six TDB epochs strictly inside synthetic Mercury Chebyshev granules, compare the returned velocity (km/day) to the central difference of two independently evaluated positions (km), using `h = 0.0001` TDB days.

The absolute acceptance threshold is `1e-7 km/day` per ICRF component. This checks the derivative scale (days, not seconds), sign and interpolation against an alternate numerical computation; it is not a comparison to external JPL/Horizons vectors. The synthetic fixture contains a quadratic Chebyshev polynomial and two records, so it can expose incorrect time scaling while remaining fast and offline.

## Verification

The GitHub Actions Rust CI is configured on `dev/ephemeris` to run locked workspace tests, release tests, formatting and Clippy (plus native Windows/macOS jobs). **Verified on this PR head `85853363e03d71c01bb76909ccc5f5976e8c6294`:** GitHub Actions run [#42811296833](https://github.com/Cesare-ml/VirgoStars-Ephemeris/actions/runs/42811296833) completed successfully on 2026-11-10. All three jobs concluded `success`: Linux `Rust fmt, test and Clippy` (including the configured release-test and documentation checks), `Windows native Rust tests`, and `macOS native Rust tests`. These results are from GitHub-hosted runners; no local Cargo execution was available in the connector environment. This is a synthetic regression, **not** independent NASA/JPL/Horizons validation.

## Scope and licensing

No astronomical production algorithm, public API, kernel file, runtime dependency, time scale or reference frame was changed. Original code remains CPAL-1.0 with existing VirgoStars attribution and Exhibit A source notices; this document records the 2026-11-10 modification. No Swiss Ephemeris code or third-party data was used.
