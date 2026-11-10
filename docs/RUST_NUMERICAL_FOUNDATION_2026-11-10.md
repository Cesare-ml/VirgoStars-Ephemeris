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
