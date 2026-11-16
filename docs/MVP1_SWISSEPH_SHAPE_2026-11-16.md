# MVP 1, row 5 — shape of the Swiss-style facade, checked against `swisseph` 0.5.17

**Status: Locally tested against the installed `swisseph` 0.5.17 package (the
backend's own `node_modules`). Covers the listed calls only. NOT a complete
drop-in replacement, NOT integrated.**

The installed package (Windows build) was loaded read-only from a caller-supplied
path; it is AGPL and is **not** a dependency of this project. Nothing it returned is
stored in the repository. The contract tests in
`bindings/node/test/swisseph-shape.test.mjs` run only when
`SWISSEPH_PATH` (directory of the installed package) and `DE441_PATH` are set and
assert agreement; without them they are skipped.

## What the real package does, and what the facade now does

| Call / behavior | Real `swisseph` 0.5.17 (observed) | Facade |
|---|---|---|
| `swe_calc_ut`, `swe_calc` result keys | `longitude, latitude, distance, longitudeSpeed, latitudeSpeed, distanceSpeed, rflag`, all numbers | **identical keys and types** (provenance only with `{ provenance: true }`) |
| Without `SEFLG_SPEED` | speeds are `0` | identical |
| `rflag` | flags used; `257` for JPL + speed | identical in JPL mode (`257`); with the SWIEPH flag mapped the SWIEPH bit is absent on purpose |
| Error | `{ error: "<text>" }`, single key, no throw | `{ error }` single enumerable key; machine-readable `code` is non-enumerable |
| Callback | called **synchronously, before the call returns**; the call also **returns the result**; errors go to the callback as `{ error }` too | identical |
| `swe_julday` | pure arithmetic, never rejects: `2001-02-30` → March 2; month 13 → January next year; day 0 / −1 / 32; hour −1 / 48 / 25; year, month and day truncated to integers; missing or non-numeric arguments give `NaN` (or a meaningless number); proleptic Gregorian without a 1582 changeover (`1582-10-10` → `2299155.5`) | **reproduced arithmetically** (`lenientJulday`); the strict `julday` that rejects invalid dates stays available for the modern API |
| `swe_revjul` | `{ year, month, day, hour }` | identical, both calendars, on a 1721059–2488069 grid |
| `swe_calc(jd, SE_ECL_NUT, 0)` | `longitude` = true obliquity, `latitude` = mean obliquity, `distance` = nutation in longitude, `longitudeSpeed` = nutation in obliquity (all degrees), the other two `0`, `rflag 0` | **implemented** from the core's IAU 2006/2000B values; agree to < 3e-7° |
| `swe_get_planet_name` | `{ name }`; `Mars` for 4; the number as a string for unknown numbers | the ten bodies and the number-as-string rule; names of other bodies come with those bodies |
| Constants | `SE_SUN…SE_PLUTO`, `SE_GREG_CAL=1`, `SE_JUL_CAL=0`, `SE_ECL_NUT=-1`, `SEFLG_*` | identical values for the ones the backend reads |

Contract tests with the real package: 10 shape tests plus the 20 engine, star,
house and contract tests: **30 pass, 0 fail** with `SWISSEPH_PATH`, `DE441_PATH`
and `HIP2_PATH` set; 20 pass / 10 skipped without the real package; 11 pass / 19
skipped with no data at all.

## Differences that remain, by decision or by gap

- **`SEFLG_SWIEPH`.** The backend passes it. The facade rejects it unless created
  with `{ acceptSwiephFlag: true }`; the result never claims Swiss data files.
- **Heliocentric and equatorial output** (`SEFLG_HELCTR`, `SEFLG_EQUATORIAL`) now exist
  and return the reference's keys (`rectAscension, declination, rectAscensionSpeed,
  declinationSpeed` for equatorial) and `rflag` values; see
  [MVP1_HELIOCENTRIC_EQUATORIAL_2026-11-16.md](MVP1_HELIOCENTRIC_EQUATORIAL_2026-11-16.md),
  including the up-to-0.9″ inner-planet difference in heliocentric light-time.
- **Flags and bodies not yet computed** (MVP 2): `SEFLG_MOSEPH`, `SEFLG_NOABERR`,
  `SEFLG_TRUEPOS`, `SEFLG_J2000`, `SEFLG_NONUT`, `SEFLG_BARYCTR`, `SEFLG_TOPOCTR`,
  sidereal modes, the interpolated apsides (21, 22) and every body above Pluto
  except the lunar points and Chiron (15, only after `loadChiron`, see
  [MVP2_CHIRON_2026-11-16.md](MVP2_CHIRON_2026-11-16.md)). They return an `{ error }` object. Bodies 10–13
  (nodes and apogees) are offered: see
  [MVP2_LUNAR_POINTS_2026-11-16.md](MVP2_LUNAR_POINTS_2026-11-16.md).
- **Moshier fallback.** With a missing flag or missing data files the real package
  silently falls back to its built-in Moshier ephemeris (`rflag 4`) or reports a
  missing `.se1` file. The facade never falls back: out-of-range epochs and
  unsupported requests are errors.
- **Time conversion.** `swe_calc_ut` converts UT with the package's own ΔT; the
  facade uses the declared policy (IERS leap seconds from 1972, Espenak–Meeus ΔT
  1900–1972). Positions agree to within 0.1″ for Sun and planets and 0.8″ for the
  Moon in the test dates (see the Node-binding document).
- **`swe_houses` / `swe_houses_pos`** (systems V, P, K, R, O; Placidus position) are offered through
  injected house methods, with all nine result keys: see
  [MVP2_HOUSES_FACADE_2026-11-16.md](MVP2_HOUSES_FACADE_2026-11-16.md).
- **`swe_azalt`, `swe_azalt_rev`, `swe_refrac`, `swe_refrac_extended`, `swe_rise_trans`**
  are offered, with the reference's result keys: see
  [MVP2_HORIZON_EVENTS_2026-11-16.md](MVP2_HORIZON_EVENTS_2026-11-16.md).
- **Not tested:** asynchronous behavior beyond the synchronous callback, other
  Node versions, Linux and macOS builds of either package, and the 2.7-million-year
  range of `swe_revjul` (the core's calendar is limited to its supported year range).
