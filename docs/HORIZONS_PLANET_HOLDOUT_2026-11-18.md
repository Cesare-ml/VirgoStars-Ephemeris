# New-date Horizons holdout for ancient planetary directions

**Status: exploratory implementation comparison, not MVP acceptance or an
accuracy guarantee.** This extends the [existing planetary frame grid](PLANET_FRAME_GRID_13000BCE_3999_2026-11-18.md)
with new dates. The stellar catalog and main-server migration remain paused.

## Method

Query the [JPL Horizons observer API](https://ssd-api.jpl.nasa.gov/doc/horizons.html)
for the Sun, Moon, Mercury, Venus and Mars–Pluto **system barycenters** at
30 TT instants, yielding 300 body/date cases. Use February and August 17 at
06:00 TT in astronomical years −9950, −9250, −8750, −7750, −6750, −5750,
−4750, −3750, −2750, −1750, −750, 250, 1250, 2750 and 3975. These instants
are disjoint from the earlier 60-epoch Horizons grid; dates use the proleptic
Gregorian calendar and astronomical year numbering.

For each target compare local and Horizons apparent geocentric airless ICRF
directions. The local classic DE441 engine receives an approximately converted
TDB instant; Horizons receives explicit TT. This approximation, output rounding
and correction conventions can contribute to the differences. Both solutions
use DE441, so the external implementation is a useful comparison but does
**not** independently validate the underlying orbit solution or historical
Earth orientation. It says nothing about UT1 or historical civil time.

Also compare local apparent ecliptic-of-date directions with the Horizons
ecliptic-of-date result, using both experimental local ecliptic conventions.
Project the same Horizons ICRF direction into the local axes separately. This
isolates the frame contribution in the comparison; it does not determine which
orientation convention represents the historical Earth better. The invariant
fixed-axis versus same-frame separation and triangle inequality are checked
with a `1e-7″` numerical sanity threshold, not a scientific acceptance limit.

## Results

| Quantity | Largest sampled separation | Case |
| --- | ---: | --- |
| ICRF, Moon | 0.000524″ | year −8750, August |
| ICRF, other bodies | 0.00000418″ | Mercury, year −7750, August |
| Full ecliptic, local pole-based frame | 211.233″ | Sun, year −9950, February |
| Full ecliptic, local direct-obliquity frame | 213.947″ | Jupiter barycenter, year −9950, August |

The frame-only maxima for the same two conventions are approximately
211.233″ and 213.947″, respectively. In these samples the cross-convention
frame contribution explains most of the large absolute ecliptic differences;
the ICRF comparisons do not establish an accuracy bound for those axes. The
largest fixed-axis versus same-frame separation mismatch is `2.61e-10″`, a
numerical consistency result, not an astronomical error estimate.

These are sampled great-circle angular separations, not individual longitude
errors. They are not limits for unsampled dates or other geometries, and they
do not validate speeds, station times, topocentric events, historical ΔT or
the range before the Horizons DE441 service limit. No model selection,
automatic correction or production integration follows from this comparison.

## Reproduction

Build the Node addon and `diagnose_horizons_orientation_components`. Supply
the classic DE441 file and a new output path. The first run queries Horizons;
the second form re-parses raw responses from the hash-checked previous report
without network access. Target, DE441 source, Earth center, TT epochs,
coordinate columns and airless status are checked for every response. Kernel,
projector, addon and prior report hashes are checked before and after. The
outputs, external kernel and Horizons responses remain locally retained and
untracked.

```text
node bindings/node/scripts/validate-horizons-planet-holdout.cjs KERNEL KERNEL_SHA PROJECTOR NEW_REPORT
node bindings/node/scripts/validate-horizons-planet-holdout.cjs KERNEL KERNEL_SHA PROJECTOR NEW_REPLAY PRIOR_REPORT PRIOR_SHA
```

- DE441 SHA-256: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
- Initial report SHA-256: `0279176fbb01cad2ca9c47ecf5345b8e964d88a9e2929b2b93a17bbb9395d885`.
- Offline replay SHA-256: `f590873f1b0b084e196bc6037117e7886bc661fcdc86d671f812bb7ea6fae63a`.

All 300 result rows and summaries were identical on offline replay. The full
report hashes differ because the replay records the initial report hash.
