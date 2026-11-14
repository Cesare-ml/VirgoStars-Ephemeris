# First-sky: IAU 2006 mean obliquity and rotation geometry regression

**Modification date:** 2026-11-14  
**Original developer:** Cesare-ml (VirgoStars project)  
**License:** original Rust source remains CPAL-1.0; see [LICENSE](../LICENSE), [NOTICE](../NOTICE), and [LICENSING_AND_CREDITS.md](LICENSING_AND_CREDITS.md).

## Scope

Added two focused offline Rust unit tests to `crates/ephemeris-core/src/first_sky.rs` without changing the implementation, public API, dependency graph, or production integration.

1. **Independent mean-obliquity anchor:** the IAU 2006 mean ecliptic obliquity returned by `mean_date_matrix` for **TT JD 2400000.5 + 54388.0** must match **0.4090749229387258204 rad**, with tolerance **1e-14 rad**. This is the published numeric fixture in ERFA's `t_obl06` conformance test, not an expected value calculated from the Rust routine under test.
2. **Rotation geometry:** for three explicitly supplied **TT Julian dates**, the ICRF-to-mean-equator-of-date Fukushima–Williams bias/precession matrix must satisfy `R × Rᵀ ≈ I` and `det(R) ≈ +1`, with tolerance **2e-14**. This detects orientation/reflection and orthogonality regressions independent of the matrix's numeric elements.

## Scientific contract and limitations

- **Input epoch:** TT Julian Date; **not** UTC or TDB, and no implicit time-scale conversion.
- **Geometry:** IAU 2006 **mean** bias/precession, equatorial frame of date. The result is **not** a true/apparent position, sidereal zodiac, or an Earth-orientation model.
- **Output checked:** mean obliquity (radians); dimensionless rotation orthonormality and handedness. No distance, velocity, observer, or ephemeris file is involved.
- **Provenance:** [ERFA `t_erfa_c.c`, `t_obl06` fixture](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c) and [ERFA `obl06.c` specification](https://github.com/liberfa/erfa/blob/master/src/obl06.c); see [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) for provenance. Only a numeric reference value is transcribed; no ERFA algorithm source is copied.
- These tests **do not** validate the DE441 reader against authentic JPL data, precession across the full historical DE441 coverage, nutation, refraction, light-time, or an apparent planet position.

## Reproducible verification

From the repository root, with Rust installed:

```bash
cargo fmt --all -- --check
cargo test --workspace first_sky::tests::iau_2006_mean_obliquity_matches_erfa_obl06_reference
cargo test --workspace first_sky::tests::mean_date_precession_matrix_preserves_orthonormal_right_handed_axes
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

All new checks use hard-coded, auditable numeric data and need neither network nor JPL kernels. Their existence is **not** evidence that they have been executed; record the exact commit's check results in the PR before merge.

## Subsequent gate

Run the checks on the feature-branch head, review the CI results, and then integrate only through the normal PR process into `dev/ephemeris`. Preserve a separate scientific-validation gate for real NASA/JPL reference vectors.
