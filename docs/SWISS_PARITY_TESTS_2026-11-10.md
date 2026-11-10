# Backend-first Swiss Ephemeris parity tests

**Added:** 2026-11-10. **Status:** partial reproducible test infrastructure, **not** a claim of planetary numerical equivalence. This plan complements [the backend call-site inventory](BACKEND_SWISS_COMPATIBILITY_2026-11-10.md) and [work package B](BACKEND_WORK_PACKAGES_2026-11-10.md).

## Real backend scope, in implementation order

Inspected `Cesare-ml/virgo-stars` `backend/services/astrology/func.js` and the existing public inventory. The **backend's actual calls**, rather than all of Swiss's API, determine the first test families:

| Priority | Backend function and configuration | Parity fields and owner | Current gate |
|---|---|---|---|
| P0 | `swe_julday(y,m,d,h+m/60,SE_GREG_CAL)` | Calendar scalar JD; core `calendar.rs` | **11 independent recorded calendar values tested** in Rust |
| P0 | `swe_calc_ut(jd,body,SEFLG_SPEED\|SEFLG_SWIEPH)` | Sun–Saturn first: longitude/latitude (deg), distance (AU), longitude/latitude speed (deg/day), distance speed (AU/day); Rust + adapter | **Offline comparator + optional oracle recorder prepared**, Rust apparent-of-date candidate API not available |
| P0 | `swe_calc_ut` for outer planets, lunar nodes/apogees, Chiron | Same fields; expanded datasets/models | Not implemented or independently validated |
| P0 | `swe_houses`, `swe_houses_pos` (V/P/K/R/O) | 12 cusps, axes/angles and `housePosition`; `astrological-functions` | Not implemented; separate functions-repo work |
| P1 | `swe_calc_ut` with `SEFLG_HELCTR` or `SEFLG_EQUATORIAL` | Correct heliocentric and equatorial modes, field names, optional speeds | Not ready; **do not** apply 6-field polar comparator blindly |
| P1 | `swe_calc(jd,SE_ECL_NUT,0)` | Mean/true obliquity and nutation; distinguish TT from UT | Not ready; existing backend passes JD built from civil UTC |
| P1 | `swe_azalt` | South-west Swiss azimuth, true/apparent altitude; refraction model | Not ready; backend rotates azimuth 180° |
| P1 | `swe_revjul(jd,SE_GREG_CAL)` | Reverse JD to year/month/day and decimal hour; Rust `calendar.rs` | **15 independent Swiss calendar values tested** in Rust (no time-scale conversion) |
| P1/P2 | `swe_rise_trans`, `swe_fixstar_ut`, `swe_fixstar_mag`, eclipse routines | Events, fixed stars, eclipses | Later, distinct per-function contracts |

These entries are *test targets*, not exposed Rust APIs. See [ADR-0001](ADR-0001-RUST-CORE-AND-BINDINGS.md): the only astronomical engine is Rust, with future thin bindings; zodiac/house logic belongs in `astrological-functions`.

## 1. First executed reference: calendar/JD

`crates/ephemeris-core/tests/fixtures/swiss_backend_julday.tsv` contains **11 separately obtained scalar outputs** of `pyswisseph 2.10.03` (Python binding), `swe_julday`, captured 2026-11-10. Dates include J2000 noon, modern date, Gregorian leap day, the 1900/2100 century boundary, 1582 Gregorian/Julian conventions, astronomical year zero and midnight. The Rust integration test `swiss_backend_julday.rs` compares the same year/month/day/decimal-hour/calendar inputs within **1e-9 JD** (86.4 microseconds) to accommodate binary64 rounding near contemporary JD numbers. It also verifies that the `Ut1` time-scale tag is preserved; **no UTC→UT1 conversion is performed**.

This is a real *calendar arithmetic* reference from a **different Swiss binding/version**, not yet a test of the backend's exact Node `swisseph@0.5.17` ABI. That backend version must be confirmed independently at adapter integration. Swiss `swe_julday` does not validate leap-second clocks, convert time scales, or produce planetary positions.

Run with: `cargo test --workspace --locked --test swiss_backend_julday` (or the entire workspace). Normal CI requires **no Swiss binary, `.se1` file, private backend repository or network**.

### Reverse JD reference added — 2026-11-10

The legacy backend also calls `swe_revjul` in planetary-hour and event workflows. The existing Rust `JulianDate::to_calendar` supports this **calendar arithmetic** without a new astronomy API. Added `crates/ephemeris-core/tests/fixtures/swiss_backend_revjul.tsv`, independently recorded with installed `pyswisseph 2.10.03` `swe_revjul(jd, SE_GREG_CAL/SE_JUL_CAL)`, and `crates/ephemeris-core/tests/swiss_backend_revjul.rs`.

