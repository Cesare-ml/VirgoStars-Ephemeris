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

## Machine-readable failure contract v1

The Rust module also exposes `UTC_TAI_ERROR_V1_SCHEMA`
(`"virgostars.utc-tai-error.v1"`) and
`UtcTaiErrorV1 { schema, code }`. Convert a failed
`UtcLeapError` with `UtcTaiErrorV1::from(error)` before passing
an error to a future thin Python or TypeScript binding.

The error contract is **implemented in Rust**; no language binding,
JSON serialization, or stable production API is claimed. These codes
are lowercase ASCII snake_case, designed to remain independent of
Rust `Display` wording or localization. The enum match is exhaustive:
adding a new low-level `UtcLeapError` variant requires an explicit
mapping decision, rather than silently inventing a code.

| Machine code | Meaning |
| --- | --- |
| `invalid_gregorian_date` | Gregorian UTC date outside valid calendar fields |
| `before_1972` | Unsupported pre-1972 UTC regime |
| `invalid_coverage` | Malformed declared schedule interval |
| `invalid_initial_offset` | Invalid initial TAI−UTC offset |
| `invalid_transition` | Invalid transition date, coverage, or ordering |
| `invalid_offset_step` | Offset jump not equal to +1/−1 second |
| `outside_coverage` | Conversion attempted beyond approved table coverage |
| `invalid_clock_time` | Invalid structured UTC clock components |
| `clock_time_outside_day` | Syntactically valid clock time absent on the named UTC day |
| `unrepresentable_tai` | Split TAI JD cannot represent the instant |
| `wrong_input_scale` | Input JD is not tagged TAI |
| `invalid_pinned_leap_data` | Bundled, version-pinned snapshot failed validation |
| `invalid_reviewed_snapshot` | Caller-supplied curated numeric snapshot invalid |
| `invalid_snapshot_provenance` | Missing or invalid declared version/source metadata |
| `duplicate_version` | Attempt to register an existing version ID |
| `unknown_version` | Requested table version not registered |
| `source_digest_mismatch` | Exact-byte source SHA-256 disagrees with expected value |

```rust
use virgostars_ephemeris_core::utc_leaps::{
    UtcClockTime, UtcDay, UtcLeapCatalog, VersionedUtcLeapSchedule,
};
use virgostars_ephemeris_core::utc_wire::{
    UtcTaiErrorV1, UTC_TAI_ERROR_V1_SCHEMA,
};

let mut catalog = UtcLeapCatalog::new();
catalog.register(VersionedUtcLeapSchedule::iana_2026b()?)?;
let day = UtcDay::from_gregorian(2017, 1, 1)?;
let clock = UtcClockTime::new(0, 0, 0.0)?;
let failure = catalog
    .utc_to_tai_record_v1("iana-tzdb-2026b", "wrong-digest", day, clock)
    .unwrap_err();
let record = UtcTaiErrorV1::from(failure);
assert_eq!(record.schema, UTC_TAI_ERROR_V1_SCHEMA);
assert_eq!(record.code, "source_digest_mismatch");
# Ok::<(), Box<dyn std::error::Error>>(())
```

Unit tests enumerate all 17 currently defined Rust error variants,
assert one-to-one code mappings and stable schema identifiers, and
exercise real catalog failure paths for digest mismatches, unknown
versions, and invalid positive leap-second clock readings. Future
language bindings must carry the code explicitly; they must not inspect
human-readable Rust error text or convert failures to invented defaults.

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

### Negative leap seconds: explicit cross-language boundary

The v1 scalar record is now also exercised with an **entirely
hypothetical, test-only -1 second UTC transition at 2027-01-01**.
No negative leap second is asserted as announced by IERS or published
in an official IANA source.

For that synthetic table, 2026-12-31 has only **86,399 seconds**:
`23:59:58.5 UTC` maps to `2027-01-01 00:00:35.5 TAI`;
the next valid UTC clock reading `2027-01-01 00:00:00 UTC`
maps to `00:00:36 TAI`. The civil time `23:59:59 UTC` on
the shortened day **does not exist** and must return the v1 code
`clock_time_outside_day`.

Two deterministic Rust integration-style unit tests verify those
scalar record values and both inverse conversions, preservation of
version/provenance/hash/expiry, rejection of the omitted UTC second,
rejection of an old pinned SHA-256 for the synthetic new table, and
non-extrapolation of the historical 2026b schedule. Future bindings
must not infer that every UTC day has 86,400 seconds or convert UTC
to a uniform JD by multiplying a day fraction by 86,400.

These are software-boundary checks against a **synthetic** negative
leap transition, not a substitute for verifying any future published
IERS announcement or an independent astronomical JPL/Horizons gate.

### Deterministic acceptance gates

Six record tests and two error-code tests in `crates/ephemeris-core/src/utc_wire.rs` check:
1. Independent date anchors, the 2016 positive leap, and agreement
   of record metadata in UTC→TAI and TAI→UTC directions.
2. Record fields remain owned after dropping the catalog.
3. Wrong source digest, unknown version, invalid leap clock, and
   non-TAI reverse input fail before producing any record.
4. The first excluded day of IANA 2026b (2026-12-28) is rejected.

Two more tests cover the complete machine-readable error codebook and
actual fail-closed conversion failures. This confirms scalar **software consistency** only. The tests do not
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

2026-11-10 follow-up: introduced the additive, exhaustive
`UtcTaiErrorV1` mapping and `UTC_TAI_ERROR_V1_SCHEMA` constant,
with two deterministic Rust tests. The error codes are part of the
experimental Rust-only contract and do not imply deployed PyO3/WASM
bindings. CPAL-1.0 notices, IANA public-domain data provenance and
`Powered by VirgoStars` attribution remain unchanged.

2026-11-10: added two negative-leap scalar-record regression tests
for a synthetic -1s transition on 2027-01-01, including both
conversion directions, rejected missing 23:59:59 second, source-hash
mismatch, and unchanged historical expiry of IANA 2026b. No actual
future leap event is asserted; no production binding, ABI, or JSON
serializer is implied.
