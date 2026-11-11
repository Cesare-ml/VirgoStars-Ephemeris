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
- the **entire approved IANA 2026b historical transition prefix**, with
  no deletions, moved dates, changed offsets or unapproved extra
  transitions inside the already approved coverage interval;
- expiration **not earlier than** the historical 2026b exclusive cutoff
  (2026-12-28); future changes remain review-only proposals;
- no more than 1,024 numeric entries and 128 KiB of input;
- a nonempty, safe version ID of at most 80 ASCII characters and a
  nonempty provenance value of at most 1,024 bytes.

Blank lines and descriptive comments are accepted. The format is
intentionally the *curated numeric proposal* emitted by
`scripts/audit_iana_leap_seconds.py --propose VERSION`, **not** a parser
for arbitrary original IANA files. In particular, Rust **rejects** raw
IANA `#$` update and `#h` integrity marker lines instead of ignoring
them as comments. Ordinary descriptive comments mentioning these fields
remain valid. Run the offline verifier against the original source,
then review and load only the curated numeric proposal.

The Rust loader checks the numeric structure, calendar/offset/expiry
rules, immutable 2026b historical prefix and no changes anywhere before
the 2026b expiration. It does **not** authenticate an official publisher,
check the original source's `#h` checksum, or independently establish
that proposed future rows are official. These checks remain the
responsibility of the **offline audit script, independent source review
and a human-approved PR** before adopting any new version.

A proposal with a later expiration is **not an automatically approved
upgrade**, even when leap offsets are unchanged. The software never
extrapolates beyond the stated cutoff. Existing pinned 2026b results
remain available unchanged after importing a candidate as a separate
object.

## Explicit selection and provenance-bearing conversion results

`UtcLeapCatalog` stores **independent named versions**, with no implicit
default and no automatic switch to a newer or unexpired table. Use
`register(versioned_schedule)`, then `utc_to_tai(version, day, clock)`
or `tai_to_utc(version, tai)`; callers **must supply the exact version
ID on each conversion**. `select(version)` provides a read-only reference
to the chosen schedule, while `versions()` lists IDs in stable lexical
order.

Both conversion functions return a `VersionedUtcResult<T>`, with owned
fields `version`, `provenance`, `expires_utc_day` and `value`.
The version and provenance remain attached even after the catalog is
dropped, making it possible to save the **data source used** alongside
a derived timestamp.

```rust
use virgostars_ephemeris_core::calendar::TimeScale;
use virgostars_ephemeris_core::utc_leaps::{
    UtcClockTime, UtcDay, UtcLeapCatalog, VersionedUtcLeapSchedule,
};

let mut catalog = UtcLeapCatalog::new();
catalog.register(VersionedUtcLeapSchedule::iana_2026b()?)?;
let day = UtcDay::from_gregorian(2016, 12, 31)?;
let selected = "iana-tzdb-2026b"; // explicitly chosen by caller
let result = catalog.utc_to_tai(
    selected, day, UtcClockTime::new(23, 59, 60.0)?,
)?;
assert_eq!(result.value.scale(), TimeScale::Tai);
assert_eq!(result.version, selected);
assert_eq!(result.expires_utc_day, UtcDay::from_gregorian(2026, 12, 28)?);

let back = catalog.tai_to_utc(selected, result.value)?;
assert_eq!(back.version, result.version);
assert_eq!(back.value.0, day);
assert!((back.value.1.second() - 60.0).abs() < 1e-8);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The catalog **refuses** duplicate version IDs (`DuplicateVersion`) and
unknown names (`UnknownVersion`) instead of silently replacing or
falling back. When two tables have different coverage intervals, a valid
date for one version can be `OutsideCoverage` for another: **the
historical 2026b object retains its expiration**. The example's version
labels and source strings are declared metadata, not an authenticated
digital signature. For stronger independent reproducibility, save an
externally verified source **content digest** and the actual conversion
inputs with your application record. Merely matching a version string
does not prove that two users loaded identical bytes.

Four additional deterministic tests check selected version metadata on
both conversion directions, lack of implicit fallback, duplicate
registration rejection, coexistence of historical and **synthetic**
future validity, and ownership of result metadata beyond the catalog's
lifetime. These are API and time-label checks, not proof of scientific
ephemeris parity.

## Exact-byte SHA-256 fingerprint for reproducible conversions

The original source/version metadata alone could not distinguish two
`leap-seconds.list` extracts given the **same caller-supplied version
label**. The experimental Rust core now computes a **lowercase SHA-256
digest of the exact bytes** in each curated numeric source:

- `VersionedUtcLeapSchedule::source_sha256()`: the fingerprint attached
  to the loaded table. The pinned IANA tzdb-2026b factory hashes the
  embedded text directly; a reviewed table hashes its input `source`
  text after numeric/calendar validation.
- `VersionedUtcResult<T>::source_sha256`: an **owned copy of the same
  digest** carried alongside each UTC→TAI or TAI→UTC conversion result.
  It persists after the table/catalog object is dropped.
- `UtcLeapCatalog::select_matching_sha256(version, expected)`: an
  optional **fail-closed check** returning `SourceDigestMismatch` if
  the exact selected content does not equal the independently recorded
  expected lowercase digest. An unknown version is still rejected
  without fallback.

The repository's `.gitattributes` **forces LF checkout for this exact
pinned data file**, regardless of the Windows `core.autocrlf` setting.
Without this rule, Windows Git checkout can silently replace LF with CRLF,
changing the exact-byte digest and causing a `SourceDigestMismatch`
even though the numerical leap table is unchanged. A CI regression must
confirm the same digest on Linux, Windows and macOS; neither side
should silently normalize data inside the SHA-256 function.

The currently pinned **989-byte** IANA tzdb-2026b *curated* file has
SHA-256 `e6120e71c65ff1969a4f63c285d6b99df8b02d9bdb90331669384d63534c41dd`,
independently calculated with Python's standard-library `hashlib.sha256`.
A Rust regression asserts this exact digest to detect changes to the
pinned bytes. This is **not** the SHA-256 of the upstream IANA original
file; the curated extract differs in comments and formatting.

This fingerprint covers the complete **curated snapshot bytes**, including
`#@` expiration, numeric rows, whitespace and comments. Two files with
the same numeric transitions but different comments or line endings
therefore have **different** SHA-256 digests. This is intentional: the hash
identifies a concrete artifact, rather than only equivalent historical
offset values. To compare data semantics across different snapshots,
inspect and audit the numeric record history separately.

