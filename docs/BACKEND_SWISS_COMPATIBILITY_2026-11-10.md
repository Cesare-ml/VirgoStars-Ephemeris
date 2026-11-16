# Backend-first Swiss-style compatibility inventory

> **Updated product priority (2026-11-11):** [three MVPs for Swiss replacement](THREE_MVP_SWISS_REPLACEMENT_2026-11-11.md) is the authoritative owner-approved sequence. **MVP 1 now includes fixed stars, houses, `swe_julday` AND `swe_revjul`, full physical planet positions/speeds, and the load/Node facade.** MVP 2 includes all *other* observed functions (including lunar nodes/apogees and Chiron); MVP 3 includes all remaining Swiss APIs even if unused by VirgoStars. The P0/P1/P2 column below is a **historical 2026-11-10 technical snapshot, not the current product order**; preserve its source evidence and per-call details but use the new MVP classification for planning.

**Snapshot:** 2026-11-10. Planning document, not an implemented API or parity claim.
This inventory was derived from the local `virgo-stars` backend checkout at
commit `01218b54100be8b7e08724f8d1d4f83f36742495` and its installed
`swisseph` Node binding 0.5.17; `backend/package.json` specifies `^0.5.17`.
The active checkout and deployed version can differ: re-inventory at integration
and distinguish reachable endpoints, offline routines, experimental workers,
comments and tests. Paths below are relative to the VirgoStars checkout and are
identifiers for collaborators who have access; this public repository does not
require that checkout to build or test.

