# Classic DE441 sliced-file exporter — experimental

**Status: locally implemented, real-file validation pending.** This exporter is
not an Android pack downloader and does not redistribute any JPL data. It is
the first code step toward [VirgoStars-Ephemeris #599](https://github.com/Cesare-ml/VirgoStars-Ephemeris/issues/599)
and the [VirgoStars Mobile pack profile](https://github.com/Cesare-ml/virgostars-mobile-app/pull/31).

## Local use

Obtain the **classic little-endian DE441** source file separately, subject to
its own distribution rights. This command accepts neither NAIF `.bsp` nor
Astrodienst `.se1` files:

```sh
cargo run --offline --locked -p virgostars-ephemeris-core \
  --example slice_de441_classic -- \
  /data/de441.eph /data/de441-1600-2100.eph 1600 2100
```

The years are **inclusive** and use astronomical numbering (year 0 is 1 BCE)
on a **proleptic Gregorian calendar interpreted directly in TDB**. No UTC or
historical calendar conversion occurs. The requested interval is January 1
of the first year, inclusive, through January 1 of the year after the last,
exclusive. The actual saved coverage expands outward to whole 32-day DE441
coefficient records and is reported as inclusive JD TDB bounds.

The program refuses to replace an existing output file. It writes to a new
temporary file alongside the destination, flushes and syncs it, checks that
the output can be opened by the existing DE441 reader with the claimed exact
coverage and byte length, and then creates the final name without replacing
an existing file. It never modifies the input. An exceptional cleanup error
is reported; a leftover named partial file can be inspected separately.

### Selected mobile ranges

| Package | Inclusive astronomical years | Planned delivery |
| --- | --- | --- |
| Base | 1600–2100 | Mobile installation (after legal and scientific gates) |
| Historical 1–8 | 0–199, 200–399, …, 1400–1599 | Explicit optional downloads |
| Future 1 | 2101–2300 | Explicit optional download |

Example for the future package: `... de441-future.eph 2101 2300`.
The first version preserves all source Chebyshev coefficients, even for
unneeded series. It does **not** compress or quantize them. The old estimate of
43–54 MB for the base package is **not a measured result**; obtain the actual
file size only by running the command on a verified DE441 source.

## Scientific and file contract

- Source: classic JPL DE441, with external input fingerprint and provenance
  recorded by the operator; the generated file is not covered by the source
  code's CPAL license merely because the exporter is.
- Time: boundaries are JD TDB, not civil UTC. Physical record selection uses
  half-open requests, while the existing reader includes the stored final
  JD endpoint. Adjacent packs may share the state at the boundary; consumers
  must apply a deterministic selection policy.
- Data: the exporter adjusts the header start/end and preserves the original
  constants record and selected Chebyshev coefficient records byte-for-byte.
  Geometric ICRF position/velocity are computed later by the existing reader
  (km, km/day). No apparent/topocentric transformations are added here.
- Error handling: invalid, non-finite, inverted or uncovered date ranges are
  rejected **before output is written**; read/write failure is explicit. A
  caller using the library API directly must treat failed writes as partial.
- Testing: synthetic multi-record tests check exact coefficients, edge dates,
  parent/slice state identity, range rejection and output I/O faults. These
  tests are **not** independent numerical verification against JPL/NAIF.

## Outstanding acceptance gates

1. Run on a legally sourced **real 2.6 GB-class DE441** file and record source
   SHA-256, source coverage, physical slice sizes, end-to-end seek latency and
   original-vs-slice state vectors at a broad sampling of epochs and bodies.
2. Validate source **and slice** against independently generated NAIF/JPL
   reference values, including positions, velocities and every pack boundary.
3. Implement a signed/versioned manifest, hashes, online optional download,
   atomic cache management, rollback, and mobile pack-selection UI separately.
4. Complete a dataset redistribution-rights and provenance review before any
   public hosting, bundling, mirror or release of generated data.
5. Execute Cargo formatting, tests and strict Clippy plus Android ARM64 tests
   in an environment with Rust toolchains; a docs-only/synthetic check does
   not certify scientific accuracy.

## Modification record, 2026-11-18

Original new library exporter method, offline CLI and deterministic synthetic
tests for VirgoStars-Ephemeris (Initial Developer: Cesare-ml / VirgoStars
project). No third-party kernels or code incorporated; no license or notice
changes. This file documents the change under CPAL-1.0 §3.3; the project's
existing `LICENSE` and `NOTICE` control.
