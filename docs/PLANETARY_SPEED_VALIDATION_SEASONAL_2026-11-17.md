# Seasonal planetary speed validation — 2026-11-17

**Status: experimental sampled comparisons; not release certification.** This
continues [the solar-rate investigation](SOLAR_RATE_OUTLIERS_AND_HISTORICAL_TIME_2026-11-17.md).
There is no change to the astronomical engine, public API or production path.
The kernel and external reference reports remain caller-owned, ignored files.

## Geometric velocity: direct Horizons vectors

Using the local classic DE441, we requested [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/manual.html)
geometric Cartesian vectors, Earth center to target, ICRF, TDB, no light-time,
aberration or deflection. Targets are Sun, Moon, Mercury, Venus, and the Mars
through Pluto **system barycenters**. We sampled 1 January, April and October
12:00 TDB for astronomical years 0–3000 every 50 years: **183 epochs × 10
targets = 1,830 cases**. Dates use proleptic Gregorian input converted to JD.
JPL states are expressed in km and km/s; the engine velocity is converted
from km/TDB-day to km/s.

The parser checks each response's body, center, DE441 solution, ICRF frame,
geometric mode, units, TDB epoch and all six components. The predeclared
investigative per-axis thresholds were **0.1 km** and **1e-7 km/s**. No case
exceeded them. Largest absolute component residuals were **1.91e-6 km** in
position (Pluto barycenter, year 200, October) and **3.55e-14 km/s** in
velocity (Mercury, year 550, October). Component velocity RMS was
**5.77e-15 km/s** across all cases.

These values show agreement of the two calculations using the **same DE441**
orbital solution and are close to numerical precision. They are not estimates
of the actual orbit's physical error, and do not establish apparent-position
or apparent-speed accuracy, continuous coverage between samples, planet-center
velocities for Mars–Pluto, or topocentric results.

## Apparent ecliptic-of-date rates: Swiss cross-model check

With caller-installed Node `swisseph` 0.5.17 / Swiss Ephemeris 2.09.03 in JPL
mode, we evaluated 1 January, April and October 12:00 TDB, years 0–3000 every
20 years: **4,530 cases**. Each center and offset reference call verifies the
JPL flag; both implementations use the same local DE441. The engine returns
geocentric apparent true-ecliptic-of-date longitude, latitude and distance
rates; Swiss computes a corresponding apparent ecliptic-of-date result.
Precession/nutation and other apparent-model conventions can differ. The
engine takes TDB, while Swiss takes approximately corresponding TT; Swiss's
reported rates are per TT day and the conversion uses an approximation.

For each reference, a fourth-order centered derivative of its *positions* at
±0.02 and ±0.04 TDB day is compared to the engine rate, using actual rounded JD
intervals and longitude wrapping. Finite differences of the engine's positions
provide a separate internal consistency check. Maximum absolute component
differences on this grid are:

| Comparison | Longitude | Latitude | Distance |
| --- | ---: | ---: | ---: |
| Engine rate vs Swiss position derivative | 0.02087″/day | 0.04879″/day | 4.29e-10 AU/day |
| Engine rate vs its own position derivative | 0.000244″/day | 0.000159″/day | 4.18e-10 AU/day |
| Engine rate vs Swiss reported speed | 0.27141″/day | 0.08737″/day | 8.10e-5 AU/day |

The first two largest angular cross-model differences both involve the Moon;
for all other targets, their maxima are 0.000956″/day longitude and
0.005804″/day latitude (both Mercury). The distance maximum is Mercury.
No sampled longitude rate changed sign between engine and Swiss position
derivative, including the nearest-to-station sample found for each of Mercury
through Pluto. These samples do **not** locate or validate the exact station
time and cannot prove that sign never differs between grid points.

The reference's directly reported speeds are a separate comparison. On this
grid, Swiss's published radial speed and the finite derivative of **its own**
distances differ by up to **8.0923e-5 AU/day** (Pluto barycenter, 860 April).
Its published longitude speed differs from its own position derivative by up
to **0.2779″/day** (Moon, 940 January). Neither observation alone identifies
the cause or proves that all Swiss versions or modes behave the same way.
Matching a published speed is therefore not interchangeable with matching a
position derivative. The reported results are not a release tolerance.

To examine numerical sensitivity, the same fourth-order calculation was
repeated with half-steps 0.01 and 0.04 day. The maximum engine-vs-Swiss lunar
longitude difference was respectively 0.03655 and 0.01121″/day; the latitude
maximum was 0.04798 and 0.04879″/day. The Swiss Pluto radial-speed difference
above remained about 8.0923e-5 AU/day across these steps. Finite-step error,
single-JD precision and frame/model differences remain in the comparison;
the broad sampling does not supply a continuous bound. Historical UT1/ΔT,
solar-conjunction validity, exact stations and an independently agreed error
budget remain open.

## Reproduction and local evidence

After building the native binding, from `bindings/node` provide a reviewed
local classic DE441 and its SHA-256. Only the first command contacts Horizons;
both require a new output path. Swiss is locally supplied, not bundled.

```text
node scripts/compare-horizons-geometric-speeds.cjs <de441> <sha256> <new-report>
node scripts/compare-swiss-seasonal-speeds.cjs <de441> <sha256> <swisseph> <new-report> 0.02
```

Repeat the second command with `0.01` and `0.04` for the step-sensitivity
comparison. The Horizons script exits nonzero if its diagnostic limits are
exceeded, after retaining the complete report. Input DE441 SHA-256:
`476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
Local report SHA-256 values:

- Horizons geometric: `5bdf6e124e9b26d164ca05dc5cfae382d99e2d14e41861bcfef2f76c3168a7b5`.
- Swiss `h=0.01`: `4c416c1f700a06a453a2113677be6592e707264ba5197c126a2cf4e88daac3d5`.
- Swiss `h=0.02`: `129f2b95fe0259fa51136e810f383d81d5ae0354a35f0aea216ba6a52f4f596b`.
- Swiss `h=0.04`: `011e12676f659669569b140c214bb1e37b36f1fe0c415ff9d119cd1b5ec948ea`.

The Horizons report retains query URLs and raw responses locally. Hashes bind
bytes but do not authenticate source or guarantee data provenance. No kernel,
third-party implementation or raw external dataset is committed.
