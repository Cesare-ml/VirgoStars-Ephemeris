# Horizons ecliptic stations and historical UT1 — 2026-11-17

**Status: sampled independent-implementation comparisons and illustrative
historical-time analysis; not a physical accuracy guarantee.** This continues
[the engine-only convergence check](ENGINE_STATION_CONVERGENCE_2026-11-17.md).
No production engine, API or activation changed.

## Independent orientation comparison near stations

For Mercury, Venus and Mars–Pluto system barycenters, we selected the nearest
two station directions at seven astronomical year seeds: 100, 500, 1000,
1500, 1900, 2000 and 2500. The resulting **112 events** are drawn from the
previous local station matrix, which uses 1 July seeds and searches ±500 days;
the actual event can be in an adjacent year. Dates are not a complete event
inventory. At each event we requested [JPL Horizons observer ecliptic-of-date
positions](https://ssd.jpl.nasa.gov/horizons/manual.html), quantity 31,
geocentric and airless, on approximately corresponding TT epochs. Body,
Earth center, DE441, output columns, seven decimal longitude places and TT
epochs were checked. The engine evaluates its apparent longitude rate in TDB.

Unlike the preceding ICRF projection check, Horizons calculates its **own**
ecliptic-of-date orientation rather than using the engine's matrix. The frame
models differ: Horizons documents an IAU76/80-based of-date convention and a
long-term precession change outside its modern range; VirgoStars uses its
experimental long-term precession and extrapolated nutation. Both use DE441,
so this is an independent calculation and orientation comparison, **not** an
independent orbital solution or observational truth.

We estimated Horizons longitude rate from five printed positions using a
fourth-order symmetric derivative. Around each engine station we also evaluated
the rate derived from Horizons positions **120 seconds before and after**.
The expected direction is positive-to-negative for entering retrograde and
negative-to-positive for leaving it. A Horizons sign is marked indeterminate
if its absolute estimated rate does not exceed the rounding allowance below.

| Derivative half-step | Both JPL signs as expected | Indeterminate | Unexpected |
| ---: | ---: | ---: | ---: |
| 0.25 day | 111 / 112 | 1 | 0 |
| 0.5 day | 112 / 112 | 0 | 0 |
| 1 day | 112 / 112 | 0 | 0 |

The indeterminate case is Pluto system barycenter, retrograde, near the year
100 seed. Engine signs were as expected at both endpoints for all 112 cases.
These are **sampled finite-difference signs**, not individually bracketed
exact JPL stations. They support the directions at those endpoints; they do
not prove uniqueness between endpoints, sub-second station times, or accuracy
outside the sampled grid. Different half-steps also introduce different
truncation errors; the 1-day stencil in particular gave a maximum rate
difference of 0.01555″/day, versus 0.00104″/day for 0.5 day.

Horizons prints these longitudes to seven decimal degrees. **If** those values
are rounded to nearest, the five-point rate formula has a worst-case rounding
allowance of `1.5 × 0.5e-7 × 3600 / h` arcseconds/day for half-step `h` in
days: 0.00108, 0.00054 and 0.00027″/day for the three steps. This is a bound
on the rate contribution of the stated rounding assumption only; it does not
include finite-difference truncation, frame/model error or time conversion.
Converting it to a time using the measured local curvature is only a
first-order sensitivity scale, not a station-time uncertainty bound. At
`h=0.5`, that scale reached about **36 seconds** for one Pluto case. The
single Newton time shifts retained in the report are likewise exploratory,
not independently located event times.

## The same dynamical events in historical UT1

We took 80 of the selected engine TT events at year seeds 100, 500, 1000,
1500 and 1900, and requested Horizons quantity 30 at those exact TT epochs.
For dates before 1962, Horizons documents the output as approximately
TT−UT1, without maintaining the very small TDB/TT distinction. We compared
that historical reconstruction with the existing experimental uncorrected
Espenak–Meeus estimate, evaluating the latter at an iterated proleptic
Gregorian UT1 year fraction; its existing 1900–1972 branch is retained.

For a fixed event in TT, `UT1 = TT − ΔT`. The following are **model UT1 minus
Horizons UT1**, not measurements of either model's error:

| Year seed | Range over 16 events |
| ---: | ---: |
| 100 | −156.0 to −155.6 s |
| 500 | −101.1 to −100.7 s |
| 1000 | +85.9 to +86.4 s |
| 1500 | +95.5 to +96.6 s |
| 1900 | +0.62 to +0.79 s |

The sign means that a larger ΔT places the same TT event earlier in UT1.
These two reconstructions may share historical observations: their separation
is **not** a confidence interval, error bound or evidence identifying the
better model. UT1 is not UTC, local civil time, a historical time zone, or a
recorded birth time. Any of those conversions needs its own information.

For scale only, the older [NASA discussion of the 2004 Morrison–Stephenson
standard-error model](https://eclipse.gsfc.nasa.gov/SEhelp/uncertainty2004.html)
gives an estimated `σ = 0.8 × ((year−1820)/100)²` seconds over approximately
1000 BCE–1200 CE. At the selected events near seeds 100, 500 and 1000, that
old formula gives approximately **236 s, 139 s and 54 s**, respectively.
If one explicitly assumes such a ΔT standard error at a fixed TT event, the
UT1 standard deviation has the same numerical size. These old illustrative
figures are not attached to the newer Horizons reconstruction, not validated
current uncertainties, and must not be combined with model differences as
if independent. No 2004 formula value is inferred for the 1500 or 1900 seeds.

## Reproduction and local evidence

After building the native Node binding, run from `bindings/node` with the
caller-owned classic DE441, its SHA-256 and the previous ignored station
matrix. All three scripts make explicit live Horizons requests, require new
output paths, and retain request URLs and raw responses only in local ignored
reports:

```text
node scripts/compare-horizons-ecliptic-stations.cjs <de441> <kernel-sha256> <matrix> <matrix-sha256> <new-ecliptic-report>
node scripts/compare-horizons-station-signs.cjs <de441> <kernel-sha256> <ecliptic-report> <report-sha256> <new-sign-report> 0.5
node scripts/compare-historical-station-ut1.cjs <de441> <kernel-sha256> <matrix> <matrix-sha256> <new-time-report>
```

Repeat the second command with 0.25 and 1 for the step-sensitivity comparison.
The sign script retains its report and exits nonzero if an unambiguous JPL
sign contradicts the selected direction, or if the engine signs do not match;
an indeterminate JPL sign is reported separately rather than called a pass.
Hashes of the retained local reports:

- Ecliptic: `4b82501d509d27b5575b8a2faa8b4733c5cea03b9113624338c266a11ce2a38d`.
- Signs, `h=0.25`: `457e1d20dd7248e70c5cf21f61c72022e232f39f6c872e05d69588a87184c800`.
- Signs, `h=0.5`: `6a69b41f33755682ef9d6bad96408a478571135485c3b5709f05bce7b86163f9`.
- Signs, `h=1`: `45c381d44a7268ccf74523cbdbcaf4d3c184049e68ba36af251b521222ec2b10`.
- Historical time: `6fc13ede11b2c929a092b8b73f88b483756b7f52591c68955d130d6b75dd306a`.

Hashes identify local bytes, not source authenticity. No JPL response or
kernel is committed. All error scopes and time/frame distinctions above are
part of the reproducibility contract.
