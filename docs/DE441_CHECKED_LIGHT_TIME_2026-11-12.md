# Checked split-TDB geocentric reception light-time

**Date:** 2026-11-12. **Status:** experimental Rust adapter, stacked on PRs [#136](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/136) and [#139](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/139).

## Purpose and scientific contract

`de441_checked_light_time::checked_geocentric_light_time_at_split_tdb` is an opt-in, Rust-only bridge between an already-TDB **split Julian date** and the existing `light_time::geocentric_light_time` solver. It checks the caller's permitted **observation-input rounding loss** before asking the DE441 reader for any coefficient record.

- **Input epoch/scale:** a `calendar::JulianDate` with `TimeScale::Tdb`; no UTC/TT/TDB conversion is implemented. A finite nonnegative tolerance specifies the maximum allowed two-part-JD-to-single-`f64` rounding error **in seconds**.
- **Target/center:** a `de441::Body` as target, Earth center at the reception epoch; target at iterated emission epoch. Earth itself is rejected as a target with undefined geocentric direction. Mars through Pluto are **planetary system barycenters** where indicated by the `Body` enum.
- **Frame and units:** output `GeocentricLightTimeState` uses fixed **ICRF** axes, position **km**, derivative wrt reception epoch **km/TDB day**, light-time **seconds**, and dimensionless light-time derivative. Both observation/emission epochs are **TDB JD**.
- **Corrections:** Newtonian one-way reception light-time only. **Not apparent position:** no aberration, light deflection, Shapiro delay, precession, nutation, topocentric observer, refraction or other corrections.
- **Fail closed:** rejects insufficient input precision or wrong time scales, then propagates undefined direction, missing DE441 series, out-of-coverage emission epochs and light-time nonconvergence. No fallback ephemeris is used.

This bridge guards the **observation input only**. The unchanged solver still iterates emission time using a **single-f64 JD**. Reporting a zero `observation_quantization_error_seconds` is **not** a bound on that emission-time resolution, astronomical positions, velocities or the scientific accuracy of DE441 interpolation.

## Minimal usage

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::de441_checked_light_time::checked_geocentric_light_time_at_split_tdb;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut de441 = De441::open("/path/to/legally-obtained/de441.eph")?;
let observation = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb)?;
let result = checked_geocentric_light_time_at_split_tdb(
    &mut de441,
    Body::MarsBarycenter,
    observation,
    0.001, // allowed input-JD rounding loss: at most one millisecond
)?;
println!("light-time: {} s", result.state.light_time_seconds);
println!("observation rounding: {} s", result.observation_quantization_error_seconds);
# Ok(())
# }
```

## Regression checks

Seven new deterministic tests in `src/de441_checked_light_time.rs` construct a **synthetic** one-record classic-DE441 file where Earth is stationary at zero and a Mars-system barycenter remains 150 million km away. The tests assert:

1. The geometric result is the known constant vector; reception delay is `distance / c`, rates and velocities are zero; a cached coefficient record is reused.
2. A too-strict observation budget rejects the epoch **before coefficient-record I/O**, while a looser budget permits the same input.
3. TAI/TT/UT1 input or invalid budgets cannot access coefficient records.
4. Geocentric Earth target is explicitly undefined.
5. Emission before the first file epoch does not extrapolate.
6. Missing target series propagates as a reader error.
7. Accepted output equals the old light-time API at the **same single-f64 rounded observation JD**, so the adapter does not alter its astronomical math.

Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo test --workspace --release --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings` plus the enabled public Rust development CI checks (Rustdoc, Windows cross-target, native Windows and macOS). Real JPL/Horizons parity is **not** validated by synthetic fixtures. A genuine externally obtained classic DE441 file and independent reference comparison remain required. No .eph or .bsp files are committed.

## Modification, provenance, license

- 2026-11-12: added `crates/ephemeris-core/src/de441_checked_light_time.rs` and its `lib.rs` module export, alongside this document.
- The existing light-time solver, DE441 kernel parser, calendar conversion and scientific algorithms are unchanged.
- Original code remains under [CPAL-1.0](../LICENSE), with the required VirgoStars attribution preserved. See [NOTICE](../NOTICE) and [licensing guidance](LICENSING_AND_CREDITS.md). No third-party code or ephemeris data was added.

**Review order:** merge/review #136 and #139 before considering this stacked adapter. Do not publish, deploy or migrate production based on these isolated deterministic tests.
