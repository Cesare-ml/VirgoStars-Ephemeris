# Decimal-hour Julian-day API — 2026-11-11

Status: **experimental Rust core implementation** for a first-MVP calendar
interface. It is not a complete Swiss Ephemeris, its Node.js binding, or a
UTC/UT1 conversion model.

## Motivation

VirgoStars currently calls `swe_julday(year, month, day,
hour + minute / 60, SE_GREG_CAL)` and `swe_revjul` for dates and planetary
hours. The canonical Rust core already offers validated proleptic Gregorian/
Julian arithmetic with split Julian days, but its `CalendarDateTime` model
takes separate hour/minute/second fields. The new
`calendar_decimal_hours` module provides a narrow public bridge for a
decimal-hour interface without maintaining a second date algorithm.

## API (Rust)

```rust
use virgostars_ephemeris_core::calendar::{Calendar, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{
    DecimalHourDateTime, julday_f64, revjul_f64, to_split_julian_date,
};

let input = DecimalHourDateTime {
    year: 2000, month: 1, day: 1, decimal_hour: 12.0,
    scale: TimeScale::Ut1,
};

let split = to_split_julian_date(input, Calendar::ProlepticGregorian)?;
let jd = julday_f64(input, Calendar::ProlepticGregorian)?;
assert_eq!(jd, 2_451_545.0);
let inverse = revjul_f64(
    jd, Calendar::ProlepticGregorian, TimeScale::Ut1,
)?;
assert_eq!(inverse.year, 2000);
assert_eq!(inverse.decimal_hour, 12.0);
assert_eq!(split.scale(), TimeScale::Ut1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`to_split_julian_date` and `from_split_julian_date` are preferred for
clients that need to preserve fractional precision. The `julday_f64` and
`revjul_f64` conveniences are for single-f64 legacy-style integration;
modern single-f64 Julian dates lose submillisecond detail. All functions
delegate to the original `calendar.rs` core.

Inputs are strict: decimal hour must be finite and in **[0, 24)**, year
must be in the existing supported range -20000..=20000, and dates must
exist in the requested **proleptic** Gregorian/Julian calendar. Unlike
some external implementations, invalid days, overflow hours and 24:00
are errors, not silently normalized. No implicit 1582 reform/cutover.

## Time-scale and compatibility boundary

The caller supplies a `TimeScale` label (TAI, TT, TDB, UT1). This module
does **not** convert a date between scales and intentionally does not
model civil UTC, time zones, leap-second labels, Delta T, EOP or UT1-UTC.
Do not pass wall-clock UTC as UT1/TDB without a separately reviewed
conversion. Passing `TimeScale::Ut1` does not prove the input is UT1.

These functions are **Swiss-shaped arithmetic building blocks**, not
exports of an exact `swisseph@0.5.17` API. Swiss's permissive-input
behavior, JS object fields, sync/callback signatures and any time-scale
policies remain to be verified in a future thin adapter. Full equivalence
for `swe_julday`/`swe_revjul` is not yet claimed.

## Tests and reference scope

`crates/ephemeris-core/tests/calendar_decimal_hours.rs` exercises
documented calendar anchors (including J2000 noon and JD zero),
proleptic 1582 conventions, BCE values, 1900/2100 century behavior,
2024 leap date, negative JD, UT1/TT/TDB labels, microsecond-scale
split fractions and invalid input failures. This is deterministic
calendar-contract coverage, *not* independent validation of ephemeris
positions, NAIF kernels, Swiss Node 0.5.17 semantics or astronomical
time conversions. The separately developed PR #44 maintains a
recorded-reference parity harness: review and coordinate it before
treating this API as accepted compatibility.

CI gate (do not claim passing until verified):

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

No third-party binaries, datasets, dependencies or API tokens are
introduced. Original code and tests retain CPAL-1.0 source attribution.
No production migration, data publication or automatic release occurs.
