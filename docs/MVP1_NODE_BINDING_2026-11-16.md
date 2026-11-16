# MVP 1, row 5 — Node binding and load interface (scaffold)

**Status: Prototype / locally tested on Windows. NOT a drop-in replacement for the
`swisseph` package, NOT published, NOT integrated into any backend.** It exists so
that the planetary, calendar and star pipelines can be called from Node.js and so
the replacement contract can be tested offline. Location: `bindings/node/`.

## Decision: native addon (N-API), not WASM, for now

The backend calls the Swiss binding **synchronously** and the engine must read a
2.8 GB DE441 file. A native N-API addon gives synchronous file I/O with no copy of
the file into memory (measured on Windows: opening the file ≈ 1 ms, one
`calcUt` ≈ 0.04 ms once records are cached). WASM would need a file-access shim or
loading the data into memory, and is not rejected for the future: the Rust core
stays free of binding dependencies and the binding is a separate crate with its
own `Cargo.toml`/`Cargo.lock` (`bindings/node`, `napi 2`), so the core workspace
and its CI are unchanged.

Build: `cd bindings/node && npm run build` (needs Rust and a C-capable linker; uses
`cargo build --release --locked`, no download of data). Output:
`native/virgostars_ephemeris.node` (ignored by Git). Only Windows was built and run;
Linux and macOS are untested.

## API

Modern, explicit API (`index.js`, typed in `index.d.ts`):

| Call | Meaning |
|---|---|
| `new Engine(de441Path)` | opens one classic DE441 binary; a `.bsp`, `.se1`, missing or empty path is `E_EPHEMERIS_FILE`/`E_RANGE`. Each engine is independent (isolation test) |
| `engine.calcTdb(jdTdb, ipl, flags)` | apparent geocentric true ecliptic of date, six fields |
| `engine.calcUt(jdUt, ipl, flags)` | same, with a **declared** UT→TDB policy (1900-01-01 … 2027-06-28 UT, else `E_RANGE`) |
| `engine.loadHipparcos(hip2Path, hips, 'strict'\|'traditional')` / `engine.fixstarTdb(hip, rvKmS\|null, jdTdb, flags)` | stars from a caller-supplied `hip2.dat`; failed loads keep the previous selection; `rvKmS = null` is an explicit 0 km/s assumption reported in `notes` |
| `julday`, `revjul` | calendar arithmetic with an explicit calendar, no 1582 changeover, strict validation |

Every result is `{ xx, rflag, timeSource, notes }`: `xx` = `[lon°, lat°, dist AU,
dlon°/day, dlat°/day, ddist AU/day]` (speeds are zero without `SEFLG_SPEED`),
`rflag` always carries `SEFLG_JPLEPH` and never a Swiss-data bit, `timeSource` says
how the epoch was obtained, `notes` lists the corrections applied and any
assumption. Errors are `Error`s with a stable `code`: `E_RANGE`,
`E_UNSUPPORTED_FLAG`, `E_UNSUPPORTED_BODY`, `E_EPHEMERIS_FILE`, `E_CALC`,
`E_CATALOG_FILE`, `E_STAR_QUALITY`, `E_STAR_NOT_LOADED`, `E_NATIVE_MISSING`.

Supported flags: `SEFLG_JPLEPH` (1), `SEFLG_HELCTR` (8), `SEFLG_SPEED` (256),
`SEFLG_NOGDEFL` (512), `SEFLG_EQUATORIAL` (2048). Anything else (including
`SEFLG_MOSEPH`, `TOPOCTR`, sidereal) is rejected. Supported bodies: 0–9 (Sun…Pluto, and the Earth, 14, with `SEFLG_HELCTR` only; Mars–Pluto are DE441
**system barycenters**, as in Swiss JPL mode, and `notes` says so). Nodes, apogees,
Chiron, `SE_ECL_NUT`, houses, azimuth/altitude and rise/set are MVP 2 or other
repositories and return `E_UNSUPPORTED_BODY`/`E_UNSUPPORTED_FLAG`.

## Swiss-shaped facade (`createSwissShapedFacade`)

Read-only inspection of the backend source (no change made) shows it calls
`swe_julday(y, m, d, hour, SE_GREG_CAL)`, `swe_calc_ut(jd, body, flags)` with
`SEFLG_SPEED | SEFLG_SWIEPH`, and reads `longitude`, `latitude`, `longitudeSpeed`,
`latitudeSpeed`, `distance`, plus helio-/equatorial calls, `swe_calc(…, SE_ECL_NUT)`,
`swe_azalt`, `swe_houses`, `swe_houses_pos`, Chiron and `swe_get_planet_name`. The
facade covers only `swe_julday`, `swe_revjul`, `swe_calc_ut`, `swe_calc`,
`swe_get_planet_name`, with those result keys, `error` objects instead of throws,
and an optional synchronous trailing callback.

