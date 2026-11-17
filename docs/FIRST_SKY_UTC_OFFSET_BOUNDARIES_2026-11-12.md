# First-sky UTC offset and leap-table boundary regressions (2026-11-12)

## Scope and regression contract

The experimental `first_sky` path accepts a civil timestamp with an explicit fixed UTC offset, then normalizes it to UTC **before** selecting the applicable TAI−UTC value from its dated IERS leap-second table.

The tests in `crates/ephemeris-core/src/first_sky.rs` guard these invariants:

1. `1972-01-01T14:00:00+14:00` identifies the same UTC instant as `1972-01-01T00:00:00Z` and uses **TAI−UTC = 10 s**, at the first covered UTC midnight.
2. `2027-06-28T13:59:59+14:00` is still `2027-06-27T23:59:59Z`, on the **last covered UTC day**.
3. `1972-01-01T00:00:00+01:00` precedes the supported interval in UTC, while `2027-06-27T10:00:00-14:00` exceeds it. Both must fail with `SkyError::UnsupportedUtc`.
4. `1971-12-31T23:59:59-01:00` is inside the supported interval once normalized to UTC and must not be rejected solely because of its *local* date.
5. Across the 2016/2017 positive leap-second boundary, the local timestamp `2017-01-01T00:30:00+01:00` still uses **36 s** while `2016-12-31T23:30:00-01:00` already uses **37 s**.

The assertions compare UTC day indices, split-day fractions or the resulting TT Julian dates for genuinely equivalent instants. They require no downloaded kernels, network, extra Rust dependencies, or application data.

## Scientific limitations

- Calendar arithmetic is proleptic Gregorian and uses astronomical date components as encoded by the existing module.
- Input is explicitly offset-tagged UTC civil time; UTC leap-second labels such as `23:59:60` are **not supported**. No leap-second-label conversion is exercised.
- The table's IERS Bulletin C 72 coverage expires before **2027-06-28 UTC**; the regression protects rejection outside that range, not an automatic extension of the table. Any extension requires authoritative updated source data and review.
- This checks **time normalization and TAI−UTC selection only**. It does not verify TDB−TT modelling, UT1, Earth orientation, body positions, differential velocities or independent JPL/Horizons astronomical parity.
- `first_sky` retains its documented approximate geometric model and does not replace the production Swiss Ephemeris integration.

## Verification

On an environment with the Rust toolchain, run:

```bash
cargo fmt --all -- --check
cargo test --workspace first_sky::tests::
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The first-sky unit tests require no `DE441_PATH` file. The existing external-DE441 integration test remains intentionally ignored unless separately configured.

Source code remains governed by the repository's **CPAL-1.0** license, `NOTICE` and Exhibit B attribution (“Powered by VirgoStars”, https://www.virgostars.it/). No third-party software or ephemeris payloads are added.
