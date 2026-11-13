# Experimental Rust SPK Type 2 and Type 3 direct states

**Date:** 2026-11-12. **Status:** experimental, synthetic tests only. This feature is stacked on draft PR [#148](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/148) which adds the read-only DAF/SPK segment index. Do not merge independently before #148.

## Supported contract

`spk_direct_state::direct_spk_state(&mut source, &index, target_naif_id, et)` evaluates the **last covering segment for one target in one indexed kernel**.

- The caller must provide the **same external SPK stream** used for `inspect_spk`; a different stream or manually modified index is unsupported.
- Input ET is finite **TDB seconds past J2000**. Coverage endpoints are inclusive. Last-covering *file-order* precedence is applied **before** checking type and frame. No fallback to an earlier covering segment when the selected segment is unsupported.
- Only **NAIF frame ID 1** (fixed J2000/ICRF axes), and **SPK Types 2 and 3** are evaluated.
- Output is the **primitive target relative to the segment's own center**, with position **km** and velocity **km/s**. The result identifies the target, center, frame and ET. It is not Earth-relative or barycentric unless the chosen segment's center is already that reference.
- Type 2 has three series of Chebyshev **position** coefficients. Velocity is the analytic derivative multiplied by `1 / radius_seconds` using the existing canonical Rust `evaluate_chebyshev`.
- Type 3 has six *independent* coefficient series. Its velocity values are evaluated directly from series 4–6; do **not** differentiate the position coefficients to replace those independent series.
- Each segment ends with four numbers `INIT, INTLEN, RSIZE, N`. The reader checks integral positive record counts/sizes, finite positive interval, coverage, exact segment address-span agreement, record midpoint/radius, normalized Chebyshev bounds, and finite outputs. A single bounded coefficient record is read on demand, not the full kernel.

These capabilities **do not implement** multi-kernel load precedence, target-center chain composition, arbitrary frame rotation, apparent/astrometric corrections, UTC/TT/TDB conversion, light-time, observer positions, or SPK types other than 2/3. Failure to find a valid direct segment is an error, never a silently substituted orbital model.

## Usage

```rust
use std::fs::File;
use virgostars_ephemeris_core::spk_index::inspect_spk;
use virgostars_ephemeris_core::spk_direct_state::direct_spk_state;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut source = File::open("/path/to/legally-obtained/de440s.bsp")?;
let index = inspect_spk(&mut source)?;
let target = 399; // NAIF Earth ID; exact primitive center depends on file
let state = direct_spk_state(&mut source, &index, target, 0.0)?;
println!("target {} center {}: {:?} km", state.target_id, state.center_id, state.position_km);
println!("velocity: {:?} km/s", state.velocity_km_per_second);
# Ok(())
# }
```

This example does not assert that the chosen kernel actually contains an Earth direct segment covering ET=0 or a specific center; the result must be checked by the caller.

## Tests and independent verification

Seven original deterministic unit tests use tiny in-memory DAF/SPK files with both **little- and big-endian** descriptors and data, two synthetic Chebyshev records, and known polynomial values. They cover:

1. Type-2 position evaluation and analytical **km/s** velocity in both byte orders.
2. Type-3 independently stored **km/s** velocity, deliberately different from the position derivative.
3. Boundary selection: the following record at a shared boundary and the final inclusive endpoint.
4. Missing targets, nonfinite ET and epochs outside coverage.
5. Later covering unsupported frame/type failures instead of fallback.
6. Invalid directory size/count/span/interval/epoch.
7. Invalid record geometry or nonfinite coefficients.

Execute CI via `cargo fmt --all -- --check`, `cargo test --workspace --locked`, release tests, strict Clippy, Rustdoc and cross-platform Rust tests. All synthetic tests establish only *internal parser/interpolator behavior*, **not JPL/Horizons accuracy or real-kernel compatibility**. Before claiming any real capability, the engineering acceptance gate requires independently hashed/obtained actual SPK kernels and matched NAIF/Horizons states at explicitly documented, reviewed tolerances.

### Format sources

- [NAIF SPK Required Reading, Types 2 and 3](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html)
- [NAIF DAF Required Reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/daf.html)

These are descriptions of documented formats; original VirgoStars Rust code does **not** copy the NAIF SPICE Toolkit implementation. No kernels, third-party code, or datasets are redistributed.

## Licensing / modification record

2026-11-12: Added `crates/ephemeris-core/src/spk_direct_state.rs`, exported it through `crates/ephemeris-core/src/lib.rs`, and updated the experimental README. Original source remains under [CPAL-1.0](../LICENSE) with the required VirgoStars attribution, [NOTICE](../NOTICE), and [credits](LICENSING_AND_CREDITS.md). No external dependencies and no publication or production changes.