Deliberate, visible differences:

- The backend passes `SEFLG_SWIEPH`. The facade **rejects** it by default (it would
  assert Swiss data files), and maps it to "use the configured DE441 file" only if
  created with `{ acceptSwiephFlag: true }`; `rflag` never carries that bit.
- A directory of `.se1` files cannot be passed: the load interface takes a DE441
  *file*, and the facade has no `swe_set_ephe_path`.
- Key names, value types, error shape, synchronous callback behavior, `swe_julday`
  leniency and `SE_ECL_NUT` slots were checked against the installed `swisseph`
  0.5.17 on 2026-11-16 and aligned; see
  [MVP1_SWISSEPH_SHAPE_2026-11-16.md](MVP1_SWISSEPH_SHAPE_2026-11-16.md) for the
  table and what still differs.

## Evidence (2026-11-16, Windows 11, Node 22.18, Rust 1.99.1)

- `npm test` without any data: 6 pass, 9 skipped. With `DE441_PATH` and
  `HIP2_PATH` set: 15 pass, 0 fail (planets, flags/bodies errors, UT policy and
  hard limits, isolation of two engines, deflection flag, facade shapes,
  callback, a ten-body chart for a fictional instant, star policies, failed-load
  rollback, reported radial-velocity assumption).
- Core: new `jd_ut_to_tdb_with_policy`, tested for agreement with the civil entry
  and for hard limits (1900-01-01 and 2027-06-28 UT).
- **UT entry against the local reference** (Swiss `swe_calc_ut`, JPL mode, same
  file, 127 dates 1900–2027; aggregates in
  `docs/evidence/mvp1-node-ut-entry-local-reference-receipt-2026-11-16.json`): Sun
  and planets differ by at most 0.09″; the **Moon by up to 0.64″ before 1972 and
  0.39″ after**, i.e. about 1.2 s and 0.7 s of time. After 1972 that is the
  unmodeled UT1−UTC (below 0.9 s); before 1972 it is the difference between the
  smoothed ΔT polynomials and the reference's table. It is a known, reported
  limitation, not hidden; closing it needs measured UT1−UTC/ΔT data that this
  package does not bundle. The TDB entry matches the reference to 0.0007″ (see the
  apparent-position document).

## Measured ΔT (added)

`engine.loadDeltaT(...)` takes three caller-supplied USNO ΔT files; `calcUt` then
uses measured ΔT (and flagged predictions to 2033) instead of the leap-second and
polynomial policy, and reports `DELTA_T_TABLE:…` in `timeSource`. The Moon then
agrees with the reference to 0.02″ in 1972–2018. See
[MVP1_DELTA_T_MEASURED_2026-11-16.md](MVP1_DELTA_T_MEASURED_2026-11-16.md).

## House angles for `astrological-functions` (added)

`house_angles` (Rust core) computes ARMC (IAU 2006/2000B apparent sidereal time
plus east longitude), true obliquity, Ascendant and Midheaven; the binding exposes
`createHouseAngleProvider()` (the `calculateHouseAngles` interface of
`astrological-functions`, UT1 and TT supplied explicitly, nothing converted
silently) and `houseAnglesUt(jdUt, lat, lon)` (UT policy declared in
`timeSource`). It needs no ephemeris file. House *systems* stay in
`astrological-functions`. With this provider the recorded cusps of all five backend
systems at both fixture sites are reproduced to 1e-5°, and a local comparison with
an independent reference over 247 cases per system agrees to 2e-7″ (Placidus
0.0025″); see `docs/HOUSES_REFERENCE_VALIDATION_2026-11-16.md` in that repository
and its aggregate receipt. Core tests: `tests/house_angles.rs` (4); binding tests:
`test/houses.test.mjs` (5).

## Not done

1. Facade methods used by the backend but absent here: helio/equatorial output,
   `SE_ECL_NUT`, `swe_azalt`, a `swe_houses`-shaped facade over the provider and
   `astrological-functions` (the systems themselves are in that repository), Chiron/nodes/apogees, `swe_fixstar_ut`/`_mag` by
   name (needs the reviewed catalog and names).
2. Shapes beyond the listed calls (`swe_houses`, helio/equatorial output, `swe_azalt`, `swe_houses_pos`) against `swisseph` 0.5.17.
3. A clean-machine consumer test on an offline application, Linux/macOS builds,
   packaging, signed prebuilt binaries and any publication (none authorized).
4. Callback semantics: synchronous here by assumption.
5. A parallel run with the existing backend and a rollback plan (no production
   substitution; separate authorization).
