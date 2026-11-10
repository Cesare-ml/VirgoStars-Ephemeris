# Offline IANA leap-second update and verification workflow

**Date:** 2026-11-10  
**Status:** contributor-facing maintenance procedure; no automatic leap-table replacement.

## Purpose

The experimental Rust core includes a separately identified **public-domain**
IANA tzdb-2026b numeric leap-second snapshot, valid from 1972-01-01 UTC
through 2026-12-27 UTC inclusive. Unlike long-lived astronomical constants,
future UTC leap announcements and the publication validity horizon **change**.
The pinned snapshot must not be silently extended past its stated expiry.

This repository now provides a **Python standard-library-only** data-maintenance
utility, `scripts/audit_iana_leap_seconds.py`, to help reviewers prepare a
future version, without adding Python astronomy algorithms or runtime
dependencies to the Rust ephemeris library.

Official source for the current pin:

- [IANA tzdb-2026b leap-seconds.list](https://data.iana.org/time-zones/tzdb-2026b/leap-seconds.list)
- The upstream list explicitly states **public domain**, timestamp of last
  update `#$ 3976686858`, expiration `#@ 4007404800`
  (2026-12-28 at 00:00 UTC) and five-word integrity checksum
  `#h 2e101270 4e6749f8 2f1792b7 14a0c188 36bb19d6`.
- The curated repository file is
  `crates/ephemeris-core/data/iana-tzdb-2026b-leap-seconds.list`.
  It contains numerical data and its own attribution, **not** a byte-for-byte
  upstream copy or a SHA-1 signature over the curated file.

## Procedure

1. **Obtain a newly published official file manually** from IANA, or from a
   trusted IERS/NIST distribution whose format and provenance are checked.
   Record the public URL, exact version, retrieval date, original SHA-256
   file digest (e.g. `sha256sum leap-seconds.list` or
   `Get-FileHash -Algorithm SHA256` on Windows) and associated IERS
   Bulletin C where appropriate. The tool never downloads anything.
2. **Run the offline integrity and history audit** against the current pin,
   supplying an explicit UTC review date:

   ```sh
   python3 scripts/audit_iana_leap_seconds.py path/to/leap-seconds.list --as-of 2026-11-10
   ```

   On Windows, `py -3` can replace `python3`. A date is **not inferred**
   from the host clock. The tool checks the upstream `#$` update, `#@`
   expiry and `#h` SHA-1 integrity marker, the 1972 TAI−UTC baseline,
   increasing NTP timestamps, UTC month starts and each ±1-second offset
   transition. It then compares all historical numerical pairs with the
   approved snapshot; existing history must never change silently.
3. **Interpret exit status deliberately**:

   | Exit | Meaning | Required action |
   | --- | --- | --- |
   | `0` | Historical rows and expiry agree with the pinned version | No update needed |
   | `1` | Corrupt/unverifiable source, historical drift, downgrade, invalid date or expired source | Stop, investigate provenance and data |
   | `2` | Valid source differs only by additional transitions and/or a later expiry | Human scientific review required before any replacement |

4. **Preview a candidate**, without writing or changing the repository:

   ```sh
   python3 scripts/audit_iana_leap_seconds.py path/to/leap-seconds.list --propose tzdb-2027a
   ```

   The text is emitted **to standard output**, and the tool returns `2`
   when the new data or expiration differs. Redirect it into a new file
   *only after accepting that this is a review-only proposal*.
   The source checksum covers the original file's numeric/date fields, not
   this generated extract. Treat that checksum as integrity, **not
   cryptographic authentication** or proof that a maliciously substituted
   file is an official publication.
5. **Review and land an atomic new version**: after the external IANA
   source audit, parse the approved numeric extract using
   `VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot` and retain
   the old `VersionedUtcLeapSchedule::iana_2026b()` alongside it in Rust.
   See [UTC_LEAP_VERSIONED_LOADING_2026-11-10.md](UTC_LEAP_VERSIONED_LOADING_2026-11-10.md)
   for the version/provenance/expiry contract and regression tests.
   Add an appropriately named
   *new* numeric snapshot under `crates/ephemeris-core/data/`, document
   third-party public-domain provenance, extend the Rust constructor with
   a new explicitly named pinned-version API, maintain existing old-version
   contracts where feasible, and update the expiry, count, last transition,
   tests, and documentation in the same reviewed PR. Do **not** modify only
   `#@`, silently relabel a version, bypass a historical divergence,
   extrapolate the UTC→TAI conversion or automatically merge/deploy.
6. Run existing Rust gates and the Python offline regression tests:

   ```sh
   python3 -m unittest discover -s scripts -p 'test_audit_iana_leap_seconds.py' -v
   cargo fmt --all -- --check
   cargo test --workspace --locked
   cargo test --workspace --release --locked
   cargo clippy --workspace --all-targets --locked -- -D warnings
   ```

   The existing `.github/workflows/rust-dev.yml` now runs the offline
   maintenance tests on Linux using the platform's Python 3. A separate
   human review of the independent official source and the complete
   regression output remains mandatory before updating production.

## Important integrity and scientific limits

The official `#h` digest is **SHA-1 over the concatenation of decimal
`#$`, `#@` and NTP/TAI−UTC row fields in their file order, with
comments and whitespace omitted**. The five displayed hexadecimal words
may omit leading zeros, so they must be left-padded to eight hex digits
before comparison. See the upstream file's checksum guidance:
[official IERS/Paris Observatory README](https://hpiers.obspm.fr/iers/bul/bulc/ntp/sources/README).

**SHA-1 is not a modern source-authentication mechanism.** Even a matching
digest cannot establish that the upstream URL, publisher, Bulletin C or
retrieved file is authentic; verify those independently before trusting a
new version. The audit intentionally rejects any altered pinned historical
rows even if the new source carries a matching freshly computed checksum.

There is **no network call**, operating-system UTC leap lookup or automatic
renewal. The Python script only handles maintenance and provenance; the
canonical time-scale conversions remain implemented in Rust. No SPK/DE441
kernel data is required or bundled, and the independent NASA/JPL
scientific validation gate remains unresolved.

## Modification and license record

2026-11-10: added `scripts/audit_iana_leap_seconds.py` (offline source
integrity/history audit and review-only candidate output) and
`scripts/test_audit_iana_leap_seconds.py` (deterministic corruption, drift,
expiry, proposed changes and nonmutation tests). Updated
`.github/workflows/rust-dev.yml` to run the tests and to trigger on
changes to these maintenance scripts. No new Python package or dependency
is introduced. All new original script source is covered by the
repository's existing **CPAL-1.0 Exhibit A**; the IANA numeric snapshot
remains separately attributed as public domain. `LICENSE`, `NOTICE`
and required `Powered by VirgoStars` attribution are unchanged.
