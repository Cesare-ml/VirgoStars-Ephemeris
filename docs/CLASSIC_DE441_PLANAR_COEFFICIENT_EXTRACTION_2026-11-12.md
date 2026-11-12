# Classic DE441: read-only selected planar Chebyshev coefficients

**2026-11-12 · Experimental Rust integration · CPAL-1.0.**

This draft extends [PR #149's header-backed granule planner](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/149) with a read-only path through the **existing** classic DE441 file reader. This is the first step from metadata-only time slices toward actual per-slice coefficients; it is **not** an apparent geocentric ephemeris, a complete Chebyshev piecewise producer, or an independent JPL/Horizons accuracy result.

## Implemented API

```rust
use virgostars_ephemeris_core::de441::{
    Body, De441, JulianDateTdb, ClassicDe441PlanarCoefficientRecord,
};
use virgostars_ephemeris_core::de441_granule_plan::{
    plan_de441_relative_granules, ActiveSeriesGranule,
};

let mut reader = De441::open("/path/to/authorized-classic-de441.eph")?;
let plan = plan_de441_relative_granules(
    &reader.granule_layout(), Body::MarsBarycenter, Body::Earth,
    2451545.0, 2451545.5,
)?;

for slice in &plan.slices {
    for &selected in &slice.active_granules {
        let source = reader.planar_coefficients_for_granule(
            selected,
            JulianDateTdb(slice.start_jd_tdb),
            JulianDateTdb(slice.end_jd_tdb),
        )?;
        println!(
            "series={} record={} subrecord={} x_coefficients={}",
            source.selection.series_index,
            source.selection.principal_record_index,
            source.selection.subrecord_index,
            source.x_coefficients_km.len(),
        );
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

The example requires a separately supplied classic binary in a compatible date range, and is illustrative only; the repository includes **no JPL data files**.

The method is added to `De441<R: Read + Seek>`, without a secondary binary parser:

```rust
pub fn planar_coefficients_for_granule(
    &mut self,
    selection: ActiveSeriesGranule,
    requested_start: JulianDateTdb,
    requested_end: JulianDateTdb,
) -> Result<ClassicDe441PlanarCoefficientRecord, De441Error>
```

`ClassicDe441PlanarCoefficientRecord` returns the validated selection, the full granule's start/end TDB Julian days, the mathematical midpoint/radius **in Julian days**, and copied X/Y coefficients in kilometers using `sum(c_k * T_k(x))` with unhalved `c_0`. No original kernel bytes are redistributed.

The existing reader's principal-record cache loading and record-date checks are extracted into one private `load_record` method and reused by both `De441::state()` and the coefficient extraction method. A failed seek/read or unexpected record dates invalidate cache before accepting data, exactly as before. This avoids duplicate file readers and preserves the existing geometric state lookup behavior.

### Validation / fail-closed conditions

The extraction method independently checks every supplied `ActiveSeriesGranule`, including selections from third parties:
- Only classic **Cartesian position** series 0..=10; rejects nutation and libration or indices outside the pointer array.
- Series must be present; principal record and granule indices must exist in the reader's validated header.
- Requested TDB Julian-day interval must be finite, strictly increasing, covered by the complete file, and wholly inside the selected granule. It **never** silently extrapolates, resamples, clamps, or crosses a subrecord boundary.
- The full granule's midpoint must be strictly representable between its endpoints in f64 TDB Julian days; otherwise the extraction fails rather than returning an unusable time normalization.
- Header-valid record bytes are fetched through the reader's on-demand cache. **Every copied X and Y coefficient must be finite**; invalid bytes return a typed `De441Error`.

### Important geometric distinction

The returned values describe a **source series**, not necessarily a body-relative-to-Earth state.

For instance, `Body::Earth` requires **two independent source series**:
- series 2: Earth–Moon barycenter (EMB) coordinates;
- series 9: Moon relative to Earth (geocentric lunar vector).

The original reader's `De441::barycentric()` reconstructs Earth's barycentric coordinates from these sources and the DE441 Earth–Moon mass ratio. The new API does **not** relabel EMB coefficients as Earth coefficients or bypass that center-chain calculation.

Likewise, the outer-planet series here are system barycenters. They are geometric positions in fixed equatorial ICRF axes; no precession/nutation, light-time, aberration, apparent-of-date ecliptic frame, topocentric observer, UTC/TT conversion, or sidereal zodiac is performed.

### Relationship to mathematical longitude-rate modules

The `piecewise_chebyshev_enclosures` code from PR #144 expects **TDB seconds** and independently validated source-coefficient centering/relative-body conventions. This extraction returns **TDB Julian days** in its exact source normalization. Conversion and Earth/Moon center-chain composition must be performed with explicit precision and provenance checks rather than directly passing one source into the previous rate helper under an incorrect origin or unit assumption.

No proof of continuous record transitions or safe angular-jump upper bounds is supplied by extraction alone. A future producer must also combine all required XYZ states, calculate suitable apparent geocentric frame/correction derivatives, and independently validate rate/enclosure results against JPL/NAIF/Horizons.

## Synthetic Rust regression tests

An integration test `crates/ephemeris-core/tests/de441_planar_coefficients.rs` builds a **synthetic classic DE441 binary** in memory, including a structurally valid 441-word principal record, one granule for EMB and two granules each for Mars-barycenter and geocentric Moon. No third-party ephemeris data is used.

**Eight authored Rust tests** check:
1. The actual header parser, planner and extracted series X/Y coefficients.
2. Coefficient-based Chebyshev position/velocity parity with existing `De441::state()`.
3. Earth-center reconstruction via independently extracted EMB and geocentric-Moon coefficients, matched against the existing Mars-barycenter–Earth state.
4. Different consecutive subrecord coefficient locations and cache reuse.
5. EMB and Moon remaining distinct source series.
6. Missing series and non-Cartesian source index rejection.
7. Invalid principal/subrecord indices, nonfinite dates and cross-granule intervals.
8. Nonfinite source coefficient and altered record endpoint rejection.

A separate **Python reference byte-layout check** successfully reconstructed the 10,584-byte synthetic fixture and checked representative X/Y coefficient offsets; this verifies the fixture design, **not** compilation/execution of the Rust tests.

Native Rust test suite **not run** in the assistant environment (neither `cargo` nor `rustc` is installed). The following commands must be executed on a Rust-equipped checkout before accepting or merging this draft:

```sh
cargo fmt --all -- --check
cargo test --workspace de441_planar_coefficients
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**Scientific limitations:** real DE441 binary fixture and independent NASA/JPL validation are not present or executed. No claims about Horizons numerical parity, ephemeris error budget, rounding-certification, full-interval physical continuity or production use. Do not interpret synthetically passing tests as planetary-accuracy evidence.

## Next milestones

1. Extract compatible **three-axis XYZ** coefficients and validate true time-scale conversion without hiding rounding loss.
2. Compose Earth-relative position Chebyshev polynomials from the actual required sources, using EMB and geocentric Moon with explicit center-chain/units.
3. Verify interval coverage and record boundary jumps with independently justified error bounds, including real DE441 fixtures.
4. Move from geometric fixed ICRF directions to *validated apparent Earth-centered ecliptic-of-date* longitude envelopes, then a thin Rust/WASM adapter for the separate [Astrological Functions PR #11](https://github.com/Cesare-ml/astrological-functions/pull/11).

No main merge, deploy, package publication, GitHub Actions edits, license edits, external kernel copies or dependencies. Original VirgoStars source code remains licensed under **CPAL-1.0**, and the repository LICENSE/NOTICE controls the required **Powered by VirgoStars** / https://www.virgostars.it/ attribution.
