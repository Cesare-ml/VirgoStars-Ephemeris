# First-sky error causes: preserve typed Rust source chains

**Modification date:** 2026-11-14  
**Initial developer:** Cesare-ml (VirgoStars project). Original covered source retains the CPAL-1.0 license and attribution; see [LICENSE](../LICENSE), [NOTICE](../NOTICE), and [LICENSING_AND_CREDITS.md](LICENSING_AND_CREDITS.md).

## Scope

The experimental `first_sky::SkyError` already wraps `CalendarError`, `AtomicTimeError` and `De441Error`, but its empty implementation of `std::error::Error` did **not** forward these typed causes via `source()`. Consequently, callers using standard Rust error-chain diagnostics could see the human-readable top-level message yet not inspect the underlying error types or an I/O failure's root cause.

The new `SkyError::source()` preserves exactly the wrapped errors:

| `SkyError` variant | `source()` |
|---|---|
| `Calendar(CalendarError)` | Original `CalendarError` |
| `AtomicTime(AtomicTimeError)` | Original `AtomicTimeError` |
| `Ephemeris(De441Error)` | Original `De441Error`, which in turn exposes an underlying `std::io::Error` for `De441Error::Io` |
| `InvalidInput(_)`, `UnsupportedUtc` | `None` (no wrapped cause) |

This is backward compatible: same public variants, `Display` text, calculation APIs, error-conversion implementations, epochs, units, astronomical algorithms and result values. Callers gain standard structured diagnostics without parsing text.

## Regression coverage

Tests in the existing `first_sky` Rust module verify:

- Real invalid civil date parsing (`2026-02-30T12:00:00Z`) returns `SkyError` with an inspectable `CalendarError::InvalidDate` source.
- A wrapped `AtomicTimeError::WrongInputScale` retains the expected and actual time-scale types.
- `SkyError::Ephemeris(De441Error::Io(...))` supports two `source()` levels, ending in `std::io::ErrorKind::UnexpectedEof`.
- Variants without wrapped errors correctly return `None`.

No external ephemeris kernels, internet, personal data, additional dependencies or production systems are used.

## Verification

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo test --workspace first_sky::tests::invalid_civil_date_exposes_calendar_source
cargo test --workspace first_sky::tests::sky_errors_preserve_typed_time_and_nested_de441_io_sources
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Record the exact-head GitHub Actions CI result before merging. These are **error-propagation tests**, not independent NASA/JPL or Horizons numerical validation. The current first-sky calculator still has the scientific limitations described in [FIRST_PLANET_POSITIONS_2026-11-10.md](FIRST_PLANET_POSITIONS_2026-11-10.md).
