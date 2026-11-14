# Guarded split-TDB state lookup for classic DE441

**2026-11-12 — draft stacked on PR #136, not an independent complete astronomy pipeline.**

This stage connects the checked split-Julian-date conversion developed in [PR #136](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/136) to the actual experimental Rust classic-DE441 state reader. The existing reader API is unchanged, and this new API is **opt-in**.

## API

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::de441_checked_state::checked_state_at_split_tdb;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut reader = De441::open("/path/to/legally-obtained/de441.eph")?;
let tdb = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb)?;

let checked = checked_state_at_split_tdb(
    &mut reader,
    Body::Moon,
    Body::Earth,
    tdb,
    0.001, // maximum permitted input-JD quantization loss in seconds
)?;
println!("{:?}", checked.state.position_km);
println!("input rounding: {} seconds", checked.quantization_error_seconds);
# Ok(())
# }
```

The result includes a geometric target-minus-center `StateVector` in **fixed equatorial ICRF axes**, position in **km**, velocity in **km/TDB day**, and the measured input JD quantization in seconds. The caller must already have converted its epoch to **TDB**. The optional precision budget is finite, nonnegative, and in seconds. Invalid time scales, imprecise epochs, or budgets are rejected **before any DE441 coefficient-record I/O**.

The errors explicitly distinguish a split-epoch or precision error (`CheckedStateError::Epoch`) from the underlying DE441 file, data, coverage, or series error (`CheckedStateError::Ephemeris`). The adapter does not silently fall back to analytical data or recalculate on a nearby date if the budget is exceeded.

## Verification and limitations

The new unit tests construct a tiny original **synthetic** classic-DE441 record (not a NASA kernel) and check:

- exact J2000 geometric target-relative state, metadata, velocity and zero rounding;
- rejected submillisecond loss without coefficient-record I/O, followed by accepted lookup with an explicitly looser budget;
- wrong time scales and invalid error budgets rejected without coefficient-record I/O;
- genuine reader `OutsideCoverage` and `MissingSeries` errors surfaced unchanged.

Run the project's approved read-only CI including `cargo fmt --all -- --check`, `cargo test --workspace --locked`, optimized tests, Clippy, Rustdoc, and platform checks.

**This does not preserve split-TDB precision throughout Chebyshev interpolation.** It only prevents unapproved quantization in the preexisting single-`f64` JD API. The rounding loss is **not** a bound on ephemeris position/velocity error, and these synthetic fixtures do not establish JPL Horizons scientific parity, correction accuracy, time-scale conversion, SPK support, apparent positions, or production readiness.

## Licensing / change record

- 2026-11-12: added `crates/ephemeris-core/src/de441_checked_state.rs`, exported it through `crates/ephemeris-core/src/lib.rs`; added this documentation.
- Existing DE441 reader and conversion implementation remain unchanged. No external dependencies or JPL ephemeris data are included.
- Original code remains [CPAL-1.0](../LICENSE) with applicable `Powered by VirgoStars` attribution; see [NOTICE](../NOTICE) and [licensing guidance](LICENSING_AND_CREDITS.md).