The owner's long-term goal is broad Swiss Ephemeris functionality, with similar
or identical backend usage and a modern interface where useful. Implement the
actual backend paths first. Swiss's [programming interface](https://www.astro.com/swisseph/swephprg.htm)
and [astronomical guide](https://www.astro.com/swisseph/swisseph.htm) are
behavioral references. Use independently sourced algorithms and respect this
repository's clean-room and licensing instructions in [AGENTS.md](../AGENTS.md).
No Swiss implementation or data is bundled or called by the new engine.

## Architecture and migration boundary

- Rust `ephemeris-core` owns astronomy and a typed, explicit configuration;
  `astrological-functions` owns house/zodiac algorithms and chart business
  logic. A future JS compatibility facade may compose both. Thin bindings call
  Rust; they must not duplicate astronomical calculations. See [ADR-0001](ADR-0001-RUST-CORE-AND-BINDINGS.md).
- The API sketches here are **proposed**, not exports available today. The
  0.5.17 binding is a compatibility reference in addition to the upstream C
  manual. Verify its actual JS behavior, not only TypeScript declarations.
- A modern per-instance context takes an explicit DE441 file, time policy and
  provenance. The compatibility layer may emulate `swe_set_ephe_path` for
  existing callers while isolating state and concurrency. A directory of
  `.se1` files is not automatically a classic DE441 file; configure the file
  explicitly. Do not map `SEFLG_SWIEPH` or `SEFLG_JPLEPH` silently to another
  source or report a Swiss source as used: either reject or offer a separately
  configured, explicit migration mapping with truthful effective flags and
  provenance. No silent Moshier or Swiss runtime fallback.
- Support the synchronous calls actually used in chart construction; preserve
  documented callback result/error behavior where used, and test ordering and
  loading behavior. Exact synchronous WASM loading and the choice of adapter
  packaging require a documented implementation decision; do not promise a
  drop-in package before its contract tests pass.
- Keep implementation tests, independent astronomical validation and backend
  migration parity as separate checks. A matching shape or a DE441 file alone
  does not establish agreement with apparent Swiss positions.

## Observed call sites and priorities

P0 = first chart / current-sky sample, P1 = chart features and related daily
calculations, P2 = other observed workflows. This is an ordering for useful
increments, not a claim that lower rows are unimportant. Scope lists **calls**,
not proof that every endpoint or routine is deployed or active.

| Priority | Observed call or flags | Evidence in `virgo-stars` | Required behavior / owner | Rust status |
|---|---|---|---|---|
| P0 | `swe_set_ephe_path`, `swe_julday`, `SE_GREG_CAL` | `backend/services/astrology/func.js`; `backend/domain/astrology/calculations/nativeEclipticLongitudeSample.js` | Context/path bridge; Gregorian date and fractional hour to JD. Callers interpret input as UTC; JD itself does not perform a timescale conversion. Rust core + JS facade. | Calendar/JD arithmetic exists; no Node facade or complete UTC/UT1 policy. |
| P0 | `swe_calc_ut(jd, body, SEFLG_SPEED \| SEFLG_SWIEPH)` | Same files; core sample pins 0.5.17 and consumes `longitude`, `longitudeSpeed` | Geocentric **apparent** ecliptic of date, degrees/AU and rates per day, for Sun–Saturn first; same 6 position/rate fields and error/flag behavior as the chosen binding contract. Rust + facade. | DE441 geometric states exist. `first_sky` is topocentric geometric, has no date-frame rates; not a substitute. |
| P0 | Sun through Pluto, mean/true lunar nodes, mean/osculating apogee, Chiron | `backend/services/astrology/func.js`: default IDs `[0..13,15]` | Implement body/model coverage in increments; declare unavailable bodies and dataset requirements explicitly. `de441.eph` does not itself supply lunar node/apogee models or Chiron. Rust / data strategy. | Planet-system vectors exist, lunar points and Chiron not implemented. |
| P0 | `swe_houses(jd, lat, lon, system)` and `swe_houses_pos(armc, lat, eps, system, lon, lat)` | `backend/services/astrology/func.js` | Systems `V`, `P`, `K`, `R`, `O`; JS result includes `house`, `ascendant`, `mc`, `armc` and other axes; house position field is `housePosition`. Astrology-functions + composed facade; core provides time, orientation, obliquity. | Not implemented in the inspected functions repo. |
| P1 | `swe_calc_ut` with `SEFLG_HELCTR` or `SEFLG_EQUATORIAL` | `backend/services/astrology/func.js` | Heliocentric ecliptic and geocentric equatorial variants; latter uses `rectAscension`, `declination`, their speed fields, distance and distance speed. Check the caller: it does not set `SEFLG_SPEED` on these two calls, so preserve observed binding semantics rather than inventing rates. Rust + facade. | Core has geometric ICRF states, not these apparent-of-date variants/facade. |
| P1 | `swe_calc(jd, SE_ECL_NUT, 0)` | `backend/services/astrology/func.js`; `backend/controllers/renderChart.controller.js` | `swe_calc` takes TT; `swe_calc_ut` takes UT. Document each ECL_NUT field and correct timescale. Existing callers pass a UT-derived JD to `swe_calc`; controller comments/field interpretations need a separate review, not automatic propagation. Rust + facade. | Full nutation/compatibility not implemented in Rust. |
| P1 | `swe_azalt(jd, SE_ECL2HOR, lon, lat, height, pressure, temperature, x, y, distance)` | `backend/services/astrology/func.js` | Return `azimuth`, `trueAltitude`, `apparentAltitude`; Swiss azimuth is measured from south toward west, and backend adds 180° to get north/east convention. Pressure 0 behavior must be tested, not treated as no refraction. Rust + facade. | `first_sky` has geometric altitude/north-based azimuth only; refraction and this signature missing. |
| P1 | `swe_revjul`, `swe_rise_trans`, `SE_CALC_RISE`, `SE_CALC_SET`, `SE_CALC_MTRANSIT` | `backend/services/astrology/func-ore-planetarie.js`; `backend/services/astrology/mezzogiorno-astronomico.js` | JD→calendar; rise, set and upper transit. Exact field is `transitTime` in binding declarations and planetary-hours call; the noon helper uses a different argument shape and `transit`, so verify runtime and fix/migrate deliberately rather than assuming both work. Include polar no-event and atmospheric/limb conventions. Rust + facade; planetary-hour orchestration belongs to functions. | Calendar inverse exists; rise/set/transit absent. |
| P1 | `swe_get_planet_name` | `backend/services/astrology/func.js` | `{name}` and stable IDs/names, including non-planetary IDs actually requested. Facade with explicit unsupported behavior. | No compatibility facade. |
| P2 | `swe_fixstar_ut`, `swe_fixstar_mag` | `backend/services/astrology/func.js` `getStella` | Names/aliases, magnitude, apparent coordinates and rates, plus missing-star behavior; acquire separately licensed catalog and review its terms. Rust + facade. | Not implemented. |
| P2 | `swe_sol_eclipse_when_glob`, `swe_lun_eclipse_when`, `SEFLG_JPLEPH`, `SE_ECL_*`, `swe_revjul` | `backend/routines/calcolo-eclissi-solari-per-db.js`; `backend/routines/calcolo-eclissi-lunari-per-db.js` | Forward global search, type flags, maximum and contact fields consumed by routines; their requested years are 0100–3000 and need a historical/future ΔT policy. Check binding results: routine field names and declarations are not fully aligned. Rust event primitives + facade. | Not implemented. |
| P2 | `swe_calc` callback, `SE_SUN`, `SE_ECL_NUT` | `backend/controllers/renderChart.controller.js` | Callback compatibility and correctly interpreted fields/timescales; validate the existing overloaded call shape and default arguments with the installed package. | Not implemented. |

`backend/scripts/agents/heart-real-sky-longitudinal-b1-preflight.js` also calls
`swe_julday` as a runtime diagnostic; it is not evidence of a production chart
calculation. `astrolabio/backend` contains similar legacy code but was not
assumed identical to the main backend and needs a separate inventory before
migration. Re-run a search for aliases and indirect wrappers as backend changes.

## Semantics that must survive the adapter

1. The manual's default `swe_calc` mode is apparent, geocentric, ecliptic,
   true equinox of date; `SEFLG_SPEED` requests rates. The facade must not label
   a geometric, topocentric or mean-equinox answer as equivalent. Body IDs,
   flags, units, callback/error object shape, effective `rflag`, ordering and
   missing-data behavior need recorded 0.5.17 contract cases.
2. `swe_calc_ut` uses UT while `swe_calc` uses TT. The backend constructs JD
   from UTC dates; a policy for UTC versus UT1 and ΔT is required. Existing
   timestamps include historical birth dates, while the experimental
   `first_sky` UTC table accepts only 1972-01-01 through 2027-06-27 UTC.
   Never silently extrapolate. Do not claim 0100–3000 until its models and
   tolerances are tested.
3. The binding uses degrees, AU and day-based derivatives for ordinary polar
   results; verify the exact coordinate and velocity conventions per flag.
   Preserve `0` as a valid coordinate or house position: existing truthiness
   checks skip zeros; this is a known caller concern, not a core requirement.
4. Existing chart assembly performs southern-hemisphere inversion, derived
   opposite points, dodecatemoria and house adjustments itself. These belong to
   chart/function orchestration; do not silently repeat them in the ephemeris.
5. A replacement also affects provenance and tests: the future worker
   `nativeEclipticLongitudeSample.js` pins package version, adapter name and
   engine in its fingerprint. Plan an explicit versioned backend migration;
   do not impersonate the original engine to satisfy that pin.
6. Real-file integration is opt-in and external. Numerical comparisons against
   official Horizons and a separately installed, appropriately licensed Swiss
   reference must record query settings, model differences, expected tolerances,
   cases, and effective ephemeris source. Do not commit kernels or reference
   executable/code. Verify permitted use of any recorded third-party fixtures.

## First integration gate

Deliver P0 as independently testable vertical slices: calendar/clock policy,
Sun–Saturn apparent geocentric positions **and speeds**, a synchronous Node
adapter with explicit configuration, followed by P0 houses and remaining
backend-default bodies. Verify both the smallest current-sky sample and a
representative natal chart, including errors, dates, field shapes and source
metadata. Both geocentric and topocentric tests are needed; a location alone
must not turn a geocentric `swe_calc_ut` into a topocentric one. Historical
birth dates remain gated until their ΔT/time policy and fixtures exist.
Backend replacement, publication and deployment remain separate authorized
integration work.

Task breakdown and acceptance tests: [backend work packages](BACKEND_WORK_PACKAGES_2026-11-10.md).
