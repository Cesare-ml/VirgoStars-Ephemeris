# Ancient ecliptic frame decomposition against Swiss

**Status: exploratory, sampled comparison, not a choice of physical model or
an MVP accuracy guarantee.** This extends the [fixed-axis planetary grid](PLANET_FRAME_GRID_13000BCE_3999_2026-11-18.md)
to isolate the contribution of the ecliptic convention, including epochs
before the Horizons DE441 service limit. Stars and server migration remain paused.

## Method

Use the previously recorded Swiss JPL/ICRS apparent directions at 5,480
body/date combinations: 69 baseline and 68 interleaved astronomical years,
four proleptic-Gregorian seasons, and Sun, Moon, Mercury, Venus and the
Mars–Pluto system barycenters. The baseline includes astronomical years
−12999 and 3999; 1,000 cases have year < −9998. Inputs are TDB and
approximately matching TT, not UT1 or historical local time. The two grids
are distinct, but use the same Swiss implementation and DE441 solution.

Recompute each Swiss fixed-axis direction and require the same values, flags,
version and file hashes as its source report. Query Swiss separately for
apparent geocentric equatorial and ecliptic coordinates of date, checking
JPL and coordinate flags and rejecting geometric, heliocentric, topocentric,
sidereal, no-nutation, no-aberration, no-deflection, Cartesian and radian
substitutions. Each output row retains the actual fixed/ecliptic/equatorial
reference flags. Project
the **same Swiss fixed-axis direction** through the local experimental
pole-based and mixed direct-obliquity ecliptic matrices and compare each with
Swiss's ecliptic direction. This frame-only comparison does not depend on the
local engine's planetary direction. In parallel compare the complete local
apparent directions against the same Swiss ecliptic direction.

The reference equatorial and ecliptic vectors also permit an *effective*
obliquity to be inferred assuming a shared equinox. Reject geometrically
ill-conditioned cases and report the X-coordinate mismatch as a check of
that assumption. This is an inferred convention from computed vectors, not a
direct measurement of the ancient Earth's obliquity. The code checks
same-axis angular separation invariance and triangle bounds with the numerical
sanity threshold described in the prior grid; that threshold is not a
scientific acceptance tolerance.

## Results

All separations are great-circle angular distances in arcseconds; maxima in
each column can occur at different dates and bodies.

| Grid | Cases | Frame-only: pole-based | Frame-only: direct obliquity | Full local position: pole-based | Full local position: direct |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 2,760 | 45.872135″ | 0.0002068″ | 45.872135″ | 0.002101″ |
| Interleaved | 2,720 | 44.222735″ | 0.0002048″ | 44.222735″ | 0.002232″ |

The baseline frame-only pole maximum is Neptune in year −12999 July; its
direct-frame maximum is Neptune in year −12999 January. Across 1,000
pre-Horizons cases the two frame-only maxima are the same baseline values.
For the baseline, the largest difference between the inferred Swiss obliquity
and the local pole-derived true obliquity is 45.8851″; versus the local
direct-obliquity alternative it is 0.0003881″. Across both grids all 5,480
obliquity inferences passed the geometric conditioning check. The largest
Swiss equator/ecliptic X mismatch was about 0.00000000036″, a numerical
consistency check only.

The frame-only result strengthens the interpretation that, **relative to
Swiss**, the two ecliptic conventions account for most of the large ancient
discrepancy. The direct-obliquity alternative remains a mixed convention and
matches this reference closely on these samples; that does not prove it is a
better physical model. [Horizons uses different of-date conventions](HORIZONS_FRAME_EPOCH_HOLDOUT_2026-11-18.md)
and yields different comparisons. Both references and the local calculation
share DE441; no independent orbital uncertainty is established. The maxima
are not bounds for unsampled dates, other bodies, historical ΔT, velocities,
events or topocentric calculations.

## Reproduction and evidence

Build the native Node addon and `diagnose_horizons_orientation_components`
example. Supply the classic DE441, the caller-installed Swiss package, one
previous fixed-ICRS grid and its hash, and a new output path:

```text
node bindings/node/scripts/audit-swiss-ancient-frame-decomposition.cjs KERNEL KERNEL_SHA SWISSEPH_PACKAGE FIXED_GRID_REPORT REPORT_SHA PROJECTOR NEW_REPORT
```

The script verifies kernel, input report, projector, addon and Swiss
package/wrapper/native-binary hashes before and after. It checks every source
epoch, fixed ICRS value and reference flag before computing the of-date
comparisons; any mismatch fails rather than falling back. Reports and external
files remain local and are not bundled.

- DE441 SHA-256: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
- Baseline fixed-ICRS source: `f055b695016353838a168ab32ca34ea99fcdc385e1e0166d3330ca7450eb2264`.
- Interleaved fixed-ICRS source: `7dd00e18c3a3935446237bd357a23532edd87fe107ffc7ac435dc77436db942c`.
- Baseline decomposition: `8315c9a1e21737db697a01f6413600433c2e58c331f907af6b6abc186852e5a1`.
- Interleaved decomposition: `8eb838d223a66023cbc7c11b1b7050b7fd154563e3dc23a156415e14c4c2ee62`.

Both decomposition runs were repeated byte-identically. Tests exercise flag
failures, invalid coordinates, a known synthetic obliquity rotation and the
ill-conditioned case. Future work remains: externally grounded long-term
orientation model selection, agreed positional tolerances, UT1/ΔT uncertainty,
independent orbit evidence and end-to-end consumer validation.
