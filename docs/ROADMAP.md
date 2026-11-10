# VirgoStars Ephemeris — implementation roadmap

Status: **planning and experimental implementation**, updated 2026-11-10. The experimental Rust reader integrated from `dev/ephemeris` currently handles the **classic DE441 binary format**, not SPK `.bsp`; JavaScript SPK prototypes on `main` remain research only. See [branch integration status](BRANCH_INTEGRATION_STATUS_2026-11-10.md). All gates below are acceptance criteria, not evidence of completion. The owner authorized GitHub Actions for safe CI on 2026-11-10; CI checks complement, but do not replace, manually documented independent astronomical validation.

| Phase | Scope | Exit gate |
|---|---|---|
| E0 | Public project setup: architecture, clean-room rules, legal provenance, API proposals | Docs reviewed, CPAL-1.0 code license with required attribution, no proprietary payload |
| E1 | Small SPK/DAF reader, type-2 Chebyshev, segment/center-chain resolver, pinned `de440s.bsp` fixture workflow | Independent NAIF vector comparison for selected Sun/Earth/Moon/planet cases |
| E2 | UTC/TAI/TT/TDB/UT1, leap seconds, geocentric/heliocentric state and velocity, frame conventions | Reproducible, tolerance-defined comparison against JPL Horizons |
| E3 | Ecliptic/equatorial of date, precession/nutation, apparent/astrometric modes, topocentric/horizon | Validation across epochs/locations; explicit EOP/refraction limitations |
| E4 | Broader body coverage, additional SPK types/kernels, supported event primitives | Every capability has supported-range contract, tests, documented limitations |
| E5 | Stable 1.0 API, offline package and performance review | Semver contracts, real tests, security/license audit |
| E6 | Controlled integration through Astrological Functions then VirgoStars/Astrolabio | Parallel-run parity, staged rollout, verified rollback; no existing users disrupted |

### Definition of E1 minimal vertical slice

Read *one official* DE440s SPK locally; parse headers and segment metadata; evaluate at least one known state at one epoch; correctly distinguish target/center/frame and position-vs-velocity. Show agreement with an independent NAIF reference at a documented tolerance. No tropical longitude, natal chart or Swiss replacement claims in E1.

### Full functional coverage target

The owner has set coverage of the documented Swiss Ephemeris capabilities as the long-term goal, including house systems, minor bodies, hypothetical points, historical ranges and sidereal traditions. The first usable milestone is a time-and-location planet calculation; the remaining families are tracked in [FEATURE_COVERAGE.md](FEATURE_COVERAGE.md). The **functions library** owns astrology-specific parts. Each extension still needs verified data availability, documented models and independent numerical tests; full coverage is not yet implemented.

### Open decisions

- Scope and packaging of thin Rust-backed WASM/TypeScript and Python interfaces, without separate astronomy implementations.
- Precisely supported reference frames/corrections in first stable release.
- Degree of support for past dates and BCE calendars; extreme ranges require independent Delta-T policy.
- Kernel packaging/download mechanism for offline Astrolabio; no automatic downloading of multi-GB kernels.
- Contributor review / funding / support model. None of these decisions block preparing an E1 prototype.

### Invariants

No Swiss Ephemeris code or runtime dependency; no code from private VirgoStars; no bundled proprietary datasets; each calculation independently testable; no silent fallbacks; production remains unchanged until explicit integration gates.


## Checkpoint di lavoro — 2026-11-09 (documentazione aggiornata)

- E1 **parzialmente implementata, non convalidata**: lettori SPK tipo 2 e tipo 3, posizione e velocità, catene target/center, gestione endian; test solo su kernel sintetici. **Non effettuato il confronto con un file NASA reale**.
- E2 **avviata ma non validata**: modulo UTC/TAI/TT/TDB approssimato valido dichiaratamente dal 1972 al 2026, con tabella leap-seconds conforme a IERS Bulletin C 72. Non sostituisce una implementazione completa del modello temporale SPICE/IAU.
- Strumenti di verifica: `scripts/inspect-spk.mjs` e `scripts/compare-horizons.mjs`, con `npm run verify:horizons -- /path/to/de440s.bsp 0 10 399` per confronto *manuale* online con Horizons. Non sono stati eseguiti con dati reali.
- Test locali eseguiti su Node 22.16.0: **18/18 superati**; prove su dati sintetici e unit test, non scientific parity. Nessuna modifica a produzione, dati o Astrolabio.
- **Blocco attuale:** occorre ottenere ufficialmente un kernel `de440s.bsp` per eseguire e registrare la comparazione numerica; nel runtime isolato il download non è riuscito.


### Subsequent checkpoint: geometric coordinates and light-time

E3 now has an **experimental partial implementation**, not scientific validation: equatorial/approximate ecliptic J2000 coordinates and angular speeds from geometric vectors, and an optional one-way light-time-only retarded vector solver. Five new synthetic math tests passed in an isolated Node 22 workspace. This does **not** constitute precession/nutation to the date, aberration, true apparent positions or a VirgoStars chart-ready ephemeris. Real SPK/Horizons scientific comparison remains blocked. See `GEOMETRIC_COORDINATES_AND_LIGHTTIME.md`.

Before E3 can be considered validated, implement and independently test frame bias, IAU precession/nutation, Earth observer/stellar aberration, light-time time-derivatives and a canonical ecliptic-of-date result with declared correction modes. Never re-label simple J2000 projections as apparent.


### E3 update — independent IAU 2006 P03 numerical benchmark

The original `src/precession.mjs` now implements bias+precession and **mean** equatorial/ecliptic-of-date geometric positional transforms. Five new local Node 22 tests passed. The numerical unit-test oracle is the published ERFA conformance suite (`pmat06` 9-element matrix, `obl06` mean obliquity) rather than only an internal synthetic numerical check. Worst observed element discrepancy approximately 1.1e-16 with a pass tolerance of 1e-14. The implementation is deliberately limited to 1800–2200 TT pending wider validation and omits velocities in rotating date frames to prevent false retrograde classifications.

**Still blocked before E3 completion:** real NASA JPL SPK vectors, IAU nutation, aberration, proper apparent observer corrections, trusted lunar/planet models and full apparent-of-date rates.


### E3 progress — full IAU 2000B short nutation

The project branch now contains `src/nutation.mjs`, a full 77-term IAU 2000B lunar-solar series and the published planetary-bias offsets (not a 4-term approximation). It produces dPsi/dEpsilon, a mean-to-true equatorial nutation matrix and geometric `TRUE_EQUATORIAL_OF_DATE` / `TRUE_ECLIPTIC_OF_DATE` projections by combining with IAU 2006 P03 precession. Original JavaScript code is separated from the BSD-licensed ERFA scientific coefficient table in `src/data/` with retained copyright/license notes.

**Independent mathematical benchmark:** at TT JD 2453736.5, the calculated two nutation components agree with ERFA nut00b within ~5e-20 rad, and the full 3×3 nutation matrix matches ERFA num00b at max absolute element difference ~5.6e-17 (using ERFA's IAU 2000 reference mean obliquity for that standalone matrix check). Five new Node tests are authored but **the complete Node suite has not been executed this checkpoint**.

**Release remains blocked** by real official NASA SPK validation (#3), aberration, proper light-time/observer composition and date-frame derivatives. IAU 2006/2000B is a declared approximate combination, not the higher-accuracy 2006/2000A standard. No change to the strict APPARENT natal contract or production.
