# Split Julian-date to TDB ephemeris seconds

**2026-11-10 — experimental Rust core; proposed for `dev/ephemeris`.**

## Scope and scientific contract

`calendar::JulianDate::to_tdb_seconds_past_j2000()` converts a **date
already expressed in TDB** from its two-part representation
`(integer day number at noon, fractional day since midnight)` to
`TdbSecondsPastJ2000` (`f64` seconds measured from JD 2451545.0 TDB).
No civil-time or time-scale conversion is performed.

The calculation keeps the integral JD offset separate from the fractional
day until both have been multiplied by exactly 86,400 seconds/day:

```text
ET = (day_number - 0.5 - 2451545.0) * 86400.0
   + midnight_fraction * 86400.0
```

This avoids first combining both Julian-date parts into a large single
floating-point number. Around modern dates, a single `f64` Julian date
resolves only tens of microseconds; an already split date retains more of
its sub-second information when converted directly to ET seconds. The
returned `f64` ET still has finite precision, and it degrades with larger
offsets from J2000. No precision beyond what was supplied in the split date
can be recovered.

The existing `to_de441_epoch()` is unchanged and **still constructs a
single `f64` Julian date**, because the experimental classic DE441 reader
currently accepts that API. This new conversion is intended for future
SPK/ET consumers and must **not** be misrepresented as fixing the precision
of `De441::state`.

## Usage

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};

let epoch = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb)?;
let et = epoch.to_tdb_seconds_past_j2000()?;
assert_eq!(et.value(), 0.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The method returns `CalendarError::WrongTimeScale` for TAI, TT or UT1;
it **never** assumes `UTC = TDB` or silently converts scales. It rejects
dates outside the split-JD exact integer range and maps any invalid ET
construction to `CalendarError::UnrepresentableTime`.

## Deterministic tests

Four unit tests in `crates/ephemeris-core/src/calendar.rs` exercise:

1. J2000 noon = zero ET, and the following midnight = +43,200 seconds.
2. Quarter-second offset from J2000 retains better than nanosecond
   precision in the split path, whereas a premature single-f64-JD roundtrip
   loses microseconds (specific synthetic arithmetic example).
3. A modern epoch with fractional day, and an epoch before J2000.
4. Explicit rejection of non-TDB scales and out-of-range integer day parts.

These are **unit-level mathematical tests**. They do not verify the
conversion from actual UTC/TT observations to TDB, the accuracy of any
TDB–TT model, JPL SPK kernel parsing or real DE441/Horizons agreement.

## Provenance, licensing and modification record

2026-11-10: extended `crates/ephemeris-core/src/calendar.rs` with one
typed convenience conversion and four unit tests, based solely on original
arithmetic and the existing Rust representations. No code or data from
Swiss Ephemeris, NAIF SPICE Toolkit or JPL files is used or bundled.
The original CPAL-1.0 Exhibit A notice is preserved in the modified file.
`LICENSE`, `NOTICE` and the project's attribution **Powered by VirgoStars**
(https://www.virgostars.it/) remain unchanged. No new dependencies,
network calls, deploys, binaries or production changes were introduced.

Required review gates on the PR head:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The repository's CI provides additional native Windows/macOS checks.
Only checked outputs should be reported as successful.
