# IAU 2000B — 77-term nutation & true-of-date axis model

Checkpoint: 9 November 2026. **EXPERIMENTAL** scientific module, not an apparent geocentric planetary-position API.

## Implemented

- Complete 77-term lunar-solar IAU 2000B nutation series (`src/nutation.mjs`), with the two published 2000B planetary-bias offsets -0.135 and +0.388 milliarcseconds, and ERFA-published coefficient data attributed separately.
- `iau2000bNutation(jdTt)` returns `dPsi`, `dEpsilon` in radians and arcseconds, limited to **1900–2100 TT** in this preliminary API. IAU 2000B's cited ≈1 mas agreement with IAU 2000A applies roughly 1995–2050, and is **not** a locally established accuracy bound for 1900–2100.
- `iau2000bNutationMatrix(jdTt,meanObliquityRad?)` computes the mean-to-true equatorial rotation; mean obliquity can be supplied for verification with the IAU 2000 reference model, or defaults to IAU 2006.
- `iau2006With2000bTrueMatrix(jdTt)` composes approximate IAU 2006 frame bias/precession with IAU 2000B nutation, with explicit model name explaining the cross-model approximation.
- `projectGeometricTrueOfDate(state,jdTt)` returns **GEOMETRIC/TRUE_EQUATORIAL_OF_DATE and GEOMETRIC/TRUE_ECLIPTIC_OF_DATE**, deliberately **not** `APPARENT/ECLIPTIC_OF_DATE`. No velocity: date-frame derivative absent.

## Independent numerical verification

Using the public ERFA nut00b/num00b test oracle at TT JD 2453736.5:
- expected dPsi = -0.9632552291148362783e-5 rad
- expected dEpsilon = 0.4063197106621159367e-4 rad
- complete nine-element `eraNum00b` 3x3 matrix benchmark with ERFA mean obliquity
- calculations evaluated independently within the JavaScript tool runtime: **nutation components differ by ≈4.6e-20 / 2.7e-20 rad**, maximum nutation-matrix element difference ≈5.6e-17.
- Five Node test cases have been authored in `test/nutation.test.mjs`. **The full Node test suite has not been executed in this development step**, and there is no claim of a new pass count for the entire repository.

## Provenance and licenses

Only original JavaScript algorithms in `src/nutation.mjs` are proposed as MIT source. The 77 numerical coefficients derive from the BSD-licensed ERFA reference and retain explicit NumFOCUS attribution and ERFA license text in `third_party/ERFA_LICENSE.txt`; see `THIRD_PARTY_NOTICES.md`. Swiss Ephemeris is not used. The mean-to-true coordinate matrix is not yet validated as a complete apparent astronomical pipeline.

## Scientific references

- ERFA tests: https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c
- ERFA nut00b: https://github.com/liberfa/erfa/blob/master/src/nut00b.c
- IAU SOFA / IERS fundamentals: https://www.iausofa.org/
- 2000B short-series basis: McCarthy & Luzum (2003), Celestial Mechanics and Dynamical Astronomy 85, 37–49.
- Previously verified 2006 bias/precession: `docs/IAU2006_MEAN_PRECESSION.md`.

## Incomplete / blocked

No actual SPK binary validated with a NASA/Horizons reference (#3). No stellar aberration, gravitational deflection, full Earth orientation/topocentric parallax or correct date-frame longitude-rate model (#4). Strict `astrological-functions` natal provider must reject these geometric results as a source for `APPARENT/ECLIPTIC_OF_DATE`.

No production switch, Astrolabio upgrade, GitHub Actions or npm publication.
