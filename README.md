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

## Manual NASA/Horizons comparison

The experimental `npm run verify:horizons -- /path/to/de440s.bsp 0 10 399` compares a locally provided SPK state with a live NASA Horizons ICRF geometric vector, reporting position/velocity residuals and kernel SHA-256. It makes no production changes and no automatic kernel download. **It has not yet been successfully run on a real NASA kernel** here because the isolated environment cannot download `.bsp` files. See [real-kernel validation](docs/REAL_KERNEL_VALIDATION.md). Current local test count: **18/18** for this PR's source including the three synthetic comparison-tool tests.


## Experimental J2000 coordinate projection and light-time (new checkpoint)

- `src/coordinates.mjs` converts validated **geometric J2000** Cartesian km/km/s states to equatorial RA/Dec, and to a **simple approximate mean-ecliptic J2000** longitude/latitude. It also computes angular and range rates. IAU 2006 J2000 mean obliquity is used, but ICRF frame bias, precession, nutation and apparent corrections are not.
- `src/light-time.mjs` computes a target's **retarded geometric vector**, relative to the observer at reception time, using an injected barycentric SPK-like state provider. This is one-way light-time only, **not complete apparent position**.
- `test/coordinates-lighttime.test.mjs`: **5 additional isolated Node 22 tests passed**, and their exact source/test Git blobs match the files locally executed. The *prior* checkpoint reported 18/18 across preceding tests; **the complete combined suite has not been rerun in this turn**.
- See [geometric coordinates and light-time specification](docs/GEOMETRIC_COORDINATES_AND_LIGHTTIME.md).

The strict `astrological-functions` natal provider must continue to **reject** these geometric/J2000 values until an accurate apparent Earth-centered ecliptic-of-date provider is implemented and validated.


## Mean-of-date IAU 2006 bias+precession — new experimental step

- `src/precession.mjs`: original implementation of the published IAU 2006 P03 Fukushima–Williams angle polynomials, mean obliquity, complete GCRS-to-mean-equator-of-date frame bias+precession matrix, and geometric **MEAN_ECLIPTIC_OF_DATE** projection.
- Local Node 22: **5/5 new tests passed**, including independent numerical ERFA reference benchmarks for mean obliquity and *all nine* IAU 2006 matrix components (max measured element error about 1.1e-16; test limit 1e-14). The committed source/test Git blob SHA hashes match the locally tested files.
- No nutation, aberration, topocentric corrections or proper rotating-date-frame velocity in this module. A mean ecliptic position is **NOT an apparent geocentric position**. The real NASA DE440s binary/Horizons comparison is still outstanding.
- Details: [IAU 2006 mean precession numerical evidence](docs/IAU2006_MEAN_PRECESSION.md).

## Full IAU 2000B nutation (experimental)

The [77-term IAU 2000B nutation module](docs/IAU2000B_NUTATION.md) now calculates dPsi/dEpsilon and true equatorial/ecliptic-of-date **geometric axis projections**. Independent ERFA reference values and the complete 3×3 nutation matrix were numerically checked (component residuals ≈1e-20 rad; matrix ≤5.6e-17). The newly authored 5 Node tests have **not** been rerun in the complete suite yet. This is **not a planetary apparent-position provider** and not scientifically validated on a real NASA .bsp. Coefficient provenance and BSD license are recorded in THIRD_PARTY_NOTICES.md.


**Additional validation:** the five newly authored IAU 2000B test cases were subsequently executed against the exact pinned GitHub code in a V8 JS isolate: **5/5 passed**. This is not a complete Node `npm test` run. Authentic NASA SPK scientific comparison remains a separate release gate.
