# Geometric coordinates and one-way light-time — experimental v0.1

**Status: IMPLEMENTED_UNVERIFIED.** These are original mathematical routines, not a complete apparent-position API and not yet checked against a real NASA/JPL SPK.

## J2000 projection
`projectGeometricJ2000(state)` consumes a valid `stateAtTdbSeconds()` result (geometric J2000 km/km/s) and returns:
- Equatorial J2000 longitude (right ascension), latitude (declination), range and angular/radial rates.
- **Approximate** mean ecliptic J2000 longitude, latitude and rates, using a simple rotation by the IAU 2006 mean obliquity of J2000, 84381.406 arcsec.
- Explicit warnings that the exact ICRF/J2000 frame bias, precession/nutation to the *date*, and observer/light-time corrections are not applied.

The rotating direction, 0/360 boundary, derivatives and degenerate vectors have synthetic tests. An approximate J2000 ecliptic longitude is **NOT** a tropical or apparent geocentric longitude of date; VirgoStars natal chart calculations must still reject it.

## Retarded-time vector
`solveOneWayLightTime(provider,target,observer,etTdbSeconds)` implements iterative one-way light-time between a moving target at its *emission time* and an observer at its fixed *reception time*. Provider must return geometric barycentric J2000 position vectors (km), with `center:0` and uncorrected coordinates. The solver returns range, emission epoch, light-time, iterations and warnings, but **does not return a velocity** (derivatives of retarded positions need separate handling). No aberration, gravity deflection, refraction or time-dependent reference frame transformation.

This is **LIGHT_TIME_ONLY**, not full `APPARENT`, and is tested only with synthetic linear motion. On a large SPK, repeatedly reading/inspecting whole buffers is currently expensive; optimization by caching parsed segments remains future work.

## Test evidence
5 new tests passed on Node 22.16.0 in isolated workspace (rotation, velocity, invalid frames/singularities, analytic retarded-time reference, and error/convergence behavior). Earlier 18 Ephemeris tests were reported in the previous checkpoint; **this new turn has not yet rerun the complete combined test suite**. Do not imply a 23/23 combined run.

## Scientific reference
NAIF SPK required reading: https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html
IAU 2006 mean obliquity reference: IAU/IERS astronomical standards (J2000, 84381.406 arcsec); match the exact frame convention before quantitative comparisons.

## Next gates
1. Validate SPK reading on a real official .bsp and Horizons geometric vectors.
2. Convert to actual apparent geocentric positions of date with physically defined correction models (precession, nutation, light-time, stellar aberration), time scales, planetary center handling.
3. Add explicit error budget and test against an independent IAU/NAIF implementation before connecting to `astrological-functions`.
