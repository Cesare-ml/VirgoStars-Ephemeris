# MVP-1: independent end-to-end reception-time rate regressions

**2026-11-12 — stacked test-only change on draft PR #70** (`feat/mvp1-geocentric-six-fields-20261111`).

## Why

The first MVP needs **positions and signed speeds** for Sun, Moon, and planets. Draft PR #70 connects the existing classic DE441 reader, reception-time one-way light-time solver, and fixed-axis spherical projection. Pure spherical projection tests and stationary synthetic examples cannot alone establish that the composed six-field adapter returns the *reception-time derivatives*, rather than a raw geometric target velocity or a position-only result.

The new offline Rust integration test `crates/ephemeris-core/tests/geocentric_six_fields_end_to_end_rates.rs` constructs a fictional, single-record classic DE441 binary. The test source explicitly marks all vectors as synthetic; it **does not contain real NASA/JPL data**, and adds no runtime dependency or network requirement.

## Test contracts

1. **Analytic radial oracle:** stationary geocentric observer, target moving along the positive X axis at constant `v` km/TDB day, initial range `r0`. With `c_day = 299792.458 * 86400` km/TDB day, a one-way *reception-time* solution satisfies `r = r0 / (1+v/c_day)`, `dr/dt_obs = v / (1+v/c_day)`, `delay = r/c`, `d(delay)/dt_obs = v/(c_day+v)`. Compare all four quantities with the actual one-call output, including km→exact IAU AU conversion, and check that angular rates vanish in this one-dimensional case.
2. **Numerical full-stack derivatives:** a fictional transverse and vertical *linear* target trajectory in a classic DE441 Chebyshev record. Query the **whole one-call pipeline** at reception TDB JD `t-h`, `t`, `t+h`, with `h = 0.02` TDB day. Compare all three published speed fields with independent centered differences of the separately computed output fields. Longitude differences are reduced to the signed shortest angular separation so a 359°→0° transition cannot masquerade as retrograde or a 360° jump.
3. **Signed retrograde and center metadata:** negative transverse velocity must yield a negative longitude speed. A second fictional series for `MarsBarycenter` is returned with that exact identifier and `Earth` center, never relabeled as the physical Mars center.
4. **Retarded coverage:** an observation at the kernel's first valid reception epoch is not automatically usable if its required *emission* epoch lies before file coverage. The typed nested `OutsideCoverage` error must propagate. The end-of-coverage reception case remains valid when the emission epoch is inside the file.

These checks use a 9 KB fixed synthetic kernel with the standard 3000-byte classic coefficient record. Polynomial position coefficients are in km and velocities in km/**TDB day**; light time is seconds. Returned longitudes, latitudes and their speeds are degrees and degrees/TDB day; distance and radial speed are AU and AU/TDB day with exact IAU 2012 AU = 149597870.7 km.

## Scientific and integration limits

This scope is **fixed-ICRF geometric + one-way light-time only**. No annual aberration, relativistic deflection, precession/nutation, moving-date-frame rates, parallax/topocentric correction, refraction, planetary-center reconstruction, validated real-kernel accuracy or `swe_calc_ut`/Swiss parity is implied. Only a **real classic DE441** external file can provide real trajectories, with independent Horizons comparisons required for any scientific claim.

Draft PR #70 remains a prerequisite; the regression imports the APIs added in that PR and therefore **cannot be merged independently into `dev/ephemeris`**. Keep the test branch stacked against the PR #70 feature branch for concise review and avoid overwriting other active integration PRs (#46/#73/#76/#77).

## Verification

Run in an environment with stable Rust:

```bash
cargo fmt --all -- --check
cargo test --workspace --test geocentric_six_fields_end_to_end_rates
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The repository's authorized read-only GitHub Actions may validate this stacked branch via a temporary CI-only draft PR against `dev/ephemeris`; no release, deployment, use of GitHub Actions for development, or merge is authorized.

## License / source modification log

2026-11-12: added the new independent synthetic Rust test and this document; existing implementation in PR #70 was **not modified**. Retain the CPAL-1.0 Exhibit A notices and the applicable “Powered by VirgoStars” attribution and https://www.virgostars.it/ link. No Swiss/ERFA/NAIF source code or NASA/JPL files were copied.
