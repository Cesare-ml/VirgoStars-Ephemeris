# MVP-1: reuse Earth's reception state across multi-target aberration

**2026-11-12 | Experimental, draft-only performance refactor on top of #107.**

## Motivation

Draft [#107](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/107) exposed `geocentric_aberrated_fixed_six_fields_many` by calling the complete scalar API for each target. That is scientifically consistent but needlessly repeated the **same** Earth-at-reception barycentric position/velocity/analytic acceleration interpolation for every target. A natal chart may request Sun, Moon and multiple planet barycenters at precisely the same reception TDB Julian date.

This improvement **does not change the public API** or introduce another astronomy implementation. It factors one crate-private helper from the existing analytic aberration path and lets the batch share the observer source state across targets.

### Before

```text
for each target:
  Earth.state_with_acceleration(reception_TDB)
  geocentric_light_time(target, reception_TDB)
  aberrate_light_time_state(lighttime, Earth velocity, Earth acceleration)
  spherical_from_aberrated_state()
  aberrated_spherical_to_degrees_au()
```

### After

```text
if target list empty: return empty after existing finite epoch/tilt check
Earth.state_with_acceleration(reception_TDB)       # ONCE for batch
for each target:
  geocentric_light_time(target, reception_TDB)
  aberrate_light_time_state(lighttime, shared Earth velocity, shared acceleration)
  spherical_from_aberrated_state()
  aberrated_spherical_to_degrees_au()
```

`src/aberration_state.rs` now delegates the **original scalar** `geocentric_light_time_annual_aberration_analytic` to the same `pub(crate)` helper `geocentric_light_time_annual_aberration_with_earth`. The helper requires an exact `StateVectorWithAcceleration` with `target=Body::Earth`, `center=Body::SolarSystemBarycenter` and `epoch=observation_epoch`; mismatched observer metadata fails closed with `AberratedStateError::InvalidIntermediate`. No Earth state becomes public mutable configuration, and no new physical model is implemented.

`src/aberrated_spherical.rs` keeps the same public `geocentric_aberrated_fixed_six_fields_many` contract, preserving **input order, duplicates**, geocenter, fixed axes, output in degrees and AU, **all three signed speeds per reception TDB day**, Earth acceleration provenance, emission epochs, light-time metadata, and all-or-error semantics. For nonempty inputs it evaluates the observer state once; on an empty input it returns no results *without* checking file coverage for unspecified targets. Missing Earth reception-epoch data and missing target emission-epoch series still produce their original typed nested errors.

This is an **architectural removal of redundant observer interpolation**, not a measured claim of overall runtime or I/O speedup. The existing `De441` coefficient-record cache already reduces some repeated record reads. Target light-time iterations, target interpolation and one per-target Lorentz boost are still computed individually. No hidden global cache, network calls, downloads, external ephemeris binaries or new dependencies.

## Scientific contract

- **Time:** explicit `JulianDateTdb`, **not UTC or UT1**. The separate civil input adapter in [draft #110](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/110) remains untouched; after integration it calls this batch transparently.
- **Target / observer:** target body relative to Earth's geocenter. Outer planet `Barycenter` variants refer to **planetary-system barycenters**, not physical planet centers.
- **Frame:** fixed ICRF equatorial axes at zero tilt; other tilts are **constant** passive rotations about X. No moving mean/true ecliptic-of-date, no `dR/dt` velocities.
- **Corrections:** existing iterated reception light time and special-relativistic Earth annual aberration, differentiating its direction with the existing analytically differentiated DE441 Chebyshev Earth acceleration. The math and units are unchanged.
- **Six outputs:** longitude, latitude (degrees), range (IAU 2012 AU), and their three **signed** reception-TDB-day derivatives in degrees/TDB day and AU/TDB day.
- **Not included:** solar deflection/Shapiro delay, precession/nutation, topocentric observer, complete physical planet-center reconstruction, new Python/WASM bindings, or independent JPL/Horizons numerical verification. **Not Swiss Ephemeris `swe_calc_ut` parity**. The original reader supports separately supplied *classic* DE441 binaries, not SPK `.bsp` or Swiss `.se1`.

## Offline regression evidence

The pre-existing [#107](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/107) synthetic tests already compare the batch bit-for-bit against scalar outputs, including all metadata and signed speeds, and validate independently centered rates. This change adds tests in `crates/ephemeris-core/tests/de441_analytic_acceleration.rs`:

1. New fresh-reader **exact batch-versus-scalar equality** for `[Moon, Mercury, Moon]` at reception dates `1016.0, 1024.75, 1032.0, 1048.0, 1064.0` TDB and constant tilts `0` and `π/6`. The fixture has quadratic Earth acceleration, two 32-day Chebyshev records, an exact record boundary and a final inclusive reception epoch. Each scalar reference uses a new reader so it cannot accidentally share the batch reader's interpolation cache.
2. Typed nested missing-Sun-series error after Mercury success, reader reuse on subsequent valid requests, original Earth out-of-coverage failure for nonempty batches, and documented lack of per-target coverage checks for empty batches.

The test file is a **fictional tiny classic-DE441** binary generated in memory; test success validates arithmetic consistency and reader composition, **not real NASA/JPL or Swiss numerical accuracy**. The intended CI commands:

```bash
cargo fmt --all -- --check
cargo test --workspace --test de441_analytic_acceleration
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

GitHub authorized **read-only CI** on Linux, Windows and macOS can run these checks through a CI-only temporary draft PR targeting `dev/ephemeris`; close that PR **without merge** after reviewing results.

## Dependency order, provenance and legal obligations

**Stacked base:** this small PR is based on the exact current draft #107 head. #107 itself depends on draft #77 (and its #76 → #73 → #46 chain). [Draft #110](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/110) uses #107 but this new optimization has **not** been integrated into that head; reconcile dependencies after review rather than silently merging or rewriting those PRs. Nothing is merged to `dev/ephemeris` or `main` without an explicit authorized integration decision.

**CPAL-1.0 modification record (2026-11-12):** modified `src/aberration_state.rs`, `src/aberrated_spherical.rs` and the synthetic test file `tests/de441_analytic_acceleration.rs`, plus this document. Existing Exhibit A source notices, `LICENSE` and `NOTICE` preserved. No external code, NAIF/Swiss/JPL binaries, new third-party dependencies or app production changes. Applicable attribution remains **Powered by VirgoStars** — https://www.virgostars.it/.
