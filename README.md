# VirgoStars Ephemeris

**An independent, free/open-source astronomical ephemeris engine for astrology applications.**

**Status: EXPERIMENTAL PROTOTYPE — the first source code and synthetic tests exist, but astronomical accuracy on real NASA/JPL kernels has NOT been validated.** This public repository is the future source of truth for a clean-room implementation based on publicly distributed NASA/JPL ephemeris kernels. It is **not** a fork, translation, wrapper, or redistribution of Swiss Ephemeris.

## Purpose

Provide verifiable astronomical quantities to [Astrological Functions](https://github.com/Cesare-ml/astrological-functions) and other clients, using NASA/JPL SPK kernels with documented algorithms. No login, database, Angular, artwork, interpretation texts, or VirgoStars application data belong here.

**Separation of responsibilities:** `virgostars-ephemeris` handles time scales, kernel access, coordinates, physical/geometric corrections and eventually astronomical event primitives. `astrological-functions` handles zodiac/house methods, aspects, synodic phases, astrological events and application-facing chart models. VirgoStars production keeps using its current implementation until tested migration gates pass.

## Read first

- [Architecture and boundaries](docs/ARCHITECTURE.md)
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
- Experimental unversioned API and local npm package metadata on the feature branch only; no scientific parity claim, public release, benchmarks or production migration yet.
- No GitHub Actions, automatic releases, or deployment configured. Tests must run manually.

Our code is intended to be MIT-licensed as specified in [LICENSE](LICENSE); each external dependency and data source retains its own terms. This project is not affiliated with or endorsed by NASA, JPL, NAIF, or Astrodienst.

## Implementation checkpoint — 9 November 2026

**Status: implemented locally, scientifically UNVERIFIED.** The first isolated Node.js ESM source files and 9 synthetic/sanity tests now exist on the feature branch (PR #1). This supersedes the initial “no engine implemented” note above for the branch only; main is still the baseline until PR merge.

- `src/spk.mjs`: independent read-only DAF/SPK parser, SPK type-2 Chebyshev interpolation and velocity, geometric J2000 target/center chain composition, endian handling and explicit errors.
- `src/analytic.mjs`: optional file-free, explicitly lower-accuracy analytic calculations using JPL short-range Kepler elements and USNO's approximate apparent Sun (not copied Moshier code).
- `test/*.test.mjs`: 9 passing Node 22 local tests against **synthetic** SPK data and numerical sanity checks; no NASA/JPL .bsp file or Horizons comparison has yet been tested.
- `package.json`: dependency-free local prototype with `npm test`; `private: true` prevents accidental npm publication.

Read [implementation checkpoint](docs/IMPLEMENTATION_CHECKPOINT_2026-11-09.md), [analytic fallback design](docs/ANALYTIC_FALLBACK.md), and [actual-kernel validation instructions](docs/REAL_KERNEL_VALIDATION.md) before attempting to integrate anything. There is no automatic fallback from precision to lower accuracy.
