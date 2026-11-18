# Planetary positions and ancient frame decomposition

Follow-up: [300 Horizons comparisons at new dates](HORIZONS_PLANET_HOLDOUT_2026-11-18.md).

Follow-up: [Swiss-based ancient frame decomposition](SWISS_ANCIENT_FRAME_DECOMPOSITION_2026-11-18.md)
extends the frame-only comparison into the pre-Horizons portion of the range.

**Status: exploratory sampled validation, not MVP acceptance or a continuous
accuracy guarantee.** The stellar catalog and main-server migration remain
paused. These checks compare apparent geocentric directions, not speeds,
station times, topocentric events or historical civil time.

All ten targets are Sun, Moon, Mercury, Venus and the Mars–Pluto **system
barycenters**. Calculations include light-time, solar deflection and annual
aberration; there is no atmospheric refraction. Dates use a proleptic Gregorian
calendar and astronomical year numbering (−12999 = 13000 BCE). All local
results use the caller-supplied classic DE441. Swiss and Horizons also use
DE441: their implementations differ, but these comparisons do not independently
validate the underlying planetary orbit solution or observations.

## Fixed-axis position checks

For Horizons, use 60 fixed-TT epochs (15 years, four seasons) from the
[frame study](HORIZONS_FRAME_EPOCH_HOLDOUT_2026-11-18.md), approximately
convert TT to TDB, and compare apparent ICRF unit directions for all ten
bodies. Raw Horizons response metadata and every epoch are checked. Six
targets reuse the stored frame-study responses; four are newly requested.
The same 600 rows were reproduced offline by re-parsing hash-checked raw
responses. Horizons cannot check the earliest part of the requested interval.

For Swiss, compare at fixed TDB with approximately matching TT, explicitly
requesting and checking JPL, equatorial, J2000 and ICRS flags on every call.
The local installation reports Swiss 2.09.03; no Swiss code is copied or
distributed. The baseline samples 69 years, including −12999 and 3999; an
interleaved grid samples 68 different years. Each year uses January, April,
July and October and all ten bodies. In particular, the two grids contain
1,000 body/date cases with astronomical year < −9998, a conservative subset
earlier than the Horizons DE441 service limit.

| Comparison | Cases | Largest Moon separation | Largest separation among other bodies |
| --- | ---: | ---: | ---: |
| Horizons ICRF, years −9997 to 3999 | 600 | 0.000474″, year −7000 October | 0.00000517″, Mercury, year −9997 October |
| Swiss ICRS baseline, years −12999 to 3999 | 2,760 | 0.001966″, year 3000 October | 0.0001433″, Mercury, year −4000 January |
| Swiss ICRS interleaved | 2,720 | 0.002097″, year 1375 October | 0.0001437″, Mercury, year −3875 January |

Across the 1,000 pre-Horizons cases, the largest Swiss separation is
0.001943″ for the Moon in year −10375 April. At the two calendar endpoint
years, the largest sampled separation is 0.001622″ for the Moon at the start
of year −12999. These are observed differences for the tested instants, not
approved tolerances, error bounds between samples, or a physical uncertainty.
The two references need not agree with each other: time conversion,
correction models, conventions and numerical precision can differ.

## Separating position from orientation

For each Horizons sample, project the **Horizons ICRF direction** through the
local pole-based and mixed direct-obliquity ecliptic matrices. Compare those
directions with the engine's own apparent directions in the corresponding
local ecliptic. This holds the axes approximately fixed while varying the
underlying direction; it is an internal consistency decomposition and reuses
the local orientation, not an independent validation of that orientation.

The same-axis separations agree with the fixed-ICRF separations to within
0.000000000354″ in these cases. This numerical invariance check, and a
triangle-inequality check, must pass with a generous 0.0000001″ numerical
sanity threshold; that threshold is **not** an accuracy specification.

By contrast, comparing the full local ecliptic positions directly with
Horizons ecliptic-of-date positions gives sampled maxima of about **211.035″**
for the pole-based path and **213.828″** for the mixed-obliquity path, both
near year −9997. The corresponding projections of Horizons directions alone
show essentially the same large frame contribution. This supports the
interpretation that, in these samples, most of that comparison is due to
different orientation conventions. It does not establish which ancient frame
is physically correct, nor justify applying a fitted correction in production.

## Reproduction and evidence

Build the Node addon and `diagnose_horizons_orientation_components` example.
Use a caller-supplied kernel, caller-installed Swiss package and new output
paths. The Horizons command requests the four missing bodies unless the
optional prior planet report and hash are supplied for an offline replay.

```text
node bindings/node/scripts/validate-horizons-planet-frame-grid.cjs KERNEL KERNEL_SHA FRAME_REPORT FRAME_SHA PROJECTOR NEW_REPORT [PRIOR_PLANET_REPORT PRIOR_SHA]
node bindings/node/scripts/validate-swiss-fixed-icrs-grid.cjs KERNEL KERNEL_SHA SWISSEPH_PACKAGE NEW_REPORT baseline
node bindings/node/scripts/validate-swiss-fixed-icrs-grid.cjs KERNEL KERNEL_SHA SWISSEPH_PACKAGE NEW_REPORT interleaved
```

Input files, native addon, projector and Swiss package metadata/wrapper/native
binary are hashed before and after as applicable. Swiss's requested flags are
verified; an unavailable JPL/ICRS result fails rather than silently falling
back. Reports are local, ignored and not distributed with the repository.

- DE441: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
- Horizons frame source: `e89a316982636771ae817afebd2a603a635efd70041f7ee305617830b285f688`.
- Horizons initial report: `dec009c1c385130d703f665e5048950c4e867aca838004bd268c108e1f5ab3f6`.
- Horizons checked offline replay: `fb0008a6d6935c427062521658c58693eca75acaa28fdf99e7df164728b83d31`.
- Swiss baseline: `f055b695016353838a168ab32ca34ea99fcdc385e1e0166d3330ca7450eb2264`; a separate replay was byte-identical.
- Swiss interleaved: `7dd00e18c3a3935446237bd357a23532edd87fe107ffc7ac435dc77436db942c`; its separate replay was also byte-identical.

Long-range nutation and precession model selection, independent orbital
evidence, historical ΔT policy and uncertainty, accepted tolerances, and
consumer end-to-end coverage remain open before an MVP release can claim the
requested interval.
