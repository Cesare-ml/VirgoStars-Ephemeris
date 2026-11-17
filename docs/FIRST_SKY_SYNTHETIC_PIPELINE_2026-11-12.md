# First-sky synthetic end-to-end regression (2026-11-12)

## Goal

Exercise the entire existing **experimental** `first_sky::calculate` path under standard native CI, with a tiny, deliberately fictitious in-memory classic-DE441 binary. The current external-file test remains an ignored opt-in integration test and must not be required by read-only CI.

The new `crates/ephemeris-core/tests/first_sky_synthetic_pipeline.rs` synthesizes a 9,000-byte DE441-style classic binary consisting of two header records and a 32-day coefficient record. It supplies all eleven series needed to resolve the ten returned bodies, including the geocentric Moon and Earth–Moon barycenter. Every coordinate is synthetic, constant and expressed in km. No real JPL data or software is redistributed.

## Checked contracts

- The production path returns the ten bodies in the declared order, with explicit **system barycenters** for outer planets and finite geometric topocentric coordinates.
- Timestamps describing the same instant as `Z` and `+01:00` produce matching TT/TDB epochs and resulting directions/distances.
- Changing the observing site changes the nearby Moon's topocentric geometric projection.
- A missing Sun series, a TDB epoch outside the caller's file interval, or invalid observer latitude causes an explicit error rather than a partially populated sky result.
- Existing approximation labels remain discoverable and accurately describe the absence of light-time, aberration, nutation, refraction and other effects.

## Scientific contract and limitations

Input: fixed-offset civil time normalized to UTC, with the existing IERS TAI−UTC policy, and WGS84 observer geodetic coordinates (degrees/metres). Kernel epochs are **TDB Julian days**. Cartesian geometric data are interpreted as ICRF, km. Returned mean-date ecliptic/equatorial positions are degrees, distances are km, and horizon directions use the documented approximate UT1=UTC, precession and Earth orientation models.

**Synthetic regression tests are software integration checks only.** They cannot establish physical solar-system accuracy, apparent geocentric ecliptic-of-date parity, NAIF/SPICE/JPL compatibility beyond the already implemented classic binary reader, or real-kernel precision. The orbital vectors intentionally do not describe real planets, and the one-record mock does not test multi-record JPL ephemeris behavior. Separate independent numerical validation is mandatory before backend substitution.

## Reproducibility and licensing

Run from the repository root:

```sh
cargo fmt --all -- --check
cargo test --workspace --test first_sky_synthetic_pipeline
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The fixture is locally synthesized in the test and requires no `DE441_PATH`, network access, additional crates, privileged workflows, or kernel downloads.

Source code remains CPAL-1.0 and retains Exhibit A attribution notices. Preserve the applicable “Powered by VirgoStars” credit and link to https://www.virgostars.it/ as specified by the repository license/NOTICE. No external files, algorithms or third-party assets were copied.
