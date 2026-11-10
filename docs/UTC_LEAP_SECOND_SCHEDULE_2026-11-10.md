# UTC leap-second day schedules — explicit, bounded model

**Date:** 2026-11-10  
**State:** experimental Rust module; no UTC→TAI conversion or independent JPL scientific parity is claimed.

## Why this exists

Unlike TAI and TT, **UTC civil days are not always exactly 86,400 SI seconds**:
a positive leap-second day has 86,401 seconds (ending 23:59:60), and a
hypothetical negative leap-second day would have 86,399 seconds. Consequently
UTC civil time must not be silently reinterpreted as a uniform-scale Julian
date, nor passed to the DE441 reader as TDB.

References:

- [IERS Bulletin C 72, 6 July 2026](https://datacenter.iers.org/data/html/bulletinc-072.html):
  TAI−UTC = 37 s since 2017-01-01; no leap second at the end of December 2026.
- [NIST leap-second table and explanation](https://www.nist.gov/pml/time-and-frequency-division/time-realization/leap-seconds):
  history and 2016-12-31 positive leap second.
- [NIST leap-second FAQ](https://www.nist.gov/pml/time-and-frequency-division/leap-seconds-faqs):
  positive and negative leap-second clock sequences.

These are *external scientific references*, not datasets or executable code
redistributed under CPAL-1.0.

## Implemented API

`crates/ephemeris-core/src/utc_leaps.rs` exposes:

- `UtcDay::from_gregorian(year, month, day)`: a strongly typed Gregorian UTC
  day from 1972 onward, backed internally by the calendar module's integer
  Julian day number (it does **not** carry a TAI/TT/TDB time-scale label).
- `UtcLeapTransition { effective_utc_day, tai_minus_utc_seconds }`: the offset
  starting at **00:00 UTC** on the new day, after any leap second at the
  immediately preceding day's end.
- `UtcLeapSchedule::new(first_day, last_day, initial_offset, transitions)`:
  owned, explicitly bounded schedule of caller-supplied transitions.
- `tai_minus_utc_at_day_start(day)`: the integer offset for a covered UTC day.
- `utc_day_length_seconds(day)`: the duration of a named UTC civil day in
  SI seconds, 86,399 / 86,400 / 86,401 according to a transition at the
  following midnight.
- `UtcClockTime::new(hour, minute, second)`: validates the clock fields
  (`second` is a finite `f64`, in `[0, 61)`). A `60.x` second is
  *syntactically* permitted only in the final minute, at `23:59`.
- `UtcLeapSchedule::seconds_into_utc_day(day, clock)`: checks that the
  clock time **actually exists** according to the approved schedule, and
  returns its elapsed SI seconds from the named UTC midnight. It does **not**
  construct a Julian date or an instant in TAI, TT or TDB.

Example using a **minimal historical test schedule**, **not** an exhaustive
IERS table:

```rust
use virgostars_ephemeris_core::utc_leaps::{
    UtcDay, UtcLeapSchedule, UtcLeapTransition,
};

let d = |y, m, day| UtcDay::from_gregorian(y, m, day).unwrap();
let schedule = UtcLeapSchedule::new(
    d(2016, 12, 30),
    d(2017, 1, 2),
    36,
    vec![UtcLeapTransition {
        effective_utc_day: d(2017, 1, 1),
        tai_minus_utc_seconds: 37,
    }],
)?;
assert_eq!(schedule.utc_day_length_seconds(d(2016, 12, 31))?, 86_401);
assert_eq!(schedule.tai_minus_utc_at_day_start(d(2017, 1, 1))?, 37);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The module relies exclusively on the existing Gregorian calendar day-number
arithmetic. An internally used TAI tag is just a convenience for invoking
the calendar's **scale-independent date arithmetic**; no UTC↔TAI conversion
is performed by it.

## UTC clock seconds and positive/negative leap-day validation

The clock component is intentionally checked in **two stages**: lexical
field validity (`UtcClockTime::new`) and date-specific validity
(`schedule.seconds_into_utc_day`). Only `23:59:60` (and fractional
`60.x` seconds) is a possible **positive** leap second, and it is accepted
only if that named UTC day is 86,401 SI seconds long. On a normal day
`23:59:60` is an error.

A **hypothetical negative** leap day of 86,399 seconds ends after
`23:59:58.999...` and does not contain `23:59:59`; this final
second is rejected by the same test against the day length.

```rust
use virgostars_ephemeris_core::utc_leaps::{
    UtcClockTime, UtcDay, UtcLeapSchedule, UtcLeapTransition,
};
let day = UtcDay::from_gregorian(2016, 12, 31)?;
let next = UtcDay::from_gregorian(2017, 1, 1)?;
let table = UtcLeapSchedule::new(
    day,
    next,
    36,
    vec![UtcLeapTransition {
        effective_utc_day: next,
        tai_minus_utc_seconds: 37,
    }],
)?;
let leap = UtcClockTime::new(23, 59, 60.5)?;
assert_eq!(table.seconds_into_utc_day(day, leap)?, 86_400.5);
assert!(table.seconds_into_utc_day(next, leap).is_err());
# Ok::<(), Box<dyn std::error::Error>>(())
```

The numeric return is the number of *elapsed SI seconds inside a civil
day*, **not** a uniform-scale Julian-date fraction and **not** an
astronomical time-scale conversion. The `f64` representation cannot
resolve arbitrary sub-nanosecond fractions near midnight; values rounding
to the next day are conservatively rejected, not wrapped or extrapolated.

Four additional deterministic tests cover the positive 2016-12-31 leap
with fractions, a hypothetical negative leap, invalid/non-finite clock
fields, and explicit out-of-coverage failures. All tests are synthetic
except the historically documented 2016 positive leap structure; they
do not establish DE441/Horizons positional parity.

## Validation and failure behavior

The table constructor rejects:

- Coverage with `first_day > last_day`, negative base offsets, transitions
  out of order, duplicates, events outside the declared dates and nonmonth
  transition boundaries.
- Offset jumps other than **exactly +1 or −1 SI second**.
- UTC dates before 1972 (when UTC/TAI frequency offsets require a different
  historical model) and invalid Gregorian dates.

A transition on the **first day after the last covered day** is permitted
so the final covered UTC day can represent a leap second. Every query
outside the explicit interval fails with `UtcLeapError::OutsideCoverage`;
the schedule never extrapolates from a finite announcement. The caller
must obtain and review the complete authoritative transition history and
set an appropriate validity horizon before making operational claims.

The tests include a real **2016-12-31 leap-day structure**, a *hypothetical*
negative leap day, repeated/non-unit/nonmonth transitions, invalid dates
and the final coverage boundary. The hypothetical test demonstrates API
behavior; no negative leap second is asserted to have occurred.

## Deliberately not implemented

There is **no UTC→TAI conversion**, no leap-second interpolation or
mapping to TDB, TT or UT1. `UtcClockTime` is a **validated structured
clock value**, not a full textual date/time parser or instant with a stable
uniform-scale Julian date. The parser/conversion of complete civil timestamps
remains future work. This module cannot make a civil-time timestamp
astronomically ready.
It is a narrow validation building block intended to precede those
capabilities with explicit provenance and tests.

No official IERS transition dataset is embedded or downloaded. The
Gregorian day-number type is separate from `calendar::JulianDate` because
UTC cannot be assumed to be a uniform Julian-day scale over leap days.
The source remains dependency-free and requires no network access.

## Checks and CPAL modification record

Run on the actual PR head:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

2026-11-10: added `crates/ephemeris-core/src/utc_leaps.rs` with an
explicit caller-supplied UTC leap-second schedule, error handling and
deterministic tests; exported the module in
`crates/ephemeris-core/src/lib.rs`. No algorithms or data from Swiss
Ephemeris, NAIF, or any JPL ephemeris file were copied. Source includes
the repository's existing **CPAL-1.0 Exhibit A** notice; `LICENSE`,
`NOTICE`, Exhibit B and the required attribution **Powered by VirgoStars**
(https://www.virgostars.it/) remain unchanged. No package release or merge.

2026-11-10: extended `crates/ephemeris-core/src/utc_leaps.rs`
with `UtcClockTime`, explicit clock-field errors, and a schedule-bound
`seconds_into_utc_day` validation for leap and non-leap UTC days.
The existing CPAL-1.0 source notice and VirgoStars attribution are
unchanged. No external data, leap-second table, or third-party code added.