The **15 golden cases** cover the same modern epochs as the forward-Julian reference plus J2000 noon, a fractional second, 1900/2100, Gregorian and Julian 1582 dates, astronomical year 0, and negative/zero Julian days. Assertions compare the exact year/month/day, compare Swiss's decimal-hour result within **3e-7 hours = 1.08 milliseconds**, and preserve the caller's explicit `TimeScale::Ut1` label. Each reverse-reference row is also converted back to a split Julian date and checked against the original input within **1e-9 JD**, covering fractional-day carry and negative JD without requiring an external Swiss binary at test time. The time tolerance is deliberately conservative for the single-`f64` JD representation: it is **not** a claim about submillisecond UTC or leap-second precision, and no UTC-to-UT1 conversion is implied.

Reproduce just this Rust integration test with:

```bash
cargo test --workspace --locked --test swiss_backend_revjul
```

These 15 reverse-date values are actual outputs of a separately installed Swiss Python binding, but do **not** certify the future Node 0.5.17 compatibility adapter, event times, planetary positions or time-scale conversion.

## 2. Optional Node 0.5.17 positional oracle (external, never CI)

`scripts/record-swiss-backend-reference.cjs` may be executed manually in a licensed environment with **separately installed** `swisseph@0.5.17` and the exact **Swiss `.se1` dataset** used for the reference. No Swiss implementation, dependency or ephemeris file is included in VirgoStars-Ephemeris. The recorder uses the observed P0 invocation (`SEFLG_SPEED | SEFLG_SWIEPH`), the same Gregorian fractional-hour JD and seven bodies Sun–Saturn over four UTC calendar instants, producing **28 positional records**. It checks each returned `rflag & 7 == SEFLG_SWIEPH` and rejects a Moshier/JPL fallback or missing output.

Example for an appropriately configured local environment (the Node module must be separately resolvable via `NODE_PATH` or installed outside this repo):

```bash
export ASTRODIENST_EPHE_PATH=/separately/installed/swiss-ephemeris-directory
export SWISS_EPHE_DATA_ID='Swiss ephemeris files + locally recorded hashes/version'
export NODE_PATH=/path/to/virgo-stars/backend/node_modules
node scripts/record-swiss-backend-reference.cjs > /tmp/swiss-reference.json
```

**No oracle reference JSON is checked in**, because the actual backend data file hashes and environment are not available on this branch. Verify `dataId` against a recorded file-hash manifest; the recorder does not calculate those hashes for you. This tool does **not** validate the Swiss license or permission to redistribute recordings: review rights before checking reference data into a public repo.

## 3. Offline numerical comparison of reference vs Rust-candidate export

`scripts/compare-swiss-backend.mjs` compares two saved JSON exports with the same `schemaVersion: 1`. A reference record must declare `source.binding = "swisseph@0.5.17"`, `source.ephemeris = "SWIEPH"`, a meaningful `source.dataId` and the effective ephemeris source through each case's `effectiveFlags`. The candidate must declare `source.engine = "VirgoStars-Ephemeris"` and be produced by the **future Rust-backed, apparent-geocentric ecliptic-of-date adapter**. It rejects missing/duplicate case IDs, altered `jdUt`/`bodyId`/`flags`, non-finite/missing six-field output, and unverified Swiss source flags. The current comparison scope is deliberately **only** `SWIEPH|SPEED` (`258`): heliocentric, equatorial and other flags will require separately typed field contracts, not reinterpretation of this six-field case.

### Explicit frame and correction identity (update 2026-11-10)

**Both** saved exports must declare the following `source.resultContract`. The Swiss recorder now emits this object directly. A Rust candidate must not emit it until its output truly implements the declared pipeline (geocentric **apparent**, true ecliptic/equinox of date, degrees/AU, signed rates per day). A geometric DE441 ICRF/J2000 result, light-time-only result or topocentric position must fail closed; merely relabeling it would be scientifically incorrect.

```json
{
  "resultContract": {
    "center": "geocentric",
    "frame": "true_ecliptic_of_date",
    "corrections": "apparent",
    "inputTimeScale": "UT",
    "angleUnit": "degree",
    "distanceUnit": "AU",
    "rateUnit": "per_day"
  }
}
```

`inputTimeScale: "UT"` denotes Swiss's `swe_calc_ut` contract, **not** an assertion that the legacy backend's UTC-derived Julian day is rigorously UT1. No UTC-to-UT1, Delta-T or TT conversion is performed by the comparator. This metadata check guards against accidental frame mixing, but does **not** prove that an exporter correctly implemented the declared model.

