# Numerical validation protocol (design / no tests executed)

**No scientific accuracy, Swiss parity, or runtime compatibility is claimed yet.** Independent astronomical correctness is the primary gate; matching an existing astrology program is a secondary compatibility signal.

## Test matrix

1. **SPK/DAF:** endian handling; segment parsing, IDs, center-chain composition, type 2/3 if implemented, Chebyshev values/derivatives and segment boundaries. Tests against independently known SPK/NAIF reference states.
2. **Time:** explicit UTC, leap seconds/TAI, TT, TDB, UT1, Delta-T model/version, timezone offsets, Julian vs Gregorian policy, BCE/year zero, leap dates and out-of-range behavior.
3. **Vectors/frames:** SSB, Sun, Earth, Moon, planet centers vs barycenters; ICRF/J2000/equinox-of-date, ecliptic/equatorial conversions, precession/nutation/obliquity, topocentric/geodetic observer.
4. **Corrections:** light time (including iteration), aberration, selected relativistic effects, parallax, sidereal time, atmospheric refraction opt-in; clearly separate apparent/geometric/astrometric comparisons.
5. **Numerical edge cases:** near 0°/360°, near poles, high latitudes, zero/very small vectors, station points, fast Moon motion, longitude sign conventions, missing reference data, kernel overlap, interval endpoints.
6. **Events (later):** rise/set and polar exceptions, meridian crossing, angular-event root solving, multiple crossings, stationary events, bounded iterations and documented time tolerance.
7. **Operation:** deterministic offline replays, artifact size, Node and Astrolabio target runtime, memory, cold/warm cache, performance without weakening accuracy.

## Reference data

- **Primary:** JPL Horizons and/or NAIF-supported calculations using identical JPL kernels and precisely matched settings.
- **Secondary:** independently published astronomical examples and reference libraries with permissive terms.
- **Regression / legacy compatibility only:** existing VirgoStars outputs for selected synthetic inputs (including Swiss-based results). A discrepancy is not automatically a defect in the new code; inspect ephemeris family, frame, timescale, Delta-T, correction mode and rounding first.

## Accuracy contracts — define per capability

Each capability gets a review-approved tolerance table: angular degrees/arcseconds, positional distance, velocity, event time seconds, coverage range and percentiles/max error. **Do not predeclare numerical targets achieved.**

Dataset record example:

```json
{
  "caseId": "synthetic-moon-001",
  "input": {"utc": "2026-01-01T00:00:00Z", "observer": null},
  "reference": {
    "provider": "JPL_HORIZONS",
    "kernelSetId": "DE440",
    "frame": "ICRF",
    "center": "EARTH",
    "corrections": "GEOMETRIC",
    "sourceDate": "TO_BE_RECORDED"
  },
  "expected": "TO_BE_CAPTURED_FROM_REFERENCE",
  "tolerance": "TO_BE_SET_AND_APPROVED",
  "provenance": "TO_BE_RECORDED"
}
```

This is a **template, not a passing numerical fixture**.

## Required evidence per milestone

- Reproducible test command and package/runtime versions
- Kernel filenames, sizes, digest, epoch ranges, source URLs
- Exact reference query and settings; input timezone and UT conversion
- Per-target error distribution and worst cases
- Evidence whether comparison is geometric/apparent and center reference
- Explicit unsupported feature list; regression across earlier releases
- Manual QA on offline Astrolabio packages before changing device behavior

## Release safety

`DESIGNED -> IMPLEMENTED_UNVERIFIED -> VALIDATED_LOCAL -> RELEASE_CANDIDATE -> RELEASED`.
No automatic promotion. Incompatibility and uncertainty warnings must never be suppressed to make a legacy test pass.


## Esecuzione attuale — 2026-11-09

Nel branch della PR #1, `npm test` ha eseguito **18 test con esito positivo**. Sono test unitari e kernel **sintetici**; l'accuratezza su effemeridi NASA **non è stata verificata**.

Il modulo `tools/horizons.mjs` costruisce richieste ufficiali `VECTORS` a Horizons con `VEC_CORR=NONE`, `REF_SYSTEM=ICRF`, `REF_PLANE=FRAME`, `TIME_TYPE=TDB` e `OUT_UNITS=KM-S`. Il comando manuale `npm run verify:horizons -- /path/to/de440s.bsp 0 10 399` calcola errori cartesiani di posizione e velocità e registra lo SHA256 del file sorgente; **nessuna misura reale è disponibile** finché non viene eseguito su un kernel ufficiale. Horizons potrebbe usare una versione DE diversa da quella locale; annotarlo nella review. Specifiche: https://ssd-api.jpl.nasa.gov/doc/horizons.html .


### New math-only evidence — J2000 projection/light-time

- Local command: `node --test test/coordinates-lighttime.test.mjs` with Node.js 22.16.0.
- Outcome: **5/5 new tests passed** (vector rotations, angular/radial speeds, domain errors/singularities, synthetic moving-target retarded-time reference, deterministic nonconvergence).
- Git blob SHA-1 of the saved source and tests matched the files locally tested.
- This is not a Horizons accuracy comparison; these tests use synthetic J2000 state vectors and do not check true-of-date precession/nutation, apparent corrections, or a real SPK.
- **Do not summarize as 23/23 rerun:** only these five new tests were executed this turn, while the previous checkpoint recorded 18/18 earlier. A complete suite run is a separate next step.

