# Split TDB Julian-date entry points for geometric SPK queries

**Experimental Rust core · 2026-11-14 · Not independently JPL/NAIF validated**

## Goal

`spk_tdb_queries` connects two already implemented **canonical Rust** modules without adding a second time conversion or interpolation algorithm:

1. `calendar::JulianDate::to_tdb_seconds_past_j2000` converts the supplied **two-part date, explicitly on the TDB scale**, into a typed ET value; it does not first round the date into a single floating-point Julian day.
2. `SpkKernelSet::relative_state` and `spk_batch::relative_states_at` use that exact resulting TDB/ET epoch to produce one or multiple geometric target-minus-center states.

This closes a practical gap for consumers that have split Julian dates and would otherwise need to collapse precision or manually convert a time-scale-labeled date. The new functions do **not** convert dates between TT, TAI, UTC, UT1 and TDB.

## Public API

```rust
use std::fs::File;
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;
use virgostars_ephemeris_core::spk_tdb_queries::{
    relative_state_at_tdb_date, relative_states_at_tdb_date,
};

fn query(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut kernels = SpkKernelSet::new();
    kernels.load(File::open(path)?)?; // Caller-supplied local SPK data

    // Already-TDB calendar date: J2000 noon plus 0.25 TDB second.
    let date = JulianDate::new(
        2_451_545,
        0.5 + 0.25 / 86_400.0,
        TimeScale::Tdb,
    )?;
    let moon_minus_earth = relative_state_at_tdb_date(&mut kernels, 301, 399, date)?;
    let states = relative_states_at_tdb_date(
        &mut kernels,
        &[(301, 399), (399, 0), (301, 0)],
        date,
    )?;
    assert_eq!(states.len(), 3);
    println!("{:?} km", moon_minus_earth.position_km);
    Ok(())
}
```

The example only succeeds if the caller's kernel set actually covers all requested bodies and required center-chain links at that date with supported SPK Types 2/3 and NAIF frame 1.

## Scientific and error contract

- **Input time scale:** split `calendar::JulianDate`, already tagged `TimeScale::Tdb`. `TimeScale::Tt`, `Tai`, and `Ut1` cause `SpkDatedQueryError::Calendar(CalendarError::WrongTimeScale)` *before* the SPK is queried. UTC leap-second conversion is not offered by this API.
- **Epoch representation:** one finite binary64 **TDB seconds since J2000** value computed from split JD parts, retained in each output's `tdb_seconds_past_j2000`. The implementation does not manufacture additional ET precision; binary64 resolution degrades at large absolute ET.
- **Reference frame:** fixed NAIF frame ID 1, **J2000/ICRF**. **Target and center:** explicit signed NAIF IDs, not inferred solar-system bodies or zodiac signs.
- **Quantity:** geometric target minus center, positions **kilometres**, velocities **kilometres per TDB second**. No aberration, light time, precession, nutation, frame rotation, topocentric correction, house/zodiac methods, or sidereal/tropical coordinates.
- **Single error:** `SpkDatedQueryError::Relative` preserves the original SPK chain/format error. **Batch error:** `SpkDatedQueryError::Batch` preserves `SpkBatchError` with failed request index and original cause. All stages implement `std::error::Error::source()`.
- **Batch semantics:** one ET conversion, original request ordering and duplicates preserved. All-or-error; no partial output vector.
- **Empty and same-body requests:** finite, valid TDB date with an empty batch succeeds; invalid time scales and unrepresentable split JDs fail even for empty/same-body requests. Identical target and center returns an exact mathematical zero, not proof of kernel coverage.

### Why not a single-f64 JD first?

The existing `to_de441_epoch()` produces a single-f64 JD for the classic DE441 reader and may quantize modern submillisecond offsets. The SPK engine already requires TDB/ET seconds from J2000, and `to_tdb_seconds_past_j2000()` already computes that offset from the two split parts. This adapter does **not** replace the classic DE441 model, which has separate precision requirements.

## Regression coverage (synthetic only)

The new module exercises:

1. A quarter-second after J2000 and a mixed-endian two-kernel Moon → Earth → barycenter chain, checking fractional ET, km, km/s and equality with the canonical state solver.
2. Same-epoch multi-target batch preserving ordering and duplicate requests and matching individual states.
3. Rejection of wrongly labeled TT/TAI/UT1 before even a same-body or empty batch.
4. Out-of-range split Julian day rejection and finite-empty-batch behavior.
5. Preservation of nested indexed batch failures, unchanged kernel-set usability and explicit unsupported higher-priority frame rejection.
6. Same-body zero identity without asserting a body is present in the kernel.

All synthetic SPK buffers are original in-memory tests; **no NASA/JPL SPK files or independent NAIF/SPICE/Horizons numerical comparisons have been executed by these tests**. Real-kernel provenance, quantitative accuracy thresholds, independent reference vectors and production readiness are distinct review gates.

## Verification and licensing

Run the repository's read-only CI on the precise proposed head:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

Modification 2026-11-14: added `src/spk_tdb_queries.rs`, exported its module in `src/lib.rs`, and added this documentation. Original source remains **CPAL-1.0** and keeps the project's Exhibit A and mandatory VirgoStars attribution. No new dependencies, external data, runtime network calls, privileged workflows, production integration or releases.
