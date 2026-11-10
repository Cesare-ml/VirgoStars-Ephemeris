# Versioned UTC leap-second schedules — reviewed offline loading

**Date:** 2026-11-10  
**Status:** experimental Rust API on PR #40, not a production time service.

## Objective

Allow applications to **keep the original IANA tzdb-2026b schedule** while
loading a separately reviewed, versioned future leap-second table into
the same process. Historical results should be repeatable by using the
**same version and the same content**. The Rust core must not silently
replace older tables, choose a hidden 'latest' version, download data, or
extend the validity interval of any snapshot.

## Implemented Rust API

`VersionedUtcLeapSchedule` in
`crates/ephemeris-core/src/utc_leaps.rs` owns:

- a literal **version ID**, such as `iana-tzdb-2026b`;
- a declared **provenance string** (link and/or approved digest / dossier ID);
- the exclusive **UTC expiration day**;
- an independent, validated `UtcLeapSchedule` with bounded UTC↔TAI
  conversion methods.

The existing `UtcLeapSchedule::from_iana_2026b()` remains unchanged for
existing callers. `VersionedUtcLeapSchedule::iana_2026b()` wraps that same
pinned schedule with metadata, while
`VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(version,
provenance, source)` accepts a previously approved **curated numeric**
NTP-style table from memory. The wrapper offers getters `version()`,
`provenance()`, `expires_utc_day()` and `schedule()`.

For example, callers can keep an old schedule and a new reviewed version
side-by-side:

```rust
use virgostars_ephemeris_core::utc_leaps::{
    UtcDay, VersionedUtcLeapSchedule,
};

let original = VersionedUtcLeapSchedule::iana_2026b()?;
let original_expiry = UtcDay::from_gregorian(2026, 12, 28)?;
assert_eq!(original.expires_utc_day(), original_expiry);

// Only after externally verifying the official source, its SHA-1 integrity,
// complete history, identity, retrieval and published expiration:
// let reviewed_numeric = std::fs::read_to_string("reviewed-iana-2027a.list")?;
// let updated = VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
//     "iana-tzdb-2027a",
//     "source URL; retrieval timestamp; separately verified SHA-256",
//     &reviewed_numeric,
// )?;
// original continues to exist unchanged in this scope.
# Ok::<(), Box<dyn std::error::Error>>(())
```

The example's future version string is **illustrative**, not a claim that
an IANA release named tzdb-2027a or any hypothetical 2027 leap exists.

## Accepted format and guarantees

The in-memory source must contain:

- **exactly one** `#@ NTP_EXPIRY` line, with an integer NTP timestamp
  divisible by 86,400 (a UTC midnight), strictly after its last record;
- increasing numeric `NTP_MIDNIGHT TAI_MINUS_UTC` rows, beginning with
  `2272060800 10` (1972-01-01 00:00 UTC baseline), no duplicate epochs;
- transitions at month starts with an **exact +1 or −1-second** change;
- no more than 1,024 numeric entries and 128 KiB of input;
- a nonempty, safe version ID of at most 80 ASCII characters and a
  nonempty provenance value of at most 1,024 bytes.

Blank lines and comment lines are accepted. The format is intentionally
the *curated numeric proposal* emitted by
`scripts/audit_iana_leap_seconds.py --propose VERSION`, **not** a generic
parser for arbitrary official IANA files. The Rust loader checks the
numeric and calendar structure, the offset jumps, the expiry and day
range; it does **not** authenticate the asserted version/provenance,
verify the source `#h` checksum or compare with an approved history.
Those checks must be completed by the **offline audit script, source
review and a human-approved PR** before loading new data into a real app.

A proposal with a later expiration is **not an automatically approved
upgrade**, even when leap offsets are unchanged. The software never
extrapolates beyond the stated cutoff. Existing pinned 2026b results
remain available unchanged after importing a candidate as a separate
object.

## Tests and compatibility

Four additional deterministic Rust tests cover:

1. Metadata and historical offsets for the versioned 2026b wrapper
   match the existing unchanged `from_iana_2026b` factory.
2. A **synthetic** later expiration leaves 2026b's 2026-12-28
   cutoff intact; the separate synthetic version is valid until its
   separately specified 2027-07-01 cutoff.
3. An explicitly **hypothetical** 2026-12-31 leap second can be
   represented by a separate future version without changing the
   historical 2026b table.
4. Missing/duplicate/malformed expiry, invalid baseline, epoch
   misalignment, invalid steps/chronology and unsafe provenance are
   rejected.

These tests verify the **versioned loading mechanism**. They do not
predict future leap seconds, validate future IERS announcements,
implement TT↔TDB or establish actual NASA/JPL DE441 accuracy.

## Workflow and licensing

First run the independent source audit and review protocol in
[UTC_LEAP_TABLE_UPDATE_WORKFLOW_2026-11-10.md](UTC_LEAP_TABLE_UPDATE_WORKFLOW_2026-11-10.md).
Record the upstream version, published source URL, SHA-256 of the
downloaded file, original official SHA-1 field, retrieval date, and
relevant IERS bulletin in the approving PR. Keep each approved numeric
snapshot under a **new versioned filename**, rather than overwriting
the existing tzdb-2026b file. Only then opt in to loading the new version.

2026-11-10 modification: added `VersionedUtcLeapSchedule`,
`from_reviewed_numeric_snapshot`, `iana_2026b`, new explicit errors,
and four deterministic tests to the original Rust module, retaining
its **CPAL-1.0 Exhibit A** source notice. The separately
attributed **public-domain IANA numeric data is unchanged**.
`LICENSE`, `NOTICE` and required **Powered by VirgoStars**
(https://www.virgostars.it/) attribution remain unchanged. No new
dependencies, privileged CI, network calls, merge or production changes.

Required checks on the final PR head:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
```