```rust
use virgostars_ephemeris_core::utc_leaps::{
    UtcClockTime, UtcDay, UtcLeapCatalog, VersionedUtcLeapSchedule,
};

let mut catalog = UtcLeapCatalog::new();
catalog.register(VersionedUtcLeapSchedule::iana_2026b()?)?;
let expected = catalog.select("iana-tzdb-2026b")?
    .source_sha256().to_owned();

// Production callers should instead use an **independently recorded**
// approved hash from their review/manifest, not take it on trust here.
catalog.select_matching_sha256("iana-tzdb-2026b", &expected)?;
let result = catalog.utc_to_tai(
    "iana-tzdb-2026b",
    UtcDay::from_gregorian(2016, 12, 31)?,
    UtcClockTime::new(23, 59, 60.5)?,
)?;
assert_eq!(result.source_sha256, expected);
# Ok::<(), Box<dyn std::error::Error>>(())
```

SHA-256 follows the published FIPS 180-4 algorithm, implemented in the
small independent Rust module `fingerprint.rs` without new dependencies.
Regression tests cover well-known SHA-256 **empty**, **abc**,
**multi-block** and **one-million-'a'** reference vectors, as well as
padding boundaries and input mutation. Additional UTC tests check that
a source-content hash is stable across a round-trip, distinguishes two
different source files carrying the *same version name*, and changes
when only a table's expiry is modified.

**Important trust boundary:** SHA-256 helps detect accidental or
unauthorized byte changes **relative to a separately trusted expected
digest**. Computing a digest of attacker-supplied bytes does **not**
authenticate the publisher, prove that the rows are official or replace
the independent IANA upstream `#h` integrity audit, source-domain
verification, review sign-off or source preservation. The upstream
`#h` is a distinct SHA-1 calculated over the official NTP numeric
tokens, *not* the SHA-256 of the curated VirgoStars snapshot.

## Opt-in conversion with a verified expected SHA-256

For applications that need a reproducible **source-checked** conversion
contract, the catalog now additionally exposes:

- `utc_to_tai_checked(version, expected_sha256, day, clock)`
- `tai_to_utc_checked(version, expected_sha256, tai)`

These methods **require** both an explicit version ID and an expected
lowercase SHA-256 for **each** conversion. They reuse the same conversions
as their nonchecked counterparts but first call
`select_matching_sha256`. Consequently an unknown version fails with
`UnknownVersion`, and a wrong digest fails with
`SourceDigestMismatch` **before** testing time-scale or date coverage.
Successful results contain the selected fingerprint alongside the
existing version, provenance and expiry.

