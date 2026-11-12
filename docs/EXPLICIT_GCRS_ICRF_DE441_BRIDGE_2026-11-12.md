# Explicit, experimental GCRS → DE441-ICRF observer adapter

**Date:** 2026-11-12. **Status:** original Rust, draft, not a validated physical frame transformation or production-ready ephemeris.

## Relationship to the existing PR stack

This feature is stacked on draft PR #92, which consumes an already
**geocentric DE441-ICRF** observer vector and computes an instantaneous
geometric line of sight to a DE441 Earth-relative target.

Draft PR #89 constructs an observer's **GCRS** position from WGS84 geodetic
coordinates, externally supplied polar-motion/CIP/CIO EOP parameters, and
a UT1 epoch. PR #92 did not mix these frame types because no model-based
GCRS/ICRF alignment or time-scale synchronization had been verified.

The new `observer_frame_bridge.rs` lets a caller explicitly opt into a
specific position-only transform, rather than silently assuming one:

```text
WGS84 location + UT1 + external EOP/CIP/CIO
       ↓ existing wgs84_observer_to_gcrs()
geocentric observer position in GCRS, km
       ↓ supplied proper 3x3 GCRS→adopted-DE441-ICRF matrix
geocentric observer position in declared ICRF axes, km
       ↓ existing de441_geometric_observer_sightline_icrf()
DE441 target relative to Body::Earth (classic binary at TDB epoch)
       ↓ explicit instantaneous target-minus-observer geometry
geometric fixed-ICRF RA, declination, range and unit direction
```

**The bridge's geometry is mathematically testable but its physically
correct GCRS→DE441 frame model is not provided by the library.**

## API

`wgs84_de441_geometric_sightline_with_alignment(&mut De441<R>, target, site,
orientation, epoch_tdb, alignment)`

Requires `ExplicitGcrsIcrfAlignment`:

- `rotation_gcrs_to_icrf`: **caller-supplied** row-major proper 3×3
  rotation, multiplying Cartesian column vectors. Finite, orthogonal
  right-handed (determinant +1) with 1e-10 numerical tolerance. No
  automatic zero rotation, frame bias or black-box IAU model.
- `tdb_minus_ut1_seconds`: caller-supplied **coordinate-time offset in
  seconds at the same physical epoch**. Finite. This is not a UTC, TT,
  TAI, TDB or UT1 conversion routine.
- `source_label`: required non-empty string for the calling
  application's provenance (e.g. independently validated
  reference/data/model identifier). **A string is not scientific
  proof of validity.** A plausible label is never accepted as a
  numerical accuracy claim.

The `orientation.ut1_epoch` must remain tagged `TimeScale::Ut1`
and the `epoch_tdb` must be a finite TDB Julian date. The code
checks their *supplied numeric relationship*:

```text
(TDB JD - UT1 JD) × 86400 seconds/day
       ≈ explicitly supplied TDB-minus-UT1 seconds
```

The comparison retains the split UT1 day/fraction and allows
**0.0001 seconds** of tolerance to account for the single-f64
TDB Julian-date representation near modern epochs. It does not
derive an offset, check EOP provenance, or establish that an offset
is astronomically correct. It can reject a mismatch between two
caller assertions, but cannot validate either assertion independently.

The output includes:
- the `ObserverGcrsPosition` with ITRS and GCRS coordinates and ERA;
- the externally rotated `observer_icrf_km`, not a full inertial state;
- the existing `De441IcrfObserverSightline` (target identity,
  TDB epoch, Earth-relative ICRF km/km-per-TDB-day target-only state,
  geometric RA/Dec/range/direction).

No observer velocity is inferred. A vector rotation does not
differentiate moving frames: dW/dt, dERA/dt, dC/dt, source station
motion, coordinate-time derivatives and relativistic conventions
remain separate physics tasks.

## Exact limitations

