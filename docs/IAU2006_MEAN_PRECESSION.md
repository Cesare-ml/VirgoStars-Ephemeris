# IAU 2006 P03 — mean-of-date precession, independently benchmarked

Checkpoint: **2026-11-09**. Status: **isolated experimental implementation**, not a complete apparent-position engine or production-ready astrology.

## Calculations

- Original JavaScript module `src/precession.mjs` implements published IAU 2006 Fukushima–Williams angle polynomials and mean obliquity using TT Julian centuries since J2000.0. No Swiss Ephemeris or SOFA/ERFA source code has been copied.
- The 3×3 bias+precession matrix maps GCRS/ICRF Cartesian coordinates to the **mean equator and equinox of date**. The matrix includes frame bias, so it is not the identity even at J2000.0.
- `projectGeometricMeanOfDate(state,jdTt)` returns mean equatorial and mean ecliptic coordinates of date, with explicit `GEOMETRIC` correction flags. It only accepts a J2000 geometric position and a finite TT Julian date, presently restricted to approximately 1800–2200.
- It purposely reports **no velocity**: simple rotation of a velocity vector does not include the derivative of the moving frame, which is necessary for meaningful longitude speed and station/retrograde analysis.

## Independent numerical reference — ERFA

Reference package: ERFA, `t_erfa_c.c`. Public numeric test examples:
- `eraObl06(2400000.5, 54388.0)` = **0.4090749229387258204 rad**
- `eraPmat06(2400000.5, 50123.9999)`, complete 3×3 matrix stored as numeric expectations in the test file.

Executed locally with Node.js **22.16.0**: `node --test test/precession.test.mjs`, **5 tests passed**, 0 failed. Every matrix element agrees within **1e-14** absolute tolerance; the maximum absolute element discrepancy observed is around **1.1e-16**. The suite additionally tests orthogonality, determinant, time variation, unsupported epochs, incompatible states and singularities.

GitHub blob identities of the source and test files were compared with locally executed files. **The earlier reported synthetic parser tests were not rerun as one combined suite this turn.**

## Scientific scope and exclusions

- Included: IAU 2006 **mean** bias+precession and mean ecliptic of date.
- Missing: full IAU 2000A/2000B nutation, stellar aberration, gravitational deflection, Earth observer location and diurnal effects, iterative light-time composition with proper rates, genuine NASA kernel/Horizons integration.
- Hence **not** a validated `APPARENT/ECLIPTIC_OF_DATE` provider for natal charts. `astrological-functions` should keep rejecting it.
- Runtime cannot currently download authentic official NASA `de440s.bsp`. That remains a separate release gate (#3), as does apparent coordinate integration (#4).
- Only original code may be MIT; the scientific references, numerical test values and external ephemeris files retain their own terms and attribution.

## Scientific references

- Capitaine & Wallace (2006), *Astronomy & Astrophysics* 450, 855.
- Wallace & Capitaine (2006), *Astronomy & Astrophysics* 459, 981.
- Hilton et al. (2006), *Celestial Mechanics and Dynamical Astronomy* 94, 351.
- [ERFA test vectors](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c) and [ERFA pmat06 documentation](https://github.com/liberfa/erfa/blob/master/src/pmat06.c).
- [NAIF SPK technical specification](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html).

## Next acceptance gates

Obtain real SPK data for science validation, implement IAU nutation with independent reference checks, introduce aberration and observer transformations, and calculate consistent date-frame derivatives before emitting fully apparent astrological positions. No hidden fallback, no production migration or GitHub Actions.
