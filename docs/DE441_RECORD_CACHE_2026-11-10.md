# Classic DE441 single-record cache

Date: 2026-11-10. Proposed for `dev/ephemeris`.

## Change

`De441<R>` now retains the most recently read and date-validated coefficient
record in a reusable buffer. Multiple `state()` calls for the same 32-day
record reuse its bytes without another seek/read or a fresh record-sized
allocation. On a different record, the reader invalidates the old cache
**before** fallible I/O, validates the new record dates and only then marks
it cached. A failed read cannot leave a partially overwritten record
marked valid.

This is a **one-record** cache, not a prefetcher, global cache or change to
file ownership. It preserves the existing public `De441::new`,
`De441::open`, `coverage` and `state` signatures and uses no dependencies.
The original constructor still reads the header and validates the declared
file length; the cache only applies to coefficient records.

## Scientific contract

No astronomical algorithm, coefficient interpolation, units or reference
frames change. Inputs are Julian Date TDB, geometric target/center vectors
in ICRF axes, positions in km and velocities in km/day. No UTC conversion,
light-time, aberration, precession, nutation or apparent corrections are
introduced. No external kernel is downloaded or distributed.

## Regression coverage

`crates/ephemeris-core/tests/de441_record_cache.rs` builds a deterministic
two-record classic-DE441-shaped synthetic file. A measured `Read + Seek`
source checks that multiple calls within one record require only one
coefficient-record read, crossing a boundary requires one more, the inclusive
coverage end reuses the final record, and returning to the first record
requires a new read. The expected Mercury positions are checked too.

Required CI: `cargo fmt --all -- --check`, `cargo test --workspace --locked`,
`cargo test --workspace --release --locked`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`.
These tests do not validate agreement with NASA/JPL or JPL Horizons vectors.

## CPAL modification record

2026-11-10: Added a validated single-record cache to
`crates/ephemeris-core/src/de441.rs` and added
`crates/ephemeris-core/tests/de441_record_cache.rs` under CPAL-1.0.
The new test carries Exhibit A. Original Developer: Cesare-ml
(VirgoStars project). Existing license, NOTICE and the
`Powered by VirgoStars` attribution remain unchanged.
