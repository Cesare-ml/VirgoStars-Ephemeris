# Classic DE441: record selection near exact TDB boundaries

**Modification date:** 2026-11-14. Original Rust source under CPAL-1.0.

## Problem

A binary64 epoch just below an exact classic coefficient-record boundary can
be rounded *onto* that boundary when the reader first subtracts a distant
file-start Julian date and divides by the 32-day record step. For instance, a
synthetic file covering JD TDB -32 to +32 sees the smallest representable
negative JD round to 0 during `jd - start`. A floor-based index then returns
the following record, even though the original requested epoch is negative.

## Change and contract

`De441::state` now selects the coefficient record by binary-searching the
header-defined chronological record boundaries and comparing the **original
f64 TDB Julian date** with each boundary. An exact interior boundary chooses
the next record; the final file endpoint remains inclusive in the final
record. The method requires no new allocation, kernel read, dependency,
external service or API. Lookup arithmetic is O(log record count) instead of
the prior O(1) approximate index; record I/O and cached reuse are unchanged.

The test-only synthetic regression verifies the nearest negative and
positive representable TDB JDs around zero, the next representable epochs
around J2000, and an archival-scale negative date boundary. Consecutive
records contain deliberately different constant ICRF Mercury-relative-to-SSB
positions, in km; velocities remain zero km/day. Cache transitions are
checked in both directions.

This is a **binary64 record-selection fix**, not a general guarantee of
arbitrary time precision. Coefficient granule selection, date-tag checks and
independent real JPL/NAIF kernel parity are separate concerns. There are no
UTC/TT/UT1 conversions, observer transformations, light-time, aberration or
apparent corrections.

## Validation

On the exact feature-branch head, run:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --test de441_record_boundary_rounding
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The synthetic tests are not independent NASA/JPL/Horizons astronomical
reference validation. Keep production, releases and package publishing
unchanged pending review.
