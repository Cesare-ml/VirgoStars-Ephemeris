# VirgoStars Ephemeris — implementation roadmap

Status: **planning only**, as of 2026-11-09. All gates below are acceptance criteria, not evidence of completion. Use reviewed PRs and manual/offline validations; do **not** create GitHub Actions.

| Phase | Scope | Exit gate |
|---|---|---|
| E0 | Public project setup: architecture, clean-room rules, legal provenance, API proposals | Docs reviewed, MIT code license, no proprietary payload |
| E1 | Small SPK/DAF reader, type-2 Chebyshev, segment/center-chain resolver, pinned `de440s.bsp` fixture workflow | Independent NAIF vector comparison for selected Sun/Earth/Moon/planet cases |
| E2 | UTC/TAI/TT/TDB/UT1, leap seconds, geocentric/heliocentric state and velocity, frame conventions | Reproducible, tolerance-defined comparison against JPL Horizons |
| E3 | Ecliptic/equatorial of date, precession/nutation, apparent/astrometric modes, topocentric/horizon | Validation across epochs/locations; explicit EOP/refraction limitations |
| E4 | Broader body coverage, additional SPK types/kernels, supported event primitives | Every capability has supported-range contract, tests, documented limitations |
| E5 | Stable 1.0 API, offline package and performance review | Semver contracts, real tests, security/license audit |
| E6 | Controlled integration through Astrological Functions then VirgoStars/Astrolabio | Parallel-run parity, staged rollout, verified rollback; no existing users disrupted |

### Definition of E1 minimal vertical slice

Read *one official* DE440s SPK locally; parse headers and segment metadata; evaluate at least one known state at one epoch; correctly distinguish target/center/frame and position-vs-velocity. Show agreement with an independent NAIF reference at a documented tolerance. No tropical longitude, natal chart or Swiss replacement claims in E1.

### Deliberately later / not promised in v1

Exhaustive Swiss Ephemeris feature equivalence (all house systems, thousands of minor bodies, hypothetical points, obscure historical ranges, unusual sidereal traditions). The **functions library** owns astrology-specific parts. Any extension needs verified data availability, separate model/rights review and a scientific test.

### Open decisions

- Pure TypeScript vs a small WASM/native module for ERFA-like astronomical routines.
- Precisely supported reference frames/corrections in first stable release.
- Degree of support for past dates and BCE calendars; extreme ranges require independent Delta-T policy.
- Kernel packaging/download mechanism for offline Astrolabio; no automatic downloading of multi-GB kernels.
- Contributor review / funding / support model. None of these decisions block preparing an E1 prototype.

### Invariants

No Swiss Ephemeris code or runtime dependency; no code from private VirgoStars; no bundled proprietary datasets; each calculation independently testable; no silent fallbacks; production remains unchanged until explicit integration gates.
