# VirgoStars Ephemeris

**A free and open-source astronomical ephemeris project for astrology applications — licensed under CPAL-1.0.**

**Status: EARLY EXPERIMENTAL IMPLEMENTATION — a classic JPL DE441 JavaScript reader and the first Rust mathematical core are under development; a validated astronomical engine and language bindings are not complete.** This public repository is the future source of truth for a clean-room implementation based on publicly distributed NASA/JPL ephemeris kernels. It is **not** a fork, translation, wrapper, or redistribution of Swiss Ephemeris.

## Purpose

Provide verifiable astronomical quantities to [Astrological Functions](https://github.com/Cesare-ml/astrological-functions) and other clients, using NASA/JPL SPK kernels with documented algorithms. No login, database, Angular, artwork, interpretation texts, or VirgoStars application data belong here.

**Language decision:** The canonical astronomical core will be written in **Rust**, with thin WebAssembly/TypeScript and PyO3/maturin Python bindings so all consumers share the same algorithms. This is an accepted design, not an implemented release. See [ADR-0001](docs/ADR-0001-RUST-CORE-AND-BINDINGS.md).

**Separation of responsibilities:** `virgostars-ephemeris` handles time scales, kernel access, coordinates, physical/geometric corrections and eventually astronomical event primitives. `astrological-functions` handles zodiac/house methods, aspects, synodic phases, astrological events and application-facing chart models. VirgoStars production keeps using its current implementation until tested migration gates pass.

## Read first

- [CPAL-1.0 license, with VirgoStars attribution in Exhibit B](LICENSE)
- [NOTICE of copyright and third-party exclusions](NOTICE)
- [Licensing, required attribution and credits](docs/LICENSING_AND_CREDITS.md)

- [Architecture and boundaries](docs/ARCHITECTURE.md)
- [Rust core, WebAssembly and Python bindings — ADR-0001](docs/ADR-0001-RUST-CORE-AND-BINDINGS.md)
- [Rust phase-one foundation and manual test procedure](docs/RUST_CORE_PHASE1.md)
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

**Important:** NASA/JPL public data and separately obtained ephemeris files are **not** licensed under this repository's CPAL-1.0 license. Do not commit .bsp/.eph files or copy NAIF toolkit / Swiss Ephemeris source code without a documented rights review. The SPK kernels' redistribution terms differ from those of the SPICE Toolkit.

## Project status

- Public repository created.
- Requirements and phased architecture being documented.
- Experimental classic JPL DE441 binary reader in `src/jpl-de441.mjs` with isolated synthetic tests in `test/jpl-de441.test.mjs`.
- Experimental Rust Cargo workspace in `crates/ephemeris-core/`: typed TDB epochs, synthetic Type-2-style Chebyshev interpolation, analytical derivatives, and unit tests. No SPK file loader, compiled test verification or independent astronomical validation yet.
- No production-ready Rust engine, language bindings, published npm/PyPI package, numerical-parity claim, benchmarks, or production migration yet.
- No GitHub Actions, automatic releases, or deployment configured.

**License:** the original VirgoStars-Ephemeris source code is provided under the [Common Public Attribution License 1.0 (CPAL-1.0)](LICENSE), with completed Exhibits A and B. See [NOTICE](NOTICE) and [Licensing and visible credits](docs/LICENSING_AND_CREDITS.md). This license permits commercial use and redistribution, but requires preservation of notices and, where applicable, prominent *Powered by VirgoStars* attribution (including in Larger Works). It also has network-use source-availability obligations. A normal About/Credits link is encouraged but cannot automatically be assumed to satisfy the actual CPAL §14 launch/session display requirement.

This project is not affiliated with or endorsed by NASA, JPL, NAIF, or Astrodienst. External dependencies and datasets retain their own terms.
