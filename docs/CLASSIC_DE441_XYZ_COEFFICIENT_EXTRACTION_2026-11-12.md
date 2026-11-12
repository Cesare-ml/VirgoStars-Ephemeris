# Classic DE441 XYZ Chebyshev coefficient extraction

**Status:** experimental, not independently NASA/JPL/Horizons validated.
**Added:** 2026-11-12. **Branch:** `feat/classic-de441-xyz-coefficient-extraction-20261112`, stacked on [draft PR #153](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/153) and the earlier granule timeline drafts.
**License:** original VirgoStars CPAL-1.0 source headers, attribution and NOTICE unchanged.

## Scope

This PR extends the existing read-only planar extraction from [PR #153](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/153) to a complete **three-axis Cartesian coefficient record**, rather than introducing an alternative binary parser, copying third-party ephemeris data, or implementing a second astronomical engine.

The public API added to the original `De441<R: Read+Seek>` reader is:

```rust
pub fn cartesian_coefficients_for_granule(
    &mut self,
    selection: ActiveSeriesGranule,
    requested_start: JulianDateTdb,
    requested_end: JulianDateTdb,
) -> Result<ClassicDe441CartesianCoefficientRecord, De441Error>
```

The output records the chosen series, principal record, subrecord, full granule start/end/midpoint in **TDB Julian days**, radius in **TDB days**, and exactly three **owned** `Vec<f64>` coefficient arrays `xyz_coefficients_km: [Vec<f64>; 3]` in **X, Y, Z** order. Coefficients follow the existing Rust evaluator convention `sum(c[k] * T_k(tau))` with **unhalved c0** and `tau = (JD_TDB - midpoint_jd_tdb) / radius_days`.

All three dimensions are geometric ICRF-position source-series data in kilometers. They are **not automatically geocentric or apparent**. In particular:

- Classic DE441 series **2** is the Earth–Moon barycenter (EMB), and series **9** is the **geocentric Moon**. Earth barycentric state must be reconstructed as `EMB − Moon_geo/(1 + EMRAT)`, with the EMRAT value read from the actual file. The existing `De441::state()` already performs that composition; this method does **not** yet build Earth-relative coefficient tracks.
- `Body::MarsBarycenter` and other outer-planet names are **system barycenters**, not planetary centers. The native source series cannot silently acquire a different physical-center label.
- The records are in fixed ICRF equatorial axes, not ecliptic-of-date longitude. This API performs **no light-time, aberration, precession/nutation, topocentric correction, time-scale conversion or astronomical validation**.

## Reuse and fail-closed behavior

The original X/Y-only `planar_coefficients_for_granule()` remains publicly available for compatibility, but now delegates to the new XYZ method, discarding Z **only after the complete X/Y/Z record has passed validation**. Consequently, a nonfinite Z coefficient now invalidates planar extraction as well; this tightening of previously incomplete validation is intentional.

The methods share the same existing read cache and the private `load_record()` checks from PR #153. Before reading they validate:

- Cartesian position source-series indices 0..=10 only; pointer must exist. Nutation/libration and unknown indices fail.
- Header-bounded principal-record and subrecord indices.
- Finite, strictly increasing requested TDB Julian-day interval **entirely contained** in the full source granule; no extrapolation, clamping or implicit crossing of record boundaries.
- Strictly representable positive midpoint/radius for the requested source granule, to avoid a collapsed `f64` interval.
- Every X/Y/Z source coefficient must be finite; missing or corrupt series data fails with `De441Error`. The caller receives no partially valid XYZ record.

The method does not claim that a mathematically representable `f64` JD midpoint is a scientifically adequate time-precision budget; that requires separate verification and possibly a split-time representation for real JPL work.

## Synthetic regression tests

The existing integration fixture in `crates/ephemeris-core/tests/de441_planar_coefficients.rs` constructs a fully synthetic **10,584-byte classic-DE441-format** source with three position series: EMB, Mars system barycenter and Moon geocentric, with different granule counts. The fixture data is original to this project and is not a JPL ephemeris redistribution.

**Five new authored integration tests** (13 total in the same test file) cover:

1. XYZ coefficients and analytic X/Y/Z position and velocity parity at both Mars subrecords against `De441::state()`.
2. Full X/Y/Z geometry and velocity for `MarsBarycenter − Earth`, composed from extracted series **2, 3, 9** and the synthetic file's EMRAT, compared to the original Rust reader at two epochs.
3. Consistency between the old planar and new Cartesian APIs, with shared cache and identical granule/time metadata.
4. Fail-closed rejection of a corrupted Z coefficient by **both** XYZ and legacy X/Y readers, while an unrelated noncorrupt series remains readable.
5. Strict rejection of unsupported source series, invalid granule selection, cross-granule time requests and nonfinite time inputs.

### Evidence and validation limits

The committed Rust tests have been **authored but not run** in this assistant's environment; `cargo` and `rustc` are unavailable. There is **no** claim that the tests, compiler, formatting, or Clippy have passed, nor any independent scientific comparison with an actual DE441 file or JPL/Horizons reference vectors.

Required native validation on the draft branch:

```sh
cargo fmt --all -- --check
cargo test --workspace --test de441_planar_coefficients
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Before merge, also verify the original `De441::state()` semantics are unchanged and review f64 time rounding at the real-kernel granule boundaries. This stack overlaps other active DE441-reader PRs, so preserve their work rather than force-merging.

## Next development steps

1. Build a validated **weighted source-series decomposition** for requested target/center pairs, including EMB + Moon-geo correction for Earth, using the actual kernel EMRAT and appropriate barycenter labels. Different Chebyshev record grids must retain their independent TDB normalization; simply subtracting unaligned coefficient vectors would be wrong.
2. Select and extract each required XYZ source for every slice from the [PR #149](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/149) timeline; enforce exact coverage and a documented numerical error model.
3. Apply interval-speed bounds and **independently proven** record-boundary jump limits to the [PR #144](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/144) piecewise longitude module. The existing caller assertions are *not* scientific certification.
4. Independently validate real Earth-centered apparent ecliptic-of-date positions/rates against NASA/JPL/NAIF/Horizons, then provide a thin Rust/WASM interface to the [Astrological Functions PR #11](https://github.com/Cesare-ml/astrological-functions/pull/11) provider.

No GitHub Actions workflow, release, deployment, new runtime dependency, license change, main merge, third-party source code or external kernel added. All source notices and `Powered by VirgoStars` / https://www.virgostars.it/ attribution remain as required by LICENSE and NOTICE.
