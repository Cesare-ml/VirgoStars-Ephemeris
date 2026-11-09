# File-free analytic fallback — what Swiss actually does

State: **PARTIALLY IMPLEMENTED, NOT SCIENTIFICALLY VALIDATED**.

Swiss Ephemeris has a **built-in Moshier analytic ephemeris** (`SEFLG_MOSEPH`); it can use it when Swiss data files are missing. This is not just a low-resolution pre-sampled NASA cache. Reference: https://www.astro.com/swisseph/swephprg.htm

VirgoStars must NOT copy the Swiss/Moshier implementation or assume its precision. Instead, the independently implemented Lite prototype uses public astronomical formulae and data:
- JPL planetary approximate orbital-element Table 1, 1800–2050: https://ssd.jpl.nasa.gov/planets/approx_pos.html
- USNO approximate Sun from UT Julian date, about one arcminute quoted accuracy within two centuries of 2000 (not our measured accuracy): https://aa.usno.navy.mil/faq/sun_approx

## Mode matrix

| Mode | Scope in current prototype | Required files | Coordinate semantics |
|---|---|---|---|
| JPL Precision candidate | SPK type-2 read-only geometric positions | NASA/JPL SPK locally supplied | J2000 frame, TDB seconds, target/center IDs, km/km/s; no apparent corrections |
| Analytic Lite planetary | Mercury, Venus, EMB, Mars, Jupiter, Saturn, Uranus, Neptune | none | Heliocentric mean ecliptic J2000, JD(TDB), AU; Earth–Moon barycenter is **not** Earth center |
| Analytic Lite USNO Sun | Apparent approximate solar longitude/latitude/distance | none | Approximate geocentric ecliptic of date, JD(UT); not interchangeable with planetary Lite vectors |

## Policies

- Never switch automatically to an unverified fallback. The caller selects a named provider/method and checks its capability flags.
- Do not claim Moon, Pluto, asteroids, nodes, or precise physical Earth center from the JPL Kepler Table 1.
- Do not claim parity with Swiss. JPL lists materially lower accuracy for Table 1 (particularly Jupiter and Saturn); our implementation still requires independent comparison.
- Do not combine JD(TDB) planetary vectors with JD(UT) Sun results in a chart without explicit time/frame conversions.
- Long-span 3000 BCE–3000 CE JPL Table 2 + extra secular terms is a **future implementation**, not a current feature. Higher-accuracy analytic models and any dependencies require independent scientific/rights review.
- Each output exposes the model, date range, frame, center, corrections, and an unvalidated accuracy status.

## Next tasks

Implement independent higher-accuracy Sun/Moon/planet analytic series with documented validity and reference comparisons, and a strict configurable quality gate. Offline Astrolabio support must be verified against realistic hardware limits. Keep both data-file and no-data modes testable, with clearly distinct error budgets and no hidden fallback.
