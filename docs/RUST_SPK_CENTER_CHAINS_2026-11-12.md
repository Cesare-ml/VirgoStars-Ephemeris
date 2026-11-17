# Experimental SPK relative states through center-chain composition

**Date:** 2026-11-12. **Status:** additive experimental Rust feature, synthetic tests only, **dependent on draft PR #148 (SPK index) and #150 (direct Type 2/3 state evaluator)**. Review and integrate dependencies first; do not merge this change independently.

## Scientific contract

`spk_relative_state::relative_spk_state(&mut source, &index, target_id, center_id, et)` computes the **geometric state of one NAIF body relative to another** from the single externally supplied DAF/SPK kernel represented by `index`. The caller must pass the **same seekable source** from which `inspect_spk` built the index. No network access or bundled kernel files are needed.

- **Time:** finite TDB/ET **seconds past J2000**; no UTC, TT or TDB conversion is done here.
- **Frame:** only the segment evaluator's **NAIF frame 1 (J2000/ICRF)**. All required constituent segments must use this same fixed frame; unsupported frames fail.
- **Units:** three position components in **km** and three geometric velocity components in **km/s**.
- **Reference:** target minus center. The returned `common_ancestor_id` indicates the first shared ancestor along the target's chain. This need not be body 0 (solar-system barycenter): a Moon→Earth direct segment is enough to compute Moon relative Earth even if the Earth→SSB link is missing.
- **Single-kernel priority:** for each target ID, the **last segment whose coverage includes ET** wins, even when another segment for that ID declares a different center, frame or type. Selection never falls back to an earlier competitor when the winner is unsupported.
- **Body-center chains:** follow declared center IDs for target and observer separately; find the first shared ancestor from the target path; reuse `direct_spk_state` for **only the links needed before that ancestor**. Compute target-to-common minus center-to-common, including both velocities. The constant identity target-relative-to-itself returns exact zeros for finite ET.
- **Fail-closed policy:** cycles in the selected center chain, disconnected paths, unsupported required type/frame, truncated or inconsistent coefficients and nonfinite accumulated vectors are explicit errors; no guessed or fallback ephemeris is returned.

## Usage

```rust
use std::fs::File;
use virgostars_ephemeris_core::spk_index::inspect_spk;
use virgostars_ephemeris_core::spk_relative_state::relative_spk_state;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut source = File::open("/path/to/legally-obtained/de440s.bsp")?;
let index = inspect_spk(&mut source)?;
let et = 0.0; // TDB seconds past J2000
let moon_relative_earth = relative_spk_state(&mut source, &index, 301, 399, et)?;
println!("relative position: {:?} km", moon_relative_earth.position_km);
println!("relative velocity: {:?} km/s", moon_relative_earth.velocity_km_per_second);
# Ok(())
# }
```

**The example requires a kernel with all necessary segments and supported J2000 Type-2/3 data.** It does not guarantee these segments exist in any specific real `.bsp` file or claim independently verified lunar accuracy.

## Deliberate limitations

This is **not equivalent to NAIF SPICE `spkez`**: only one indexed kernel, no multi-kernel load precedence, no frame transformations, no non-J2000 segments, no SPK types other than 2/3, and no aberration, light-time or apparent positions. A cyclic branch is conservatively rejected even if a shared ancestor might make higher links unnecessary. Double-precision addition/subtraction can lose small differences when two unrelated large absolute vectors cancel; this is not a published positional error bound. Zero vectors for an identical body express a mathematical identity, **not** an assertion that body ephemeris data were found.

The module shares the canonical `select_covering_segment` lookup with `spk_direct_state`, and calls the existing Rust Chebyshev engine for each required edge. It does **not** duplicate polynomial evaluation.

## Validation

Nine original synthetic, in-memory binary DAF/SPK regressions exercise:

- Moon relative Earth without a full Earth→SSB chain, and antisymmetry, in both little- and big-endian files.
- Moon→Earth→EMB→SSB and Mars→SSB branch composition; velocity signs and common ancestor.
- Identical body zero vectors with nonfinite-ET rejection.
- Disconnected paths, explicit cycle errors, and no cross-kernel approximation.
- Later segment selecting a **different** central body, changing both its coefficients and the center chain.
- Required unsupported frame/type rejection before fallback.
- Nonfinite sum overflow, and epochs without covering data.

The existing public Rust CI checks `cargo fmt --all -- --check`, `cargo test --workspace --locked`, optimized tests, strict Clippy, Rustdoc, Windows GNU target and Windows/macOS native Rust tests. All results must be inspected before claiming this change is validated.

**Independent NASA/JPL scientific accuracy remains unverified**: real SPK files must be obtained lawfully, hashed, used locally, and compared against matching NAIF/Horizons geometric ICRF vectors and velocities over a reviewed epoch/target matrix. Synthetic success is not real-kernel compatibility or production readiness.

## Provenance / modification record

- 2026-11-12: added `crates/ephemeris-core/src/spk_relative_state.rs` and one module export in `src/lib.rs`, factored the shared last-covering selection in `src/spk_direct_state.rs`, and updated English documentation/README.
- Original Rust code remains **CPAL-1.0**, required VirgoStars attribution preserved. See [LICENSE](../LICENSE), [NOTICE](../NOTICE), [licensing guidance](LICENSING_AND_CREDITS.md).
- No Swiss Ephemeris or NAIF Toolkit implementation was copied, no JPL kernel files, data, dependencies, releases or production migrations were introduced.

Format and priority reference: [NAIF SPK Required Reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html), [NAIF kernel competition rules](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/kernel.html).
