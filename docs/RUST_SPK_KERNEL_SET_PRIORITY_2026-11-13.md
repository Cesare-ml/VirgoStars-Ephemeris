# Experimental Rust SPK kernel-set priority and cross-file center chains

**Date:** 2026-11-13 · **Status:** experimental, synthetic tests only.

This change depends on three **unmerged draft PRs**, reviewed in order: [#148 DAF/SPK index](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/148), [#150 Type-2/3 primitive states](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/150), [#157 single-file center chains](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/157). **Do not merge this change before its dependencies.**

## Scope and public API

`spk_kernel_set::SpkKernelSet<R>` owns a sequence of independently indexed, externally provided SPK streams, with the source and `SpkIndex` kept together. `R` must implement `Read + Seek` and be the **same Rust type** across the set (for example `std::fs::File`). Kernels with differing IEEE endianness may be mixed.

- `SpkKernelSet::new()` constructs an empty set.
- `load(source)` validates the DAF/SPK index before adding the source. **A failed load adds no entry**; a successful load makes the new kernel highest priority. This is strictly append-in-load-order, without filenames/meta-kernels.
- `unload_last()` returns and removes the most recent source, restoring the remaining files' priority. `kernel_count()` and `is_empty()` expose the current inventory.
- `direct_state(target_naif_id, et)` returns a primitive **target relative to the winning segment's declared center**.
- `relative_state(target_naif_id, center_naif_id, et)` composes the necessary links from multiple files to their shared ancestor, **without duplicating Chebyshev evaluation or relative-vector composition**.

### Segment precedence

For a given target and finite epoch:

1. Search **last loaded SPK file first**, then earlier files.
2. Within each file, search **last physical segment first**, considering only that segment's target ID and coverage.
3. Apply frame, SPK type, coefficients and data checks **after** selecting the highest-priority covering segment. If it is corrupt, uses an unsupported frame/type, or creates a cyclic center chain, return an explicit error. Do **not** bypass it to earlier competing data.
4. Only when a higher-priority file has **no matching time coverage** may an earlier covering file provide the required link. Each body/center link is selected separately.

This follows the NAIF SPK precedence rules, including competing segments with different centers or frames: [SPK Required Reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html), [NAIF SPK training, segment priority](https://naif.jpl.nasa.gov/pub/naif/self_training/individual_docs/15_spk.pdf).

### Scientific contract

The output remains **geometric** at one finite **TDB/ET seconds past J2000** instant, in fixed **NAIF frame 1 (J2000/ICRF)**, with position **kilometres** and velocity **kilometres per second**. Only **SPK Types 2 and 3** can be evaluated. A primitive direct state identifies its selected center; a relative result identifies the requested target/center and the shared ancestor. An identical target and center gives an exact zero by mathematical identity only.

**Not included:** non-J2000 frames, other SPK segment types, light-time/aberration/relativity or apparent positions, TT/UTC to TDB transformations, SPICE kernels other than SPK, event searches, filename-based unload of arbitrary earlier loads, or dynamically selected heterogeneous source types. This is not an implementation of the full NAIF SPICE toolkit or a replacement for all NAIF `spkez` functionality.

## Usage

```rust
use std::fs::File;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut kernels = SpkKernelSet::new();
kernels.load(File::open("/path/to/legally-obtained/planetary.bsp")?)?;
kernels.load(File::open("/path/to/legally-obtained/earth-moon.bsp")?)?;
let moon_minus_earth = kernels.relative_state(301, 399, 0.0)?;
println!("Moon relative Earth: {:?} km", moon_minus_earth.position_km);
println!("Velocity: {:?} km/s", moon_minus_earth.velocity_km_per_second);
# Ok(())
# }
```

The example only succeeds if the supplied files' required segments actually cover ET=0 and are of supported type/frame. A given JPL file may use other centers and segment formats; synthetic tests do not establish compatibility with it.

## Validation and safety

Nine original synthetic unit tests construct small in-memory DAF/SPK files with known constant/linear Chebyshev data (no NASA kernels), verifying:

1. Cross-file Moon→Earth→EMB→SSB and Mars→SSB composition with different endian formats and mixed SPK Types 2/3.
2. Later-loaded same-target segment with **different center** changing the resulting center chain; removing the file restores earlier data.
3. Fallback to older data only outside the newer file's time coverage; inclusive start boundary.
4. An unsupported newer frame or type preventing fallback.
5. A higher-priority segment with a corrupt footer preventing fallback.
6. Cycles formed only after composing edges from separate files.
7. Later in-file descriptor precedence *within* the highest-priority file.
8. Invalid file rejection leaves inventory unchanged; empty set reports explicit missing target.
9. Identical-body zero, nonfinite epoch rejection and unloading the latest source.

These test behaviors, format handling, and strict platform checks are **not independent scientific validation**. Real SPK ephemeris files must be legally acquired externally, hashed, and compared against identical-kernel NAIF/Horizons geometric J2000 reference vectors and rates at approved tolerances before accuracy claims or production integration.

Run existing read-only Rust CI (`cargo fmt --all -- --check`, debug/release workspace tests, strict Clippy, Rustdoc, Windows GNU cross-target and native Windows/macOS checks). Report only **actual observed** results.

## Modification and provenance

2026-11-13: original implementation `crates/ephemeris-core/src/spk_kernel_set.rs`, refactoring of the preexisting geometric center-chain resolver to a shared internal `SpkStateResolver` interface, one `lib.rs` export, English documentation and README capability correction. All computational work stays in the canonical Rust engine; no duplicated astronomy in JavaScript or TypeScript.

Original source remains under [CPAL-1.0](../LICENSE) with the required VirgoStars attribution: [NOTICE](../NOTICE), [licensing guidance](LICENSING_AND_CREDITS.md). No Swiss Ephemeris/NAIF Toolkit source, kernel data, external packages, releases, deployments, or production modifications were introduced.
