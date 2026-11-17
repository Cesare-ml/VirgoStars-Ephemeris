# Planet comparison from astronomical year 0 through 3000

**Status: experimental diagnostic, not release approval or a continuous error
bound.** All comparisons below were executed locally on Windows on 2026-11-17
with the experimental long-term path and the light-time fix in this branch.
The ordinary `calcTdb`/`calcUt` apparent path still has its existing date
guard; this does not activate long-range calculations in the desktop or the
main server.

## Scope and methodology

Ten geocentric targets: Sun, Moon, Mercury, Venus, and the Mars through Pluto
**system barycenters**. The caller-supplied classic DE441 file has SHA-256
`476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`;
no kernel, Swiss catalog or Horizons response is committed. Dates are
proleptic Gregorian with astronomical year numbering (year 0 = 1 BCE),
sampled at 12:00 **TDB**, not civil UTC or UT1.

The experimental ICRF direction applies light-time, solar deflection and
annual aberration; its output is compared with Horizons apparent airless
geocentric ICRF right ascension/declination. Horizons is queried at an
approximately corresponding TT instant. Response body/center, DE441 source,
reference columns and each TT Julian date are checked. Differences are angular
separations in arcseconds. Both use DE441: this tests separate calculations
against the same underlying orbital solution, not observational truth or
independence of the orbital model. [JPL's DE441 paper](https://ssd.jpl.nasa.gov/doc/de440_de441.html)
and [Horizons manual](https://ssd.jpl.nasa.gov/horizons/manual.html) describe
the ephemeris and its time/frame conventions.

The separate comparison with caller-installed `swisseph` 0.5.17 uses its JPL
mode and verifies the actual JPL flag on every reference call. Both use the
same DE441; absolute ecliptic-of-date differences also include the effects of
different precession/nutation implementations. Longitude differences wrap at
360 degrees. One-second-of-arc component tolerance was fixed before the
seasonal run as an investigation threshold, **not** a scientifically approved
product tolerance. The TT conversion is approximate and both historical
orientation and time uncertainties are outside these bounds.

## Results

| Diagnostic | Samples | Result |
| --- | ---: | --- |
| Horizons ICRF direction | 301 dates × 10 bodies = 3,010 | Maximum Moon 0.000070886″ (year 470); maximum other targets 0.000007766″ (Mercury, year 2760). Below the predeclared investigative limits of 0.05″ and 0.01″. |
| Swiss ecliptic positions, four seasons | 301 years × 4 dates × 10 bodies = 12,040 | Maximum longitude difference 0.065564″ (Pluto barycenter, year 0, April); latitude 0.205861″ (Mercury, year 0, January). No component exceeded the 1″ diagnostic threshold. |
| Swiss and finite-difference rates, July | 301 dates × 10 bodies = 3,010 | Maximum engine lunar longitude rate minus Swiss finite difference 0.305204″/day (year 2490). Excluding Moon, the maxima are 0.001746″/day longitude and 0.003040″/day latitude (both Venus). Swiss's own reported rate differs from its finite difference by up to 0.343076″/day on this grid. Cause and acceptable rate tolerance remain open. |
| Internal rate consistency, July | same 3,010 | Maximum engine rate minus its own ±0.001-day finite difference: 0.000115″/day longitude and 0.000074″/day latitude, both Moon. This is a consistency check, not independent validation. |

Horizons used 1 July only, in steps of ten years. Swiss seasonal dates are
1 January, April, July and October every ten years. The tests do not establish
accuracy between dates, for every time of day, for planet centers distinct
from barycenters, or for topocentric coordinates, houses and small bodies.
They do not independently establish solar-system ephemeris uncertainty.

## Failure found and fixed

On the denser grid, the old solver rejected Uranus at 1100-07-01 TDB: the
fixed-point light-time iteration stopped with a four-epsilon tolerance, yet
the subsequent refinement correctly rejected the larger residual hidden by
rounding. The solver now continues until the next emission epoch is within
`epsilon * |observation JD|`, accepting adjacent representable epochs while
still handling a two-cycle. A synthetic linear-motion case compares the
refined range and velocity with a closed-form solution; an opt-in real DE441
test exercises the observed epoch. Both failed before and passed after the fix.
Existing kernel-coverage and invalid-geometry checks are preserved.

## Civil time and release limits

The long-term TDB path does not convert historical dates into UT1. The
separate experimental UT1 method needs an explicitly supplied ΔT and
uncertainty; the available old polynomial is an **estimate**, with no
uncertainty or guarantee for ancient/future dates. UTC, UT1, TT and TDB must
not be conflated. No actual birth-time or local-time validation is implied.
In particular, the extrapolated nutation/precession combination and model
differences need an agreed error budget and independent review before an
accuracy claim for astrology or production. Complete stellar coverage remains
the owner's separate gate for the main server.

## Reproduction and evidence

After building `bindings/node` with `npm run build`, provide a reviewed local
classic DE441 file, its SHA-256, and a local Swiss module. Output paths must be
new and remain local:

```text
node scripts/compare-horizons-apparent-icrf.cjs <de441> <sha256> <new-report> 0-3000-step-10
node scripts/compare-swiss-long-range.cjs <de441> <sha256> <swisseph> <new-report> 0-3000-step-10
node scripts/compare-swiss-seasonal.cjs <de441> <sha256> <swisseph> <new-report>
```

The Horizons script batches requests and retains the public request URL and
raw response in the ignored local report. Locally retained report SHA-256s:

- `horizons-icrf-10-review.json`: `52c64f17fd864949e570d4f8e08c263bd892bb5a44bb7557f1e771599fface4b`;
- `swiss-10.json`: `808011e7462fbe064b56924bd4cd661875055b3c298f8bab2f52da4627d371f4`;
- `swiss-seasonal-10.json`: `a117f45125d3a23f2cd232833dad23cd19241b59b936d5229548e5eef9ec7682`.

These checksums bind the local files but do not authenticate their source.
Earlier 250-year reports were separately summarized by
`scripts/summarize_planet_validation_0_3000.mjs`; that utility only
re-examines saved differences and does not run the engine.
