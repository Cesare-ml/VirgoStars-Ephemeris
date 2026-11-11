# VirgoStars Ephemeris

**A free and open-source astronomical ephemeris project for astrology applications — licensed under CPAL-1.0.**

**Status: EARLY EXPERIMENTAL IMPLEMENTATION — the Rust core includes a classic JPL DE441 binary reader and synthetic tests, but independent NASA/JPL numerical validation, SPK support, cross-language bindings and production integration remain incomplete.** This is an original implementation, not a fork, translation, wrapper or redistribution of Swiss Ephemeris.


## Purpose

Provide verifiable astronomical quantities to [Astrological Functions](https://github.com/Cesare-ml/astrological-functions) and other clients, using NASA/JPL ephemerides with documented algorithms. The current experimental reader supports the classic DE441 binary; SPK support is planned. No login, database, Angular, artwork, interpretation texts, or VirgoStars application data belong here.

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
- [First geometric planet positions for a time and place](docs/FIRST_PLANET_POSITIONS_2026-11-10.md)
- [Experimental geocentric light-time state](docs/GEOCENTRIC_LIGHT_TIME_2026-11-10.md)
- [MVP-1 geocentric light-time six-field bridge in fixed axes (not yet apparent)](docs/MVP1_GEOCENTRIC_SIX_FIELDS_2026-11-11.md)
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
- The historical JavaScript prototypes and synthetic tests are research artifacts, not a second production astronomical engine.
- Real NASA/JPL reference-vector validation and supported SPK kernel reading remain open gates; passing synthetic tests does not establish scientific accuracy.
- Rust-backed WebAssembly/TypeScript and Python bindings, stable public releases and production integration are not implemented.
- GitHub Actions `Rust development CI` is present on `dev/ephemeris` and checks Rust formatting, debug/release tests, strict Clippy, Rustdoc, Windows GNU cross-compilation, and native Windows/macOS tests. Passing CI validates these code checks, **not** astronomical accuracy against independent NASA/JPL data.

**License:** Original VirgoStars-Ephemeris code is provided under [CPAL-1.0](LICENSE), including its Exhibits A/B, with required attribution and network-use provisions. See [NOTICE](NOTICE) and [Licensing and visible credits](docs/LICENSING_AND_CREDITS.md). A Credits/About link is encouraged but does not necessarily satisfy the license's launch/session prominence requirement.

This project is not affiliated with or endorsed by NASA, JPL, NAIF or Astrodienst. External datasets and dependencies retain their own terms.
