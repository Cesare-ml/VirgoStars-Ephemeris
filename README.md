# VirgoStars Ephemeris

**A free and open-source astronomical ephemeris project for astrology applications — licensed under CPAL-1.0.**

**Status: EARLY EXPERIMENTAL IMPLEMENTATION — the Rust core includes a classic JPL DE441 binary reader and synthetic tests, but independent NASA/JPL numerical validation, SPK support, cross-language bindings and production integration remain incomplete.** This is an original implementation, not a fork, translation, wrapper or redistribution of Swiss Ephemeris.


## Purpose

Provide verifiable astronomical quantities to [Astrological Functions](https://github.com/Cesare-ml/astrological-functions) and other clients, using NASA/JPL ephemerides with documented algorithms. The current experimental state reader supports the classic DE441 binary. Rust DAF/SPK **metadata indexing, type-2/3 J2000 direct states and ordered multi-kernel target–center chain composition** are experimental; non-J2000 frames, other SPK types and real-kernel accuracy validation remain open. No login, database, Angular, artwork, interpretation texts, or VirgoStars application data belong here.

**Language decision:** The canonical astronomical core is written in **Rust**, with thin WebAssembly/TypeScript and PyO3/maturin Python bindings planned so all consumers share the same algorithms. The bindings are not implemented. See [ADR-0001](docs/ADR-0001-RUST-CORE-AND-BINDINGS.md).

**Separation of responsibilities:** `virgostars-ephemeris` handles time scales, kernel access, coordinates, physical/geometric corrections and eventually astronomical event primitives. `astrological-functions` handles zodiac/house methods, aspects, synodic phases, astrological events and application-facing chart models. VirgoStars production keeps using its current implementation until tested migration gates pass.

## Read first

- [CPAL-1.0 license, with VirgoStars attribution in Exhibit B](LICENSE)
- [NOTICE of copyright and third-party exclusions](NOTICE)
- [Licensing, required attribution and credits](docs/LICENSING_AND_CREDITS.md)

- [Architecture and boundaries](docs/ARCHITECTURE.md)
- [Engineering roadmap](docs/ROADMAP.md)
- [Astronomical accuracy and validation](docs/VALIDATION.md)
- [NASA/JPL kernels and legal provenance](docs/DATA_AND_LICENSING.md)
- [Proposed public contracts](docs/API_CONTRACTS.md)
- [Contributor and AI-agent rules](AGENTS.md)
- [Italian overview](docs/README.it.md)
- [Experimental DE441 reader](docs/DE441.md)
- [Experimental Rust DAF/SPK metadata index](docs/RUST_SPK_DAF_INDEX_2026-11-12.md)
- [Experimental SPK type-2/type-3 direct-segment states (not JPL-validated)](docs/RUST_SPK_TYPE2_TYPE3_DIRECT_STATE_2026-11-12.md)
- [Experimental single-kernel SPK center-chain states (not JPL-validated)](docs/RUST_SPK_CENTER_CHAINS_2026-11-12.md)
- [Experimental loaded-kernel precedence and cross-file states (not JPL-validated)](docs/RUST_SPK_KERNEL_SET_PRIORITY_2026-11-13.md)
- [Offline NAIF/SPICE SPK validation comparator and evidence requirements (no real results yet)](docs/RUST_SPK_NAIF_OFFLINE_VALIDATION_2026-11-13.md)
- [Independent SPICE reference generator, ordered SHA256 verification and offline Rust comparison](scripts/spk_naif_reference.py)
- [First geometric planet positions for a time and place](docs/FIRST_PLANET_POSITIONS_2026-11-10.md)
- [Experimental geocentric light-time state](docs/GEOCENTRIC_LIGHT_TIME_2026-11-10.md)
- [Functional coverage target](docs/FEATURE_COVERAGE.md)
- [Backend-first Swiss-style compatibility inventory](docs/BACKEND_SWISS_COMPATIBILITY_2026-11-10.md)
- [Work packages for contributors and AI agents](docs/BACKEND_WORK_PACKAGES_2026-11-10.md)

## Scientific sources

- [NASA/JPL planetary ephemerides](https://ssd.jpl.nasa.gov/planets/eph_export.html)
- [NAIF generic SPK kernels](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/)
- [NAIF SPK required reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html)
- [NAIF rules governing kernels and toolkit](https://naif.jpl.nasa.gov/naif/rules.html)
- [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/)

**Important:** NASA/JPL public data and separately obtained ephemeris files are **not** licensed under this repository's CPAL-1.0 license. Do not commit .bsp/.eph files or copy NAIF toolkit / Swiss Ephemeris source code without a documented rights review. The SPK kernels' redistribution terms differ from those of the SPICE Toolkit.

## Project status

- Experimental Rust workspace: `Cargo.toml`, `crates/ephemeris-core/`, and the classic binary DE441 reader documented in [docs/DE441.md](docs/DE441.md).
- Experimental time-and-place CLI computes topocentric geometric positions using an externally supplied DE441 binary; see its documented time range and approximations.
- The experimental Rust `spk_index::inspect_spk` indexes NAIF DAF/SPK headers and segments in both IEEE byte orders. The `spk_direct_state::direct_spk_state` module evaluates primitive J2000/ICRF type-2/3 states. `spk_relative_state::relative_spk_state` combines states within **one kernel**, while the experimental `spk_kernel_set::SpkKernelSet` applies **later-loaded SPK precedence** and composes cross-file center chains. Frame conversion, other SPK types and real-kernel validation remain unimplemented.
- An experimental **offline** Rust SPK comparison example accepts externally obtained, independently reviewed NAIF/SPICE geometric state vectors, ordered SPK files and explicit error tolerances. A Python standard-library `spk_naif_reference.py compare` command verifies ordered kernel hashes, invokes the canonical Rust comparator with `cargo --offline`, and checks the reference and kernels again afterward. This is still **not** independent numerical parity evidence; no actual real-kernel comparison has been verified.
- The validation-only Python helper generates explicit J2000 geometric reference vectors using a separately installed NAIF/SPICE toolkit via SpiceyPy and checks ordered kernel/source-script SHA256 fingerprints; it is **not** part of the canonical Rust astronomical engine and has not established real JPL parity.
- The historical JavaScript prototypes and synthetic tests are research artifacts, not a second production astronomical engine.
- Real NASA/JPL reference-vector validation and supported SPK kernel reading remain open gates; passing synthetic tests does not establish scientific accuracy.
- Rust-backed WebAssembly/TypeScript and Python bindings, stable public releases and production integration are not implemented.
- GitHub Actions `Rust development CI` is present on `dev/ephemeris` and checks Rust formatting, debug/release tests, strict Clippy, Rustdoc, Windows GNU cross-compilation, and native Windows/macOS tests. Passing CI validates these code checks, **not** astronomical accuracy against independent NASA/JPL data.

**License:** Original VirgoStars-Ephemeris code is provided under [CPAL-1.0](LICENSE), including its Exhibits A/B, with required attribution and network-use provisions. See [NOTICE](NOTICE) and [Licensing and visible credits](docs/LICENSING_AND_CREDITS.md). A Credits/About link is encouraged but does not necessarily satisfy the license's launch/session prominence requirement.

This project is not affiliated with or endorsed by NASA, JPL, NAIF or Astrodienst. External datasets and dependencies retain their own terms.
