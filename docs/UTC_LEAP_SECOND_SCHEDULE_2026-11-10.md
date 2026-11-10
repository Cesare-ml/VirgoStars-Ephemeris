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

There is **no UTC clock time struct**, no parsing of `23:59:60`, no UTC→TAI
conversion, and no leap-second interpolation or mapping to TDB, TT or UT1.
This module cannot make a civil-time timestamp astronomically ready.
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
