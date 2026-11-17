# Checked TDB epoch bridge for classic DE441

**Date:** 2026-11-12  
**Status:** isolated experimental Rust-core addition; not a new DE441 reader, conversion of time scales or independent numerical validation.

## Why

`calendar::JulianDate` stores an integral day and midnight fraction separately. The current classic-DE441 reader `de441::De441::state` takes `de441::JulianDateTdb(f64)`, so converting a split JD to that legacy interface can lose microseconds around modern epochs. The pre-existing `JulianDate::to_de441_epoch()` transparently converts but has no user-specified precision gate.

`de441_epoch_bridge::checked_de441_epoch(split, max_error_seconds)` is an **optional**, explicit adapter: only already-TDB split dates are accepted, and a nonnegative, finite budget specifies the maximum allowed loss **from collapsing the two-part JD into one f64**. The return value carries both `JulianDateTdb` and the measured quantization error in seconds. A strictly zero budget is permitted for exactly representable epochs.

## Example

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::de441_epoch_bridge::checked_de441_epoch;

let split = JulianDate::new(
    2_451_545,
    0.5 + 0.25 / 86_400.0,
    TimeScale::Tdb,
)?;

// A caller-defined 1-millisecond input quantization budget.
let conversion = checked_de441_epoch(split, 0.001)?;
println!(
    "DE441 JD(TDB): {}, input quantization: {} s",
    conversion.epoch.0,
    conversion.quantization_error_seconds
);
// Pass conversion.epoch to the existing De441::state API.
# Ok::<(), Box<dyn std::error::Error>>(())
```

The adapter makes **no** UTC/TAI/TT/TDB conversion, leap-second assumption, date-frame correction, or astronomical-position error guarantee. A measured zero JD-rounding error does not establish any position accuracy, kernel provenance or JPL parity. It also does **not** change the reader to use a two-part epoch internally; a native split-JD/ET implementation is a separate future task.

## Validation

Six deterministic unit tests in `de441_epoch_bridge.rs` cover an exact J2000 epoch, quarter-second loss at J2000, a sub-day near midnight, historical negative JD, rejection of wrong time scales, invalid budgets and unrepresentable dates. No ephemeris file or external dataset is used; tests prove the conversion contract only.

Run with a Rust toolchain:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

**Independent scientific validation remains open:** a genuine classic DE441 binary and separately verified JPL Horizons vectors are still required to measure astronomical accuracy. Do not treat this adapter as Swiss Ephemeris equivalence or production readiness.

## Modification and licensing record

- **2026-11-12:** Added `crates/ephemeris-core/src/de441_epoch_bridge.rs` and exported it from `crates/ephemeris-core/src/lib.rs`. The old conversion and existing reader signatures remain unchanged.
- Original code remains CPAL-1.0 with VirgoStars attribution; no third-party code or ephemeris data imported. See [LICENSE](../LICENSE), [NOTICE](../NOTICE), and [Licensing and credits](LICENSING_AND_CREDITS.md).
