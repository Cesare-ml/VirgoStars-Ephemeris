# First-sky UTC leap-second history regression — 2026-11-14

**Status:** deterministic historical time-handling tests for the *existing*
experimental Rust `first_sky` model. Not a new UTC conversion algorithm or
independent ephemeris science certification.

## Purpose and reference

`first_sky::epochs` uses a hard-coded UTC-effective Modified Julian Date
(MJD) versus TAI−UTC table, currently through the 2017-01-01 transition.
Before this change, its test suite checked the 2016/2017 transition and
some UTC offset examples but did not verify the complete historical
sequence against **calendar dates independent of the encoded MJDs**.

Two new Rust tests use a separate, human-readable fixture containing
the **28 effective dates and corresponding offsets, from the
1972-01-01 baseline (TAI−UTC = +10 s) through the 2017-01-01
transition (+37 s)**. The effective-date list and values are corroborated
by the [Hong Kong Observatory's historical leap-second table]
(https://www.hko.gov.hk/en/gts/time/Historicalleapseconds.htm),
and the leap-insertion dates are listed by the
[US National Institute of Standards and Technology (NIST)]
(https://www.nist.gov/pml/time-and-frequency-division/time-realization/leap-seconds).

The initial 1972 baseline is **not** itself an inserted leap second.
The other **27 positive steps** are leap insertions. Dates are
**proleptic Gregorian** and converted to MJD using the existing calendar
module before checking each corresponding value in `LEAP_SECONDS`.
A wrong MJD can no longer pass simply because both the implementation
and its test reuse the same numeric table.

## What is tested

1. For every effective civil date, UTC midnight selects the expected
   TAI−UTC offset, and the table's original MJD matches the date's
   independently calculated day number.
2. At each post-baseline positive leap transition, the supported UTC
   labels `23:59:59` immediately before and `00:00:00` immediately after
   correspond to a **two-second** TT separation. The test permits
   0.1 ms due to collapsing the split TT instant into a single `f64`
   Julian date at modern Julian-day magnitudes.
3. At **all 27 transitions**, a local `+14:00` civil timestamp on the new
   local day is still the previous UTC day, whereas a `−12:00` civil
   timestamp on the preceding local day has already reached the new UTC
   day. Both paths must select the **same** TAI−UTC offset and TT epoch
   as the equivalent `Z` timestamp. This catches timezone handling
   accidentally occurring *after* selecting leap-second data.

These checks do **not** accept a literal `23:59:60` timestamp; the
public timestamp parser continues to reject leap-second labels. The
two-second TT separation measures the implied physical interval without
adding support for parsing that omitted label. The `Ut1` type label in
this early experimental code remains a documented approximation to UTC,
not a proof that UT1 actually equals UTC.

## Limits and maintenance

Passing these regressions verifies the encoded historical effective
dates and offset normalization, **not** predictions of future leap
seconds, official UTC service compliance, IERS EOP accuracy, TDB−TT
model fidelity, DE441 interpolation, JPL/NAIF comparisons, or
planetary-position error bounds. The future UTC coverage expiry
2027-06-28 is deliberately unchanged and continues to fail closed
outside the announced table.

Run from the repository root:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The approved read-only GitHub CI additionally runs optimized tests,
Rustdoc and native macOS/Windows Rust test and lint checks. No JPL
kernel downloads or external services are needed for the tests.

## Modification record and license

Modified 2026-11-14: added two historical boundary tests and a
civil-date reference table in `crates/ephemeris-core/src/first_sky.rs`,
plus this document. No production algorithms, public APIs,
dependencies, external ephemeris data, Swiss/NAIF code, deployment
scripts or release artifacts are changed. Existing CPAL-1.0 Exhibit A
notice, [NOTICE](../NOTICE) and required **Powered by VirgoStars**
attribution remain in force. This PR is independent from the
more limited UTC coverage/offset test PR #88; it must be reviewed
separately and does not assume #88 has merged.
