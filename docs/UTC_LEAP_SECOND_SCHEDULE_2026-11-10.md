# UTC leap-second day schedules — explicit, bounded model

**Date:** 2026-11-10  
**State:** experimental Rust module; bidirectional UTC↔TAI conversion and
an opt-in, version-pinned, historically complete IANA 2026b snapshot are
implemented. A current-after-expiration leap table, UTC text parsing,
TT→TDB and independent JPL scientific parity remain unimplemented.

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
- `UtcLeapSchedule::from_iana_2026b()`: load the **historically complete,
  publicly available 1972–2017 transition series** from a bundled, curated
  public-domain IANA tzdb-2026b snapshot. Its validity is strictly
  1972-01-01 through **2026-12-27 inclusive**, since the upstream NTP
  expiration is midnight on 2026-12-28.
- `tai_minus_utc_at_day_start(day)`: the integer offset for a covered UTC day.
- `utc_day_length_seconds(day)`: the duration of a named UTC civil day in
  SI seconds, 86,399 / 86,400 / 86,401 according to a transition at the
  following midnight.
- `UtcClockTime::new(hour, minute, second)`: validates the clock fields
  (`second` is a finite `f64`, in `[0, 61)`). A `60.x` second is
  *syntactically* permitted only in the final minute, at `23:59`.
- `UtcLeapSchedule::seconds_into_utc_day(day, clock)`: checks that the
  clock time **actually exists** according to the approved schedule, and
  returns its elapsed SI seconds from the named UTC midnight, not a JD.
- `UtcLeapSchedule::utc_to_tai(day, clock)`: converts a validated civil
  reading into a split `JulianDate` explicitly tagged `TimeScale::Tai`.
- `UtcLeapSchedule::tai_to_utc(tai)`: converts an **already TAI-tagged**
  split Julian date to `(UtcDay, UtcClockTime)`, including `23:59:60.x`
  when appropriate; it refuses instants outside the bounded schedule.

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

The module relies on the existing Gregorian calendar day-number arithmetic.
The internal TAI label used to calculate a Gregorian UTC **day number** is
only an implementation convenience: that helper does not convert a UTC
instant. The actual conversion requires `utc_to_tai`, which validates a
named UTC clock reading and applies the verified schedule offset.

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

## UTC→TAI conversion — explicit schedule and continuous seconds

The method `UtcLeapSchedule::utc_to_tai(day, clock)` uses the **offset at
the start of the UTC civil day**, not the offset at the following midnight.
This distinction is essential during a positive leap second. It first
validates the clock against the day length and covered dates, then computes:

```text
elapsed UTC SI seconds since named midnight = clock seconds, including :60
TAI seconds since the same Gregorian midnight =
    elapsed UTC SI seconds + (TAI−UTC offset at start of UTC day)
TAI day carry = floor(TAI seconds since that midnight / 86400)
TAI fraction = residual seconds / 86400
```

The **result** is a split `calendar::JulianDate` with
`TimeScale::Tai`, normalized to 86,400 SI seconds/day as appropriate for
the atomic time scale. No UTC quasi-JD is constructed. The method returns
`UtcLeapError::UnrepresentableTai` rather than an invalid or overflowing
split date. Any timestamp outside the supplied schedule is rejected.

A positive leap-second example using the *deliberately minimal* schedule
from above:

```rust
use virgostars_ephemeris_core::calendar::TimeScale;
use virgostars_ephemeris_core::utc_leaps::{
    UtcClockTime, UtcDay, UtcLeapSchedule, UtcLeapTransition,
};
let last = UtcDay::from_gregorian(2016, 12, 31)?;
let next = UtcDay::from_gregorian(2017, 1, 1)?;
let schedule = UtcLeapSchedule::new(
    last, next, 36,
    vec![UtcLeapTransition {
        effective_utc_day: next, tai_minus_utc_seconds: 37,
    }],
)?;
let leap = schedule.utc_to_tai(last, UtcClockTime::new(23, 59, 60.0)?)?;
let following = schedule.utc_to_tai(next, UtcClockTime::new(0, 0, 0.0)?)?;
assert_eq!(leap.scale(), TimeScale::Tai);
assert_eq!(leap.day_number(), next.julian_day_number());
assert!((leap.midnight_fraction() * 86_400.0 - 36.0).abs() < 1e-8);
assert!((following.midnight_fraction() * 86_400.0 - 37.0).abs() < 1e-8);
# Ok::<(), Box<dyn std::error::Error>>(())
```

This verifies that the following clock readings are **three separate TAI
instants**, one SI second apart:

| UTC clock | Equivalent TAI clock on 2017-01-01 |
| --- | --- |
| 2016-12-31 23:59:59 | 00:00:35 |
| 2016-12-31 23:59:60 | 00:00:36 |
| 2017-01-01 00:00:00 | 00:00:37 |