A proper Cartesian rotation is **not generally a complete
relativistic transformation between GCRS and BCRS/ICRF**. It does
not change the spatial origin (both positions here are intended
Earth-centered) and does not include velocity, geocenter
translation, relativistic scaling/metric or barycentric light-time.
The caller must independently establish whether a rotation-only
approximation is valid for its intended error budget. The input
matrix could be mathematically valid but astronomically wrong.

The DE441 reader remains **experimental classic DE441 binary**,
not NAIF SPK. Some outer-planet series are barycenters rather than
physical planet centers. Correct target/center and true target
ephemeris validation are still required. Never use this bridge to
claim Swiss Ephemeris precision, observed/apparent sky positions,
topocentric velocity, Horizons parity or production correctness.

## Synthetic offline tests

Eight test groups build a minimal, entirely synthetic classic
DE441 binary, with a constant geocentric Moon vector and zero
velocity. They exercise:

1. Full existing WGS84/GCRS and DE441 reader path with a **non-identity**
   explicitly supplied analytic +Z rotation and geometric sightline.
2. Time consistency: a wrong offset or TDB epoch is rejected.
3. Empty or whitespace-only provenance labels are rejected.
4. Reflections, shears and non-finite rotations are rejected.
5. Non-finite TDB offsets and non-finite JD values are rejected.
6. TT-labeled UT1 is rejected by the existing strict ERA primitive.
7. Missing DE441 planet series are propagated, not silently replaced.
8. Deliberately explicit synthetic identity rotation works only when
   *chosen by the caller*, not as an implicit default.

**These synthetic fixtures are not independent JPL numeric validation,
and their GCRS/ICRF alignment is not a scientific model.**

## Rust usage

```rust
use virgostars_ephemeris_core::observer_frame_bridge::{
    ExplicitGcrsIcrfAlignment, wgs84_de441_geometric_sightline_with_alignment
};
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb};

// Assume `ephemeris`, `site` and `orientation` were prepared by
// the caller from independently supported scientific reference data.
// Never substitute the synthetic identity below for a validated model.
# fn example<R: std::io::Read + std::io::Seek>(
#   ephemeris: &mut virgostars_ephemeris_core::de441::De441<R>,
#   site: virgostars_ephemeris_core::observer_gcrs::Wgs84ObserverSite,
#   orientation: virgostars_ephemeris_core::observer_gcrs::ObserverOrientationInputs,
# ) -> Result<(), Box<dyn std::error::Error>> {
let validated_external_matrix = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];
let explicit_external_offset_seconds = 69.184;
let alignment = ExplicitGcrsIcrfAlignment {
    rotation_gcrs_to_icrf: validated_external_matrix,
    tdb_minus_ut1_seconds: explicit_external_offset_seconds,
    source_label: "caller-provided-reference; example only",
};
let _result = wgs84_de441_geometric_sightline_with_alignment(
    ephemeris, Body::Moon, site, orientation,
    JulianDateTdb(2_451_545.0), alignment
)?;
# Ok(())
# }
```

This snippet **does not** assert that the identity rotation or
illustrative 69.184-second offset are valid at any actual date;
they are placeholders for externally verified inputs and will be
rejected if the declared UT1/TDB numerical relationship disagrees.

## Required next scientific gates

1. Find and independently verify numerical GCRS/ICRF alignment
   conventions and the actual accuracy of any adopted rotation
   (and whether the full relativistic transformation is needed).
2. Supply verified same-physical-epoch UTC/UT1, TT, TDB, xp, yp,
   CIP X/Y and CIO s data using recognized IERS standards.
3. Validate real observer GCRS positions and DE441 target/center
   vectors against independent SOFA/ERFA, JPL/Horizons data,
   not a self-derived synthetic reference.
4. Then add emission light-time and observer velocities,
   aberration, relativistic deflection, and apparent sky corrections.

**License:** the new code/tests preserve CPAL-1.0 Exhibit A headers
and required VirgoStars attribution. No external binary kernels,
data, dependencies, deployment or source-code translation.

**References:** [IERS Conventions (2010)](https://iers-conventions.obspm.fr/conventions_material.php);
[SOFA/ERFA numerical references](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c);
[DE441 classic reader contract](DE441.md).
