# Rust numerical foundation — 2026-11-10

The experimental crate now includes the mathematical foundation originally
proposed in PR #8 and the analytical contract tests from PR #14, alongside
the existing DE441 reader. The original VirgoStars-Ephemeris source remains
under CPAL-1.0; existing source notices and attribution are retained. No
third-party source, dependencies, or ephemeris files were added.

`evaluate_chebyshev` computes a series and its derivative with respect to the
normalized coordinate by extended Clenshaw recurrence. `ChebyshevPositionRecord`
represents a synthetic, SPK Type-2-style Cartesian record and returns position
in km and velocity in **km per TDB second**. Its epoch and interval are TDB
seconds past J2000. The separate classic DE441 reader accepts JD TDB and returns
velocity in **km per day**. These units are explicit in their respective APIs;
the two records are not interchangeable. `JulianDayTdb` offers conversion to
TDB seconds but no civil time or UTC conversion. A single `f64` JD loses time
precision; a two-part representation remains future work.

On Windows with Rust 1.89, `cargo fmt --all -- --check`,
`cargo test --workspace --locked`, `cargo clippy --workspace --all-targets
--locked -- -D warnings`, the synthetic example, and the ignored external-file
test with the local classic DE441 binary passed. Mathematical identities and
real-file regression are not independent JPL/Horizons accuracy validation.
The synthetic record is **not an SPK parser**; SPK binary reading, reference
comparisons, apparent corrections, bindings and production use remain open.

## TDB ET seconds → Julian Day conversion (2026-11-13)

The typed Rust `TdbSecondsPastJ2000::to_julian_day_tdb()` method
converts an ET offset in **TDB seconds relative to J2000** to a
single-`f64` `JulianDayTdb`. The existing
`JulianDayTdb::to_tdb_seconds_past_j2000()` method handles the opposite
direction. Both are **representation conversions within TDB**, not
UTC/TAI/TT/TDB time-scale conversions; neither introduces leap-second
handling, Earth orientation parameters or SPK support.

```rust
use virgostars_ephemeris_core::TdbSecondsPastJ2000;

let et = TdbSecondsPastJ2000::new(43_200.0)?;
let jd = et.to_julian_day_tdb()?;
assert_eq!(jd.value(), 2_451_545.5);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The single-`f64` Julian day has an increment of roughly **40 microseconds
near J2000**. Thus, converting a 1-microsecond ET offset to this representation
rounds to J2000; do not use it for sub-millisecond-precision time stepping.
A two-part JD API is the intended solution and remains separate work.
Synthetic public-API tests exercise exact full/half days, ±1-second
round trips and explicitly lost microsecond offsets. They are **not**
an independent DE441/JPL accuracy test.

Verification commands from the repository root:
`cargo fmt --all -- --check`, `cargo test --workspace --locked`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`.
No NASA kernel, new dependency or third-party source is introduced.
This 2026-11-13 change preserves the CPAL-1.0 Exhibit A notices and the
required Powered by VirgoStars attribution.