An explicitly **hypothetical negative leap** likewise maps the existing
23:59:58 and the following 00:00:00 to consecutive TAI seconds, and rejects
the nonexistent UTC 23:59:59. These are arithmetic tests, not claims that
negative leap seconds have occurred.

The added deterministic tests cover those two boundary cases, regular
midday/fractional seconds, composition with `tai_to_tt`, and errors for
unknown coverage and invalid day-specific clock readings. The decimal
`f64` second and split-JD fractional day impose finite precision;
sub-nanosecond exactness and full historical UTC calibration are **not**
claimed.

The original test fixture (the 2016–2017 36→37-second transition) is
consistent with [IERS Bulletin C 72](https://datacenter.iers.org/data/html/bulletinc-072.html)
and [NIST's leap-second record](https://www.nist.gov/pml/time-and-frequency-division/time-realization/leap-seconds).
Production callers must import and review a complete authoritative
transition history with a version and a known **coverage/announcement
horizon**; nothing here retrieves or updates that data automatically.

## TAI→UTC inverse conversion and day-boundary search

The inverse method `UtcLeapSchedule::tai_to_utc(tai)` accepts only a
`calendar::JulianDate` tagged **TAI**, and returns the matching
`(UtcDay, UtcClockTime)`. To avoid the precision loss from collapsing a
large Julian day into a single `f64`, it compares normalized two-part TAI
dates and uses a binary search over **TAI instants of UTC midnights**.
The method builds these midnight instants from the supplied TAI−UTC offset
for each UTC day, including the exclusive boundary *after* the last covered
day. It then subtracts the selected midnight's TAI instant to obtain elapsed
SI seconds in that UTC day. The conversion is logarithmic in the number of
covered days.

A positive leap day's duration is 86,401 seconds: TAI clock
`2017-01-01 00:00:36` becomes UTC
`2016-12-31 23:59:60`, while TAI `00:00:37` becomes
UTC `2017-01-01 00:00:00`. A hypothetical negative leap day has
86,399 seconds and produces no nonexistent `23:59:59` reading.

```rust
use virgostars_ephemeris_core::utc_leaps::{
    UtcClockTime, UtcDay, UtcLeapSchedule, UtcLeapTransition,
};
let last = UtcDay::from_gregorian(2016, 12, 31)?;
let next = UtcDay::from_gregorian(2017, 1, 1)?;
let schedule = UtcLeapSchedule::new(
    last, next, 36,
    vec![UtcLeapTransition {
        effective_utc_day: next,
        tai_minus_utc_seconds: 37,
    }],
)?;
let tai = schedule.utc_to_tai(last, UtcClockTime::new(23, 59, 60.5)?)?;
let (utc_day, utc_clock) = schedule.tai_to_utc(tai)?;
assert_eq!(utc_day, last);
assert_eq!(utc_clock.hour(), 23);
assert_eq!(utc_clock.minute(), 59);
assert!((utc_clock.second() - 60.5).abs() < 1e-8);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The conversion rejects **non-TAI** inputs
(`UtcLeapError::WrongInputScale`), missing or exceeded schedule coverage
(`OutsideCoverage`) and unrepresentable split dates
(`UnrepresentableTai`). Neither direction downloads a leap-second table
or invents past/future corrections. Precision is limited by the existing
`f64` fractional days, especially for arbitrary sub-nanosecond timestamps.
The original UTC→TAI implementation and the inverse should be tested with
the **same** official versioned schedule in operational deployments.

Four deterministic tests exercise 2016 positive-leap and hypothetical
negative-leap round-trips, exact start/end coverage rejection, wrong-scale
input, and an unusual two-day TAI−UTC offset to ensure the algorithm does
not assume the two day numbers differ by at most one. These tests do not
establish DE441/Horizons parity or source authenticity.

## Version-pinned public-domain historical snapshot (IANA tzdb-2026b)

This implementation optionally includes the **complete published leap
history starting 1972-01-01**, as captured from the public-domain
[IANA tzdb-2026b leap-seconds.list](https://data.iana.org/time-zones/tzdb-2026b/leap-seconds.list).
The upstream file explicitly labels itself **public domain**. It includes
28 NTP-stamped offset entries: one initial 1972 baseline (TAI−UTC = 10 s)
and 27 positive leap-second transitions, ending at 2017-01-01
(TAI−UTC = 37 s). We have included only those numeric NTP/offset pairs and
the expiry marker, with VirgoStars-authored provenance comments rather
than copying the upstream textual explanation. Source attribution is
maintained separately from the Rust source license CPAL-1.0.

The upstream `#@` field is **4007404800 NTP seconds**, representing the
file's expiration at **2026-12-28 00:00 UTC**. The Rust factory enforces the
validity interval from **1972-01-01 00:00 UTC** to the instant **strictly
before 2026-12-28 00:00 UTC**, and rejects the 28 December UTC date or
later. Although [IERS Bulletin C 72](https://datacenter.iers.org/data/html/bulletinc-072.html)
confirms no new leap second at the end of December 2026, the *pinned*
2026b data should not be extrapolated past its own stated expiry.
To extend the interval, update and review the source and tests in a
separate versioned change; do not merely alter a numeric expiry constant.

```rust
use virgostars_ephemeris_core::utc_leaps::{
    UtcClockTime, UtcDay, UtcLeapSchedule,
};

let schedule = UtcLeapSchedule::from_iana_2026b()?;
let last_leap = UtcDay::from_gregorian(2016, 12, 31)?;
let tai = schedule.utc_to_tai(last_leap, UtcClockTime::new(23, 59, 60.25)?)?;
let (back_day, back_time) = schedule.tai_to_utc(tai)?;
assert_eq!(back_day, last_leap);
assert!((back_time.second() - 60.25).abs() < 1e-8);
assert!(schedule.tai_minus_utc_at_day_start(
    UtcDay::from_gregorian(2026, 12, 28)?
).is_err());
# Ok::<(), Box<dyn std::error::Error>>(())
```

`from_iana_2026b()` rejects malformed snapshot metadata, duplicate or
unsorted transitions, incorrect initial offset, wrong date boundaries,
unexpected record count and non-unit changes through the existing
`UtcLeapSchedule::new` validator. The numeric snapshot is checked against
the source's declared 28 rows, 1972 baseline, 2017 final offset and
expiration, with historical regression tests. This is intentionally
*version-specific*, not a general-purpose untrusted NTP-file importer;
an arbitrary externally downloaded file must not be passed to this API.

The data snapshot is public domain according to its IANA file notice and
not relicensed as CPAL; the Rust parser and test code remain under the
project CPAL-1.0 license. For provenance, see the versioned file's
source URL and last-update/expiration metadata.

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

There is **no complete UTC string parser**, embedded official IERS table,
leap-second extrapolation or mapping to TDB, TT or UT1 inside this module.
`UtcClockTime` is a structured civil reading, not a JD or a parsed
timezone-bearing timestamp. The bidirectional UTC↔TAI conversion is
**conditional on a properly reviewed, bounded leap schedule**. It does
not by itself make the result suitable for DE441, which requires TDB.
The separate Rust `tai_to_tt` function can convert TAI to TT under
the conventional 32.184-second offset, but TT→TDB requires a distinct,
documented physical model.

A compact **historical IANA snapshot** derived from an official
public-domain `leap-seconds.list` file is included in
`crates/ephemeris-core/data/iana-tzdb-2026b-leap-seconds.list`.
It is *not* an IERS-certified new publication, not a byte-for-byte copy of
the original file and not a promise of updates. The explicit IANA expiration
is enforced; **beyond 2026-12-27 the pinned schedule fails**, rather than
assuming that 37 seconds remains correct. The Gregorian day-number type
remains separate from `calendar::JulianDate` because UTC cannot be
assumed to be a uniform Julian-day scale over leap days. No runtime network
access, external dependency or automatic update is introduced.

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

2026-11-10: implemented `UtcLeapSchedule::utc_to_tai` and
`UtcLeapError::UnrepresentableTai` in `crates/ephemeris-core/src/utc_leaps.rs`,
with four deterministic tests for positive and hypothetical negative leap
boundaries, fractional civil time, TAI→TT composition and failure modes.
Original CPAL-1.0 source notices and mandatory VirgoStars credits remain
unchanged. Neither third-party code nor a complete IERS table was bundled.

2026-11-10: added `UtcLeapSchedule::tai_to_utc`, strict
`UtcLeapError::WrongInputScale` rejection, and four deterministic
round-trip/boundary tests to `crates/ephemeris-core/src/utc_leaps.rs`.
The inverse preserves positive and hypothetical negative leap seconds,
does not require an embedded external transition table or new dependency,
and retains CPAL-1.0 Exhibit A / VirgoStars credits. No merge or deploy.

2026-11-10: added the public-domain IANA tzdb-2026b numeric
leap-second snapshot and `UtcLeapSchedule::from_iana_2026b()`
with explicit expiration on 2026-12-28 UTC, historical coverage
1972-01-01 through 2026-12-27, a version-specific metadata/offset gate
and three deterministic regression tests. Upstream source:
https://data.iana.org/time-zones/tzdb-2026b/leap-seconds.list.
The public-domain data remains separately attributed, CPAL-1.0 Rust
source notices and VirgoStars Exhibit B attribution are unchanged.