Mandatory gating tests for future astrology: compare RA/Dec, ecliptic-of-date longitude/latitude/velocity against matched NAIF/IAU/JPL reference settings and error budgets. Do not feed `projectGeometricJ2000` output to the natal-chart provider as `APPARENT`.


### IAU 2006 independent numerical validation — local 2026-11-09

- Executed command: `node --test test/precession.test.mjs` with Node.js 22.16.0, yielding **5 passed / 0 failed**.
- Independent reference source: ERFA `t_erfa_c.c`, tests `eraObl06(2400000.5, 54388.0)` and `eraPmat06(2400000.5, 50123.9999)`; expected mean obliquity 0.4090749229387258204 rad and nine matrix coefficients reproduced in `test/precession.test.mjs`.
- Matrix maximum absolute discrepancy in an independent offline calculation approximately 1.11e-16, compared with acceptance of 1e-14 per element. Also tested orthogonality, determinant, time variation, invalid TT epochs, incompatible frames, zero radius, pole singularity.
- Github `src/precession.mjs` and `test/precession.test.mjs` blob SHAs matched the files actually executed locally.
- **Scope of this validation:** matrix and mean-coordinate algebra only. It does NOT validate nutation, apparent positions, rates, real NASA kernels or a full natal chart. Prior synthetic 18/18 and 5/5 reports were from previous separate runs; no complete combined-suite rerun was conducted in this iteration.


### IAU 2000B 77-term independent reference check — 2026-11-09

- Original JS `iau2000bNutation` evaluated against ERFA `eraNut00b` numeric reference at TT JD 2400000.5+53736; the two residuals were about 4.57e-20 and 2.71e-20 radians.
- The 3×3 mean-to-true nutation matrix was checked against ERFA `eraNum00b` using the matching IAU 2000 mean obliquity 0.4090791789404229916 radians; max absolute matrix-element residual approximately 5.60e-17.
- Complete 77 coefficient rows were audited for 11 finite values each, with scientific provenance and original NumFOCUS/ERFA BSD license retained.
- Five additional Node regression test cases were written to `test/nutation.test.mjs`, but **were not run as part of a complete `npm test`** in this development step; do not add them to a claimed passing test count.
- The expected 2000B ~1 mas accuracy around 1995–2050 is a published model characteristic, not a measured local full JPL position/velocity accuracy. A true-of-date GEOMETRIC projection is not an APPARENT position.


### Executed committed IAU 2000B regression logic — V8 isolated (2026-11-09)

After the new module was committed, the exact GitHub file blobs `src/nutation.mjs` (SHA `4cc712a0012a8928c513234ff1db5f73dc8117c0`), the 77-row data table and `test/nutation.test.mjs` were fetched by pinned commit `95a2a4f2498606b2c0f65d0404f35e64c8d5cefc`. Their JavaScript was evaluated in a standalone V8 isolate with minimal `node:test` and `node:assert` adapters and the committed IAU 2006 precession dependency. **All five nutation regression test cases PASSED (5/5, 0 failures)**, covering coefficients, ERFA nut00b and num00b reference values, geometric frame semantics, invalid epochs/inputs, and null date-frame rates.

**Scope limitation:** this is execution of the actual committed mathematical test logic in V8, **not** a Node.js `npm test` run of the complete source tree. Do not label it as CI, Node test suite success or NASA kernel validation. On a local checkout, run `npm test` and record Node version, date and pass count as a separate validation receipt.


### Whole authored Ephemeris test logic: 33/33 in V8 (9 November 2026)

All seven existing test files were independently fetched with their production source files from the current GitHub PR branch, and their actual JavaScript bodies executed in a V8 isolate with adapters for Node's `node:test`, `node:assert/strict`, Buffer binary reads/writes (DataView), and URL query parsing. There were **33 passing test cases, zero failures**:

| File | Tests executed and passed |
|---|---:|
| test/spk.test.mjs | 5 |
| test/type3-time.test.mjs | 6 |
| test/analytic.test.mjs | 4 |
| test/coordinates-lighttime.test.mjs | 5 |
| test/horizons-tools.test.mjs | 3 |
| test/precession.test.mjs | 5 |
| test/nutation.test.mjs | 5 |
| **Total** | **33** |

This confirms the authored test logic in a JavaScript engine against current GitHub sources, but the V8 adapters are not the Node standard library and the test files were evaluated without their original ESM import statements. **This is NOT evidence that `npm test` completed in Node, NOT a GitHub Actions/CI execution, and NOT a proof that authentic NASA/JPL kernels are handled correctly.**

Next: clone PR branch in an independent Node >=20 environment, run `npm test` unmodified, record Node version, exit code and exact commit SHA; then run `npm run verify:horizons -- /path/to/de440s.bsp 0 10 399` with real JPL files and independent vector comparisons. Keep these gates separate.
