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
