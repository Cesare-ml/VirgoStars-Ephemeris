# Split JD(TDB) from J2000 ET — 2026-11-13

## Scope and scientific contract

The new Rust `time_split::split_julian_day_tdb_from_et` function maps a
`TdbSecondsPastJ2000` value (TDB seconds past JD 2451545.0) to the
existing `calendar::JulianDate` model. The result stores an **integer
Julian day number (at noon) and a separate fraction since midnight**, with
`TimeScale::Tdb` explicitly attached.

This is only a representation conversion **within TDB**. It does not perform
UTC/TAI/TT/UT1 conversions, leap-second handling, TDB–TT modeling, correction
of reference frames, observer geometry or astronomical interpolation.
There are no target/center/body units involved.

## Why use a split Julian date?

At approximately J2000, a single IEEE-754 binary64 Julian date has a
spacing of roughly 40 microseconds. Directly adding a one-microsecond
offset to JD 2451545.0 drops that offset. Separating the day from the
midnight fraction retains the microsecond at this epoch, subject to the
finite precision of both the input ET `f64` and the fraction `f64`.

```rust
use virgostars_ephemeris_core::time_split::split_julian_day_tdb_from_et;
use virgostars_ephemeris_core::{TdbSecondsPastJ2000, J2000_JD_TDB};

let et = TdbSecondsPastJ2000::new(0.000_001)?;
let split = split_julian_day_tdb_from_et(et)?;
assert_eq!(split.day_number(), 2_451_545);
assert!(split.midnight_fraction() > 0.5);

// Collapsing the two parts back into one f64 JD still loses the microsecond:
let (whole, fraction) = split.parts()?;
assert_eq!(whole + fraction, J2000_JD_TDB);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The implementation uses separate Euclidean whole-day and remainder
calculations to maintain consistent day rollover for **positive and negative**
ET seconds. The function rejects values outside the split model's exactly
representable integer-day range and cases with an unrepresentable remainder.
At very large ET magnitudes, **the input f64 seconds itself** already loses
fine resolution; no interpolation accuracy is claimed by this conversion.

The classic DE441 reader still consumes a **single f64 JD(TDB)**; this
change does not upgrade that reader or make it scientifically validated.
An actual split-epoch reader would need its own precision and JPL tests.
Other in-flight temporal PRs may add further conversion/stepping methods;
they are independent and should be reviewed for API overlap before merge.

## Tests and integration rules

`crates/ephemeris-core/tests/split_tdb_et.rs` covers exact J2000/noon,
midnight and whole-day cases, positive/negative one-microsecond offsets,
the four sides of midnight, multi-year positive and negative offsets, and
fail-closed extreme values. These are **synthetic numerical contracts**,
not independent NASA/JPL/Horizons reference comparisons.

From the repository root:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## License and modification record

Modification on 2026-11-13: additive Rust `time_split.rs`, public module
export in `lib.rs`, new tests, and this document. No external ephemeris
kernel, proprietary source, new dependency, license text or production
application was changed. All original code remains under **CPAL-1.0** with
required attribution **Powered by VirgoStars**:
https://www.virgostars.it/.

## Synthetic legacy DE441 hand-off regression (2026-11-13)

The additional `split_tdb_de441_bridge.rs` integration test creates a
**synthetic classic DE441 binary in memory** and performs the complete
`TdbSecondsPastJ2000` → `JulianDate` split → legacy
`JulianDateTdb(f64)` → `De441::state` sequence. Its Mercury/SSB example
uses an exact two-day Chebyshev record with positions in **km**, velocities
in **km/day**, and an epoch already on the **TDB** scale. It verifies known
polynomial values and derivatives at both coverage endpoints, midnight,
J2000 noon and a one-second offset. UTC/TT conversion is never implied.

Importantly, the test also demonstrates a precision boundary: an ET offset
of ±1 microsecond survives in the two-part `JulianDate`, but collapsing
it into the reader's single-`f64` JD rounds back to JD 2451545.0, so the
reader returns the *same* state as at the exact J2000 instant. This is
expected with the existing reader; the test makes it difficult to
accidentally claim microsecond resolution at the **astronomical output**.

A non-TDB split date is explicitly rejected by the legacy epoch converter,
and an ET offset outside the fixture coverage remains an explicit
`OutsideCoverage` error. No new legacy reader API or duplicated
precision-budget guard is introduced: work on the opt-in checked bridge
is already underway in PR #136 / #139.

This test proves **interface compatibility and observed binary64
quantization**, not precision of a real JPL kernel, matching Horizons,
or production-ready planetary results. Verification is provided by the
existing Rust CI checks in the PR; no external downloads are required.

Change record: 2026-11-13, added one CPAL-1.0-noticed integration test
without modifying the original code's licensing or attribution.


## Opt-in ET-to-split precision audit (2026-11-14)

The same Rust `time_split` module now provides
`audited_split_julian_day_tdb_from_et(et, max_roundtrip_error_seconds)`.
This does **not** perform any additional time-scale conversion or
kernel calculation. It evaluates the split date and reconstructs the
provided **binary64 ET seconds** using the integer-day offset from
J2000 and the separately retained midnight fraction. The result
includes the split date and its observed binary64 roundtrip residual.

```rust
use virgostars_ephemeris_core::time_split::audited_split_julian_day_tdb_from_et;
use virgostars_ephemeris_core::TdbSecondsPastJ2000;

let et = TdbSecondsPastJ2000::new(1.0e-12)?;
assert!(audited_split_julian_day_tdb_from_et(et, 1.0e-13).is_err());
let allowed = audited_split_julian_day_tdb_from_et(et, 1.0e-11)?;
assert_eq!(allowed.roundtrip_error_seconds, 1.0e-12);
# Ok::<(), Box<dyn std::error::Error>>(())
```

A one-picosecond offset near J2000 is representable in the ET input but
**not** in the `f64` Julian midnight fraction, which rounds back to noon.
A strict 0.1-ps residual budget rejects that offset; a relaxed 10-ps
budget accepts it with its loss reported. The audit also rejects NaN,
infinite and negative budgets; any invalid or out-of-range split epoch
remains an explicit error. Tests cover exactly represented whole/half-day
offsets, one-microsecond offsets, negative near-boundary inputs and
unsupported magnitudes.

**Scope of the number:** the residual is an observed comparison between
binary64 values, **not** a mathematically certified absolute error bound
for arbitrarily large timestamps. It cannot measure detail already lost
before the `TdbSecondsPastJ2000` input was constructed, or precision
lost later when collapsing the split date to the classic DE441 reader's
single-`f64` JD. Callers who require a *subsequent* split→DE441 budget
must apply the separate checked-epoch bridge of PR #136 after that PR
has been integrated with the appropriate dependency chain.

No SPK handling, external datasets, astrophysical correction or
independent NASA/JPL scientific comparison is added. CPAL-1.0 source
notice and visible **Powered by VirgoStars** attribution remain applicable.
The change was added to the draft #171 branch on 2026-11-14.