```rust
use virgostars_ephemeris_core::utc_leaps::{
    UtcClockTime, UtcDay, UtcLeapCatalog, VersionedUtcLeapSchedule,
};

let mut catalog = UtcLeapCatalog::new();
catalog.register(VersionedUtcLeapSchedule::iana_2026b()?)?;
let expected = "e6120e71c65ff1969a4f63c285d6b99df8b02d9bdb90331669384d63534c41dd";
let tai = catalog.utc_to_tai_checked(
    "iana-tzdb-2026b",
    expected,
    UtcDay::from_gregorian(2016, 12, 31)?,
    UtcClockTime::new(23, 59, 60.5)?,
)?;
let back = catalog.tai_to_utc_checked(
    "iana-tzdb-2026b", expected, tai.value,
)?;
assert_eq!(tai.source_sha256, expected);
assert_eq!(back.source_sha256, expected);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Both operations remain local, deterministic and free of network access.
They provide a convenient primitive for future **thin Python and
TypeScript/WASM bindings**; those bindings are not implemented by this
PR. If an application stores its results, it should persist the
version, SHA-256, actual input values/time scale, output fields and
reviewed provenance. Time values should not be represented as a
single floating-point Julian Day if the split-date precision matters.

The digest comparison alone is not proof of an authentic IANA publisher
or scientific accuracy, particularly when the expected value has been
copied from the same unverified input. Independent source acquisition,
the offline IANA SHA-1 audit, and human review remain prerequisites for
a genuinely trusted expected digest.

Three additional deterministic Rust tests check a successful
positive-leap-second roundtrip on the pinned SHA-256; rejection of
unknown versions and invalid hashes in **both** directions; and
rejection of a table reusing an old version label with changed bytes.

## Experimental scalar record v1 for future bindings

The first language-neutral Rust record is now implemented in
`crates/ephemeris-core/src/utc_wire.rs`. See
[UTC_TAI_SCALAR_RECORD_V1_2026-11-10.md](UTC_TAI_SCALAR_RECORD_V1_2026-11-10.md)
for the exact scalar fields, explicit version/SHA-256 checking, leap-second
clock semantics, example, numeric/calendar anchors, test gates, and
limits. This does **not** claim that JSON, WASM, Python or TypeScript
bindings are already available.

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

2026-11-10: added `UtcLeapCatalog` and `VersionedUtcResult<T>`
with explicit selection, owned result provenance and expiry metadata,
duplicate/unknown-version errors, four deterministic tests and no new
dependencies. The old `UtcLeapSchedule` and
`VersionedUtcLeapSchedule` signatures remain compatible. CPAL-1.0,
the separately attributed public-domain IANA history, and
`Powered by VirgoStars` attribution are unchanged.

2026-11-10: added SHA-256 content fingerprints to pinned and reviewed
`VersionedUtcLeapSchedule` objects, metadata-bearing
`VersionedUtcResult<T>`, and catalog selection by an explicit expected
SHA-256. New original, dependency-free `fingerprint.rs` and standard
reference-vector tests are covered by the existing CPAL-1.0 source
license; the public-domain IANA numeric snapshot is unmodified.
No network access, automatic acceptance, hidden defaults or merge.

2026-11-10: added explicit per-call digest-checked UTC↔TAI catalog
conversions (`utc_to_tai_checked`, `tai_to_utc_checked`) with three
additional deterministic fail-closed tests. Kept the existing
nonchecked API compatible for experimental callers and left the
pinned IANA data unchanged. No Python or TypeScript runtime binding
is claimed, and the independent JPL scientific validation gate remains
open. Source CPAL-1.0 and required VirgoStars credits unchanged.

2026-11-10 CI portability correction: added a path-specific
`.gitattributes` entry forcing LF checkout for the approved 2026b
public-domain numeric file, preserving the exact-byte SHA-256 across
Windows and Unix runners. The numeric source, expected digest and
time-scale conversion behavior are unchanged.

2026-11-11: the Rust reviewed-numeric importer now rejects raw
unverified IANA `#$` / `#h` markers and preserves the *complete*
approved 2026b historical schedule, including the absence of any
unannounced leap events through the pinned coverage interval. It rejects
earlier expiry dates and historically shifted or omitted transitions,
but still permits review-only future extensions. Deterministic tests
cover those cases. The upstream identity/official checksum review
remains external; no data files were changed.
