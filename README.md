# VirgoStars Ephemeris

**An independent, free/open-source astronomical ephemeris engine for astrology applications.**

**Status: EXPERIMENTAL FOUNDATION — an initial Rust numerical crate is under review; no binary SPK loader, validated planetary ephemeris, TypeScript/Python bindings or production integration yet.** This public repository is the future source of truth for a clean-room implementation based on publicly distributed NASA/JPL ephemeris kernels. It is **not** a fork, translation, wrapper, or redistribution of Swiss Ephemeris.

## Purpose

Provide verifiable astronomical quantities to [Astrological Functions](https://github.com/Cesare-ml/astrological-functions) and other clients, using NASA/JPL SPK kernels with documented algorithms. No login, database, Angular, artwork, interpretation texts, or VirgoStars application data belong here.

**Language decision:** The canonical astronomical core will be written in **Rust**, with thin WebAssembly/TypeScript and PyO3/maturin Python bindings so all consumers share the same algorithms. This is an accepted design, not an implemented release. See [ADR-0001](docs/ADR-0001-RUST-CORE-AND-BINDINGS.md).

**Separation of responsibilities:** `virgostars-ephemeris` handles time scales, kernel access, coordinates, physical/geometric corrections and eventually astronomical event primitives. `astrological-functions` handles zodiac/house methods, aspects, synodic phases, astrological events and application-facing chart models. VirgoStars production keeps using its current implementation until tested migration gates pass.

## Read first

- [Architecture and boundaries](docs/ARCHITECTURE.md)
- [Rust core, WebAssembly and Python bindings — ADR-0001](docs/ADR-0001-RUST-CORE-AND-BINDINGS.md)
- [Rust core phase-one milestone and manual test instructions](docs/RUST_CORE_PHASE1.md)
- [Engineering roadmap](docs/ROADMAP.md)
- [Astronomical accuracy and validation](docs/VALIDATION.md)
- [NASA/JPL kernels and legal provenance](docs/DATA_AND_LICENSING.md)
- [Proposed public contracts](docs/API_CONTRACTS.md)
- [Contributor and AI-agent rules](AGENTS.md)
- [Italian overview](docs/README.it.md)

## Scientific sources

- [NASA/JPL planetary ephemerides](https://ssd.jpl.nasa.gov/planets/eph_export.html)
- [NAIF generic SPK kernels](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/)
- [NAIF SPK required reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html)
- [NAIF rules governing kernels and toolkit](https://naif.jpl.nasa.gov/naif/rules.html)
- [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/)

**Important:** NASA/JPL public data is not itself licensed under our repository's MIT license. Do not commit .bsp files or copy NAIF toolkit / Swiss Ephemeris code without a documented rights review. The SPK kernels' redistribution terms differ from those of the SPICE Toolkit.

## Project status

- Public repository created.
- Requirements and phased architecture being documented.
- Initial Rust crate provides pure mathematical Chebyshev/time primitives and synthetic unit tests; no executable kernel reader or astronomical position validation yet.
- No compatible API, npm package, numerical-parity claim, benchmarks, or production migration yet.
- No GitHub Actions, automatic releases, or deployment configured.

An MIT license for original source is a proposed distribution choice, pending license/attribution review; do not assume NASA/JPL kernels or third-party tools are covered by it. Each external dependency and data source retains its own terms. This project is not affiliated with or endorsed by NASA, JPL, NAIF, or Astrodienst.
