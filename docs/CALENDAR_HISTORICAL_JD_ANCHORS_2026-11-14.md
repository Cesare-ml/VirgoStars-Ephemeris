# Historical calendar JD anchors (Rust regression gate)

Created 2026-11-14 as a focused, additive test suite for the existing Rust
`calendar` module. The canonical core is Rust, and its calendar arithmetic
supports caller-selected **proleptic** Gregorian or Julian calendars. Neither
calendar automatically switches at October 1582; the caller chooses the
chronology.

## Fixed, independently known anchors

| Instant (calendar, astronomical year numbering) | Expected conventional JD |
| --- | ---: |
| 2000-01-01 12:00 Gregorian (J2000 reference) | 2451545.0 |
| 1582-10-15 00:00 Gregorian | 2299160.5 |
| 1582-10-05 00:00 Julian | 2299160.5 |
| -4712-01-01 12:00 Julian (4713 BCE) | 0.0 |
| -4713-11-24 12:00 Gregorian (4714 BCE) | 0.0 |

Regressions also check that Gregorian 1582-10-04 is still a valid proleptic
date (no hidden skipped days), that Julian 1900-02-29 is legal while Gregorian
1900-02-29 and 2100-02-29 are not, and that year 0 is a leap year in
both stated proleptic calendars. They exercise round-trips between the public
calendar and split-JD APIs, including JD zero.

## Reproduce

```sh
cargo test --locked -p virgostars-ephemeris-core --test calendar_historical_jd_anchors
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

These tests require **no** JPL kernel, network access or new dependency.
They are pure arithmetic/oracle checks. A successful run does **not** validate
NASA/JPL/NAIF ephemeris positions or velocities, leap-second models, actual
UTC->TDB conversion, Delta T, observer frames, or the proposed Swiss
compatibility layer. The `TimeScale` field is only a declared label; the
DE441 epoch bridge intentionally rejects labels other than TDB.

The test fixture uses externally established Julian-day calendar definitions,
not the existing VirgoStars JavaScript implementation as its correctness
oracle. Do not re-label it as an independent celestial-vector benchmark.
The applicable project terms and visible attribution remain in
[LICENSE](../LICENSE) and [NOTICE](../NOTICE).

**Verification status (at authorship):** test source added to GitHub through
the repository connector; the initiating environment has no `cargo` or
`rustc`. Require actual read-only CI evidence at the pull request head
before treating the checks as passed. No release, merge or production change.
