# Calendar and Julian date arithmetic — 2026-11-10

The `calendar` module converts calendar date/time fields to and from a split
Julian date, for a caller-selected **proleptic Gregorian** or **proleptic
Julian** calendar and a caller-specified TAI, TT, TDB or UT1 time scale.
Years use astronomical numbering (0 = 1 BCE) and range from -20000 to 20000,
including the local DE441 binary's supported calendar span. There is no
automatic historical cutover: callers decide which calendar to use. The
same instant can be converted into either calendar without changing scale.

```rust
use virgostars_ephemeris_core::calendar::{Calendar, CalendarDateTime, TimeScale};

let date = CalendarDateTime {
    year: 2000, month: 1, day: 1, hour: 12, minute: 0, second: 0.0,
    scale: TimeScale::Tdb,
};
let jd = date.to_julian_date(Calendar::ProlepticGregorian)?;
let (first, second) = jd.parts()?; // sum = 2451545.0 TDB
assert_eq!(first + second, 2451545.0);
assert_eq!(jd.to_calendar(Calendar::ProlepticGregorian)?, date);
let epoch = jd.to_de441_epoch()?; // input is already TDB
# Ok::<(), Box<dyn std::error::Error>>(())
```

`JulianDate` stores an integer day number plus a fraction since midnight.
`parts()` returns two components whose sum is the conventional noon-based JD;
keep them separate for precision. `from_f64` supports existing single-number
JD inputs but cannot restore precision that has already been lost, and
returns an error when its midnight-based split cannot represent the instant.
`to_de441_epoch()` accepts only an already-TDB date and combines the parts
into the reader's single `f64`, losing sub-millisecond precision near modern
dates. It does not convert a date from another time scale.
Invalid dates, invalid times, non-finite fractions and unsupported ranges are
explicit errors. Fractions and seconds are `f64`, not arbitrary precision.

This is calendar arithmetic, **not a conversion between time scales**. A day
is represented as 86400 coordinate seconds. UTC, time zones, leap-second
labels (including second 60), ΔT, TT↔TDB and Earth-orientation corrections
are deliberately not implemented here. Do not label a UTC clock reading as
TDB; converting an actual birth time to a DE441 epoch requires those missing
stages and their data policies.

Tests include the USNO Julian-date origin and Gregorian reform examples, the
ERFA `t_cal2jd` numerical reference, leap-year rules, BCE and boundary years,
fractional seconds, input errors, and round trips across both calendars.
Sources: [USNO Julian date converter](https://aa.usno.navy.mil/data/JulianDate),
[USNO Julian date formula](https://aa.usno.navy.mil/faq/JD_formula), and the
[ERFA conformance suite](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
ERFA reference-data attribution and terms are in
[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

Modification date: 2026-11-10. Original VirgoStars code remains under CPAL-1.0;
no ephemeris data or runtime dependency is added.
