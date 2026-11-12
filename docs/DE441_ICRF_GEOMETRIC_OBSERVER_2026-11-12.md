# DE441 geometric Earth-centered ICRF observer sightline

**Date:** 2026-11-12. **Status:** experimental standalone stacked PR
onto the unmerged observer integration branch of PR #89.
No published package, `main` merge or production migration.

## Motivation and required frame boundary

The existing observer integration computes a WGS84 site position in
**geocentric GCRS** with explicitly supplied IERS polar motion,
UT1 ERA, and CIP/CIO celestial orientation parameters.

The experimental classic binary **DE441** reader supplies an
Earth-centered `StateVector` using
`ephemeris.state(target, Body::Earth, epoch_tdb)`. Its reported axes
are **DE441 equatorial ICRF**, with target and center metadata, an
epoch on the **TDB Julian-date** scale, geometric kilometers and
km/TDB day. DE441 planetary labels include **system barycenters**
where those are the available series (e.g., `JupiterBarycenter`).

**Do not assume that a GCRS observer vector from PR #89 is an
ICRF-aligned observer vector.** GCRS/ICRF origin, orientation, frame
bias, and time-scale assumptions must be reviewed and independently
validated before a rigorously physical use. A caller passing the
existing `ObserverGcrsPosition.position_gcrs_km` straight into this
API violates its contract. Mathematical coordinate subtraction can
only be meaningful after center, axes and epoch are aligned.

This increment keeps the frame boundary explicit rather than
silently adding a zero-rotation or `GCRS≈ICRF` fallback.

## New Rust API

```rust
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::de441_observer_geometry::
    de441_geometric_observer_sightline_icrf;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut eph = De441::open("/separately/provided/de441.eph")?;
let epoch_tdb = JulianDateTdb(2_451_545.0);
// This value MUST already be geocentric in matching DE441 ICRF axes.
// A GCRS WGS84 position must not be supplied without validated alignment.
let observer_geocentric_icrf_km = [100.0, 200.0, 300.0];
let sight = de441_geometric_observer_sightline_icrf(
    &mut eph,
    Body::Moon,
    epoch_tdb,
    observer_geocentric_icrf_km,
)?;
println!(
    "{:?} km, RA {:?} rad, Dec {} rad",
    sight.range_km,
    sight.right_ascension_icrf_rad,
    sight.declination_icrf_rad
);
# Ok(())
# }
```

The output type `De441IcrfObserverSightline` preserves:

- `target: Body` and `epoch_tdb: JulianDateTdb`: explicit
  identity and **TDB** coordinate epoch;
- `target_geocentric_icrf_km`: Earth-centered DE441 geometric
  target position in equatorial ICRF kilometers;
- `target_geocentric_velocity_icrf_km_per_tdb_day`: **target only**
  Earth-relative ICRF derivative (km/TDB day), **not** observer-relative;
- `observer_geocentric_icrf_km`: caller-supplied observer position
  in *identical* Earth-centered ICRF kilometer axes and epoch;
- `displacement_icrf_km = target - observer`: instantaneous
  geometric line-of-sight displacement in ICRF kilometers;
- `range_km`: Euclidean distance in kilometers;
- `unit_direction_icrf`: dimensionless unit vector;
- `right_ascension_icrf_rad: Option<f64>`: normalized to [0, 2π),
  `None` at an exact ±Z singularity;
- `declination_icrf_rad`: [-π/2,π/2], north-positive.

Errors distinguish Earth as an invalid target, a non-finite TDB
epoch, non-finite observer coordinates, underlying DE441 errors
(including missing center/target series or epoch outside coverage),
unrepresentable displacement/range and coincident target/observer.
No network requests, implicit DE441 data download, time-scale
conversion, target-center fallback, or physical correction occurs.

## What the independent tests actually validate

The integration tests construct a **fully synthetic, in-memory**
classic-DE441 binary record. A synthetic lunar Earth-relative vector
`[100,-50,25]` km is chosen to match a separate, independently
published SOFA/ERFA **`t_p2s`** Cartesian→spherical test:

```text
Input XYZ: 100, -50, 25
theta: -0.4636476090008061162 rad
phi:    0.2199879773954594463 rad
r:    114.5643923738960002
```

The new API normalizes negative theta into [0,2π) and checks range,
declination and unit-vector norm against the published oracle.

Eight further test groups cover changed observer ICRF position,
direct geocentric lunar series without EMB series,
Mercury target-vs-Earth center preservation, missing planetary
series without fallback, invalid/out-of-coverage TDB, rejected Earth
target/nonfinite observer, exact-pole undefined RA, coincident
observer and overflow range.

**This is NOT an independent JPL numerical parity test.** A
synthetic binary exercises the reader's arithmetic and metadata
contract but is not an official JPL DE441 kernel. The independent
SOFA/ERFA angular oracle validates elementary projection only.
Actual DE441 scientific accuracy still requires real NASA/JPL
kernel/Horizons comparisons with documented target/center, TDB
epoch, axes, units and tolerances.

## Future integration gate

Before combining this DE441 ICRF API with the WGS84→GCRS observer
pipeline in PR #89, implement and independently validate a specific
geocentric observer-to-DE441-axis model and an explicit TT/UT1/TDB
same-physical-epoch relationship. In a high-accuracy apparent
observation, the geometric simultaneous line of sight must later be
replaced or supplemented by a reception-time target solution,
observer velocity, annual/diurnal aberration and gravitational light
deflection, with separately tested assumptions.

The new source and integration test are original, carry CPAL-1.0
Exhibit A notices, preserve VirgoStars required attribution, and
include neither external SOFA/ERFA implementation code nor DE441
binary data.

**References:** [ERFA independent `t_p2s` fixture](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c),
[IERS Conventions (2010), chapter 5](https://iers-conventions.obspm.fr/conventions_material.php),
[Classic DE441 reader contract](DE441.md).
