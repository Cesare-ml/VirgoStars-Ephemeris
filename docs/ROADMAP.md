# VirgoStars Ephemeris — implementation roadmap

Status: **planning and experimental implementation**, reviewed 2026-11-16. The classic Rust DE441 reader handles the classic binary `.eph` format. Experimental Rust modules index DAF/SPK metadata, evaluate supported J2000 frame-1 type-2/type-3 segments, and compose one- and multi-kernel target–center chains. A [limited type-2 DE440s comparison](SPK_NAIF_LOCAL_COMPARISON_2026-11-16.md) and [type-3/multi-kernel comparisons](SPK_NAIF_TYPE3_MULTI_KERNEL_2026-11-16.md) against CSPICE passed locally; provenance, scientific review and broader validation remain open. This is not a full SPICE implementation or production-ready. Historical JavaScript milestones below are retained as history. See the [current MVP checkpoint](MVP1_RECONCILIATION_2026-11-16.md). Gates below are acceptance criteria, not evidence of completion. CI complements independent astronomical validation.

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


## Historical JavaScript prototype checkpoint — 2026-11-09 (documentazione aggiornata)

- E1 **parzialmente implementata, non convalidata**: lettori SPK tipo 2 e tipo 3, posizione e velocità, catene target/center, gestione endian; test solo su kernel sintetici. **Non effettuato il confronto con un file NASA reale**.
- E2 **avviata ma non validata**: modulo UTC/TAI/TT/TDB approssimato valido dichiaratamente dal 1972 al 2026, con tabella leap-seconds conforme a IERS Bulletin C 72. Non sostituisce una implementazione completa del modello temporale SPICE/IAU.
- Strumenti di verifica: `scripts/inspect-spk.mjs` e `scripts/compare-horizons.mjs`, con `npm run verify:horizons -- /path/to/de440s.bsp 0 10 399` per confronto *manuale* online con Horizons. Non sono stati eseguiti con dati reali.
- Test locali eseguiti su Node 22.16.0: **18/18 superati**; prove su dati sintetici e unit test, non scientific parity. Nessuna modifica a produzione, dati o Astrolabio.
- **Blocco attuale:** occorre ottenere ufficialmente un kernel `de440s.bsp` per eseguire e registrare la comparazione numerica; nel runtime isolato il download non è riuscito.


### Historical JavaScript prototype checkpoint — geometric coordinates and light-time (not the current Rust implementation)

The legacy JavaScript prototype recorded an experimental partial implementation of equatorial/approximate ecliptic J2000 coordinates and angular speeds from geometric vectors, plus an optional one-way light-time-only retarded-vector solver. Its five synthetic Node 22 tests were historical software checks, not scientific validation. This does **not** establish that the current Rust branch provides precession/nutation to date, aberration, true apparent positions or a VirgoStars chart-ready ephemeris. Real SPK/Horizons scientific comparison remains blocked. For the current Rust geometric light-time scope, see [Geocentric light-time state](GEOCENTRIC_LIGHT_TIME_2026-11-10.md).

Before E3 can be considered validated, implement and independently test frame bias, IAU precession/nutation, Earth observer/stellar aberration, light-time time-derivatives and a canonical ecliptic-of-date result with declared correction modes. Never re-label simple J2000 projections as apparent.


### Historical JavaScript prototype checkpoint — IAU 2006 P03 numerical benchmark (not the current Rust implementation)

At that historical checkpoint, the legacy JavaScript prototype on an older branch contained `src/precession.mjs` with bias+precession and **mean** equatorial/ecliptic-of-date geometric positional transforms. Five Node 22 tests were reported for that prototype. Its numerical oracle was the published ERFA conformance suite (`pmat06` 9-element matrix, `obl06` mean obliquity); the reported worst matrix-element discrepancy was approximately 1.1e-16 against a 1e-14 tolerance. The prototype was limited to 1800–2200 TT and omitted rotating-frame velocities. These historical results do **not** establish that `src/precession.mjs` exists on the current `dev/ephemeris` branch or that the Rust implementation supplies this model.

**Still blocked before E3 completion:** independently validated real NASA/JPL SPK vectors, IAU nutation integrated and validated in the Rust pipeline, aberration, proper apparent observer corrections, trusted lunar/planet models and full apparent-of-date rates.


### Historical JavaScript prototype checkpoint — IAU 2000B short nutation (not the current Rust implementation)

At that historical checkpoint, a legacy JavaScript prototype on an older branch contained `src/nutation.mjs`, a 77-term IAU 2000B lunar-solar series and published planetary-bias offsets. It produced dPsi/dEpsilon, a mean-to-true equatorial nutation matrix and geometric `TRUE_EQUATORIAL_OF_DATE` / `TRUE_ECLIPTIC_OF_DATE` projections when combined with the prototype's IAU 2006 P03 precession. The original JavaScript and the BSD-licensed ERFA coefficient table under `src/data/` had separate provenance and notices in that historical implementation.

**Historical mathematical benchmark (not current Rust validation):** at TT JD 2453736.5, the prototype's two nutation components were reported to agree with ERFA nut00b within ~5e-20 rad, and its 3×3 nutation matrix with ERFA num00b within ~5.6e-17 maximum absolute element difference (using ERFA's IAU 2000 reference mean obliquity for that standalone matrix check). Five Node tests had been authored, but the complete Node suite had not been executed at that checkpoint. Neither those tests nor these measurements certify the current Rust implementation.

**Release remains blocked** by real official NASA SPK validation (#3), aberration, proper light-time/observer composition and date-frame derivatives. IAU 2006/2000B is a declared approximate combination, not the higher-accuracy 2006/2000A standard. No change to the strict APPARENT natal contract or production.
