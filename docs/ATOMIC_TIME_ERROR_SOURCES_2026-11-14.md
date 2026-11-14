# Preserve calendar error causes in TAI ↔ TT operations

**Modification:** 2026-11-14. Rust source covered by the repository's CPAL-1.0; see [LICENSE](../LICENSE), [NOTICE](../NOTICE) and [LICENSING_AND_CREDITS.md](LICENSING_AND_CREDITS.md).

## Scope

The existing `AtomicTimeError` enum distinguishes a wrong input time scale from invalid or unrepresentable split Julian dates. `InvalidJulianDate(CalendarError)` has always contained the original error type, but its empty `std::error::Error` implementation did not forward it through `source()`.

This change returns:

- `AtomicTimeError::InvalidJulianDate(error)` → `Some(&error)`, preserving the typed `CalendarError`.
- `AtomicTimeError::WrongInputScale { .. }` → `None`, because there is no underlying error.

Error variants, public API signatures, `Display` output, units, arithmetic, 32.184-second TAI/TT offset, midnight rollover handling, time-scale restrictions and numerical results are unchanged. The change is additive to standard error-chain diagnostics used by Rust applications and future bindings. It complements (but does not depend on) [first-sky error chaining PR #188](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/188).

## Deterministic regression tests

1. Call the real `tai_to_tt` conversion with a deliberately out-of-range split Julian date. Verify the resulting `AtomicTimeError::InvalidJulianDate(CalendarError::YearOutOfRange)` exposes its inner calendar error using `Error::source()`, and the calendar error has no further source.
2. Call the real `tt_to_tai` conversion with a `TimeScale::Tdb` input. Verify the existing scale mismatch and `None` for `source()`.

These tests require no external DE441 file, API credentials, downloaded data or additional dependencies.

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Use the exact-head GitHub Actions run for the final verification result. This is error-reporting hardening, **not** independent JPL/Horizons validation, not a leap-second model and not a TDB/TT conversion.
