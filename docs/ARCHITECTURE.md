# Architecture — astronomical engine (design v0, 2026-11-09)

Status: **DESIGNED**; no implementation or released API. Goal: independent, testable, offline-capable astronomical calculations; not a reverse-engineered Swiss Ephemeris clone.

## Boundaries

```text
User / Astrological Functions / other clients
                  |
      Public, versioned ephemeris API
                  |
 [time scales + EOP] -- [SPK catalog + segment resolver]
                  |             |
 [observer models] -- [geometric state vectors]
                  |             |
 [light-time + aberration + relativistic corrections]
                  |
 [precession / nutation / frames / coordinates / rates]
                  |
           validated numeric results
```

- This package **owns** SPK kernel ingestion, target/center state vectors, astronomical time, coordinate systems, observational corrections, Sun/Moon/planet state and optional event primitives.
- `astrological-functions` **owns** astrological house system definitions, signs, aspects, orbs, interpretations-independent chart types, aspect searches, transits, returns and method-specific rules. No duplication of astrological policies inside Ephemeris.
- The private VirgoStars application **owns** authentication, DB, users, interpretations, content, REST/MCP, monetization, caching policy and production rollout.
- `astrological-charts` visualizes chart facts and does not infer new astronomical or astrological facts.

## Technical direction (proposal, not shipped)

- **TypeScript** library with strongly typed input/output and no Angular or PostgreSQL dependency.
- SPK/DAF reading layer: file summary, endian detection, segment descriptors, target/center IDs, validity intervals, supported SPK types (type 2 first; type 3 if required), Chebyshev evaluation and analytic derivatives. No silent unsupported-segment approximation.
- Kernel catalog: immutable manifest with source URL, JPL solution, SHA-256, NAIF IDs, segment coverage, frame and required companion kernels; offline cache managed by caller, not hidden network calls.
- Vector evaluation: explicit barycenter/planet-center chain composition; distinguish planetary system barycenters from physical planet centers. Earth/Moon paths require correct Earth-Moon barycenter and relative segments.
- Time module: UTC with leap-second policy; TAI, TT, TDB, UT1 and optional EOP; J2000 conventions; Julian-day variants; calendar handling for BCE/long span, no reliance on JavaScript Date outside supported range.
- Coordinate module: ICRF/J2000, equatorial/ecliptic of date, geocentric/heliocentric/topocentric, alt/az, local sidereal time, obliquity, precession, nutation, Earth rotation and polar motion where available. Provenance/tolerances per model.
- Apparent-position module: geometric vs astrometric vs apparent must be selectable and named; topocentric needs observer geodesy, light time and aberration choices, optional refraction (inputs explicit).
- Core must not contain Swiss-derived transformations, zodiac sign names, interpretations, houses, configurable astrology policies, account data or any graphic renderer.

## Recommended technology decision gate

Evaluate an **original SPK reader** for type-2 Chebyshev first. Cross-check with authoritative NAIF tests; use JPL Horizons as external validation. ERFA (BSD-3-Clause) is a candidate for IAU-style routines; using it means clearly attributing its license and deciding native vs WASM portability. Any other dependency requires a provenance/license decision. Never bundle the NAIF toolkit as though it were MIT.

## Versioning and reproducibility

Every result must report: `engineVersion`, `algorithmVersion`, `kernelSetId`, `kernelHashes` or digest, `validityRange`, `frame`, `timeScale`, `corrections` and `warnings`. An epoch outside the kernel's coverage or any broken center chain is an explicit error.

Proposed package namespaces: `time/`, `spk/`, `kernels/`, `frames/`, `state/`, `observer/`, `corrections/`, `events/`, `api/`, `tests/`. Directories do not exist yet.

## Consumers

Web backend and Astrolabio should load **the same pinned, tested released package** (Astrolabio offline bundle separately tested). No runtime network dependency by default. The new code must not be activated in production merely by merging a repository or publishing a version.

## Source references

- NAIF SPK Required Reading: https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html
- NAIF Generic Kernels: https://naif.jpl.nasa.gov/naif/data_generic.html
- JPL Horizons: https://ssd.jpl.nasa.gov/horizons/
- JPL DE440/DE441 overview: https://ssd.jpl.nasa.gov/doc/de440_de441.html
