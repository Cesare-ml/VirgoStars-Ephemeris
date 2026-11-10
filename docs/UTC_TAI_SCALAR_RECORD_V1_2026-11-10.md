# UTC ↔ TAI scalar record v1 (experimental cross-language contract)

**Date:** 2026-11-10  
**Status:** implemented as a Rust struct and methods in draft PR #40.
**Not yet implemented:** JSON serializer, PyO3 or wasm-bindgen bindings,
TypeScript package, shared dynamic library ABI, production release.

## Purpose

For the same UTC/TAI instant, this Rust record retains **both**
representations and every piece of leap-table metadata relevant to
reproducing the conversion. A later Python or TypeScript binding should
map these fields directly, **without** porting any time arithmetic.

The authoritative source of the calculations remains the Rust core.

## Available Rust API

The public module `virgostars_ephemeris_core::utc_wire` exposes:

- `UTC_TAI_RECORD_V1_SCHEMA` = `"virgostars.utc-tai.v1"`.
- `UtcTaiRecordV1`: an **owned**, scalar record with explicit field names.
- `UtcLeapCatalog::utc_to_tai_record_v1(version, expected_sha256, day, clock)`.
- `UtcLeapCatalog::tai_to_utc_record_v1(version, expected_sha256, tai)`.

Both factory methods require an **exact version** and an
**independently trusted, exact-byte SHA-256** matching the selected
curated leap-second table. They refuse unknown versions, digest mismatch,
non-TAI input, nonexistent UTC clock readings, and timestamps outside
the table's explicitly declared coverage.

Both directions produce the **same logical fields**:

| Rust field | Intended cross-language type | Meaning |
| --- | --- | --- |
| `schema` | string | Literal schema ID, `virgostars.utc-tai.v1` |
| `leap_table_version` | string | Selected version ID, no implicit default |
| `leap_table_sha256` | string | Lowercase 64-character SHA-256 of exact curated source bytes |
| `leap_table_provenance` | string | Declared source, not cryptographic authentication |
| `leap_expires_utc_day_number` | integer | Exclusive UTC coverage endpoint, Gregorian noon-based day number |
| `utc_day_number` | integer | Named Gregorian UTC civil date as noon-based Julian day number |
| `utc_hour` | integer | Civil hour, 0–23 |
| `utc_minute` | integer | Civil minute, 0–59 |
| `utc_second` | number | May equal `60.x` **only** on a validated positive leap-second date |
| `tai_day_number` | integer | Normalized noon-based Julian day number on TAI |
| `tai_midnight_fraction` | number | Fraction of a uniform 86,400 SI-second TAI day since preceding midnight |
| `tai_time_scale` | string | Literal `"TAI"`, never UTC/TT/TDB |

The day numbers use the same integer convention as the existing
`calendar::JulianDate`:
`JD = day_number - 0.5 + midnight_fraction` **for the uniform TAI
scale only**. UTC is kept in named-day/hour/minute/second fields;
**never** interpret `utc_day_number` as a uniform UTC Julian date.
No target, center, reference frame, kernel or corrections are attached
because this is a **time conversion record**, not a planetary state.

The intended JSON/TypeScript/Python field names are specified here as
an **experimental design target**. No serializer has been published.
Source fields are currently public Rust fields, not an independently
guaranteed stable binary ABI.

## Rust example

```rust
use virgostars_ephemeris_core::utc_leaps::{
    UtcClockTime, UtcDay, UtcLeapCatalog, VersionedUtcLeapSchedule,
};
use virgostars_ephemeris_core::utc_wire::UTC_TAI_RECORD_V1_SCHEMA;

let mut catalog = UtcLeapCatalog::new();
catalog.register(VersionedUtcLeapSchedule::iana_2026b()?)?;
let version = "iana-tzdb-2026b";
let expected_sha256 =
    "e6120e71c65ff1969a4f63c285d6b99df8b02d9bdb90331669384d63534c41dd";

let utc_day = UtcDay::from_gregorian(2016, 12, 31)?;
let clock = UtcClockTime::new(23, 59, 60.5)?;
let result =
    catalog.utc_to_tai_record_v1(version, expected_sha256, utc_day, clock)?;

assert_eq!(result.schema, UTC_TAI_RECORD_V1_SCHEMA);
assert_eq!(result.leap_table_sha256, expected_sha256);
assert_eq!(result.utc_second, 60.5);
assert_eq!(result.tai_day_number, 2_457_755);
assert_eq!(result.tai_time_scale, "TAI");
assert!((result.tai_midnight_fraction * 86_400.0 - 36.5).abs() < 1e-8);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The independent calendar anchor for this example is 2017-01-01,
noon-based integer Julian day number **2457755**. The UTC leap reading
2016-12-31 23:59:60.5 corresponds to
2017-01-01 00:00:36.5 TAI, because the 2016 leap occurred while
TAI−UTC was 36 seconds and the next UTC midnight uses 37 seconds.

### Precision and cross-language conversion

- Rust `i64` day numbers should map to Python `int`. In JavaScript,
  the supported 1972–2026 dates are safely representable as `number`
  (absolute values below `Number.MAX_SAFE_INTEGER`); future general
  ranges must explicitly handle 64-bit integer conversion.
- Rust `u8` fields map to bounded integer scalars.
- Rust `f64` maps to IEEE-754 binary64. Never round or truncate the
  fractional day to fewer decimal digits without declaring the policy.
- Rust error variants must map into **explicit, documented error codes**,
  not silent null values or UTC=TAI defaults.
- Do not use JavaScript `Date` or Python naive `datetime` to carry
  the `23:59:60` clock value; many libraries cannot represent it.

### Deterministic acceptance gates

Four Rust tests in `crates/ephemeris-core/src/utc_wire.rs` check:
1. Independent date anchors, the 2016 positive leap, and agreement
   of record metadata in UTC→TAI and TAI→UTC directions.
2. Record fields remain owned after dropping the catalog.
3. Wrong source digest, unknown version, invalid leap clock, and
   non-TAI reverse input fail before producing any record.
4. The first excluded day of IANA 2026b (2026-12-28) is rejected.

This confirms scalar **software consistency** only. The tests do not
validate an astronomical TT↔TDB model, NAIF/JPL vectors, leap-source
authenticity, or real production bindings.

Run `cargo fmt --all -- --check`,
`cargo test --workspace --locked`,
`cargo test --workspace --release --locked`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`
and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`
on the exact feature head. The existing Linux/Windows/macOS CI is
configured to execute these gates; record only actual observed outcomes.

## License and modification record

2026-11-10: added `crates/ephemeris-core/src/utc_wire.rs` and exported
`pub mod utc_wire` from `src/lib.rs`. The new original Rust source
retains **CPAL-1.0 Exhibit A**. The pinned public-domain IANA
tzdb-2026b numerical source is unchanged. Required
**Powered by VirgoStars** (https://www.virgostars.it/) attribution
and `LICENSE` / `NOTICE` remain unchanged. No new dependencies,
network calls, merge, deploy or production modifications.