The reference's effective flags must show `SEFLG_SWIEPH` **and** `SEFLG_SPEED`; a silent ephemeris fallback or dropped speed bit fails. Longitudes must be normalized into `[0,360)`, latitudes into `[-90,90]`, distances nonnegative; all six components must be finite. The circular angle distance remains in `[0,180]` even for negative or multi-turn values because each argument is normalized before subtracting. Such out-of-range raw outputs still fail validation as invalid Swiss-shaped results.

Each `cases[]` element:

```json
{
  "id": "2026-11-10T18:30Z-body-4",
  "function": "swe_calc_ut",
  "input": { "jdUt": 2461355.2708333335, "bodyId": 4, "flags": 258 },
  "effectiveFlags": 258,
  "output": {
    "longitude": 0.0, "latitude": 0.0, "distance": 0.0,
    "longitudeSpeed": 0.0, "latitudeSpeed": 0.0, "distanceSpeed": 0.0
  }
}
```

**The JSON above is an intentionally fictitious shape example, NOT a Swiss reference value.** The candidate JSON uses the same `id`, `function`, `input`, `output` shape and the engine identification above; only the reference requires `effectiveFlags`.

Create a separately reviewed tolerance file with **exactly six nonnegative finite numeric fields** named `longitude`, `latitude`, `distance`, `longitudeSpeed`, `latitudeSpeed`, `distanceSpeed`. Units are degrees, AU, degrees/day and AU/day. Angular longitude error is **circular**, i.e. 359.999° vs 0.001° differs by 0.002°, while rates are **signed**, so a retrograde-vs-direct mismatch fails. Other fields use absolute error; all six are tested per case. The CLI also prints the **maximum observed absolute error per field, with the case ID**, including on an otherwise successful comparison, so accuracy reviews can record a worst case instead of only a pass/fail. The comparison returns `maxErrors` alongside its prior `failures`, `totalCases` and `totalFields`; existing consumers of those fields remain compatible.

```bash
node scripts/compare-swiss-backend.mjs swiss-reference.json rust-candidate.json reviewed-tolerances.json
node --test test/compare-swiss-backend.test.mjs
```

**No production planetary tolerances are approved yet.** Choose and document budgets *before* claiming scientific parity, using independent JPL Horizons/NAIF validation, data version, time model, nutation/precession, light-time, aberration, frame and observer configuration. Do not widen thresholds just to turn red tests green. Node comparator tests use **synthetic values only** to exercise edge cases, not to claim astronomy accuracy.

## 4. Planned acceptance grid

- Bodies: Sun, Moon, Mercury, Venus, Mars, Jupiter, Saturn; then Uranus, Neptune, Pluto, mean/true lunar node, lunar apogees and Chiron as supported.
- Dates: J2000, modern leap day, current epoch, **verified** retrograde and station intervals, 0/360° crossings, historical births only when supported UTC/UT1/Delta-T policy is defined.
- Modes: default `swe_calc_ut` geocentric apparent tropical ecliptic-of-date + speeds; later heliocentric/equatorial, nutation, horizontal, fixed-star and event workflows.
- Error parity: missing data (must **not** silently use another source), bad body IDs, invalid/unsupported dates/flags, known polar no-event cases, and exact JS field shapes.
- Geography: geocentric `swe_calc_ut` must not depend on location; `swe_houses` and `swe_azalt` use explicitly ordered **longitude/latitude** inputs and separate locations (e.g. northern, southern, equatorial, polar).
- Test classes: (a) offline deterministic fixtures; (b) optional Swiss backend-binding parity; (c) independent Horizons science validation. Passing one **does not imply** the others.

## Reproducibility, safety, licensing

Swiss Ephemeris is **an external test oracle only**, never the astronomy engine, a build dependency or a vendored library. No Swiss source or `.se1` data may be copied into the Rust core. The Node oracle tool is disabled by default and **must not** run in the public CI. Values created by the Python Swiss binding for the small Julian-day reference dataset are identified with their binding/version separately from the native Node target. Preserve CPAL-1.0 notices and the VirgoStars credits requirements; all changes remain on a feature branch, with no deployment or backend migration.

Source pointers: [Swiss programming manual](https://www.astro.com/swisseph/swephprg.htm), [backend Swiss call inventory](BACKEND_SWISS_COMPATIBILITY_2026-11-10.md), [JPL/Horizons comparison process](DE441_HORIZONS_OFFLINE_COMPARISON_2026-11-10.md).

**Next action:** implement an actual six-field Rust apparent ecliptic-of-date output that can be independently compared to SWIEPH after a real external DE441 kernel, time-scale policy and correction model pass scientific validation. Until then, the positional comparator is **prepared but cannot be run end-to-end**.
