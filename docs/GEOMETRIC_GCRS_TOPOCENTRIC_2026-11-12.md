# Geometric GCRS topocentric sightline — integration note (2026-11-12)

**Status:** experimental, review-only on the unmerged
`integration/observer-itrs-gcrs-20261112` branch, PR #89.
No changes to VirgoStars production or `main`.

## New capability

Two independently testable Rust functions are added to the
integrated observer pipeline:

1. `topocentric_geometry::geometric_topocentric_direction_gcrs(target, observer)`:
   subtracts an already **Earth-centered GCRS target** vector and an
   already **GCRS observer** vector in **kilometers**.
2. `topocentric_geometry::wgs84_topocentric_geometric_direction_gcrs(site, orientation, target)`:
   first calls the existing explicit-input
   `observer_gcrs::wgs84_observer_to_gcrs` WGS84→ITRS→TIRS→CIRS→GCRS
   site-position transform; then calls the exact same target-minus-site
   geometry. It returns both the computed observer and sightline.

The geometric outputs are:
- `displacement_gcrs_km = target_gcrs_km - observer_gcrs_km` (km);
- `distance_km` (km), a geometric observer–target separation;
- `unit_direction_gcrs` (dimensionless, length 1);
- `right_ascension_rad` in **[0, 2π)**, or `None` on the
  **exact GCRS north/south celestial axis** where RA is undefined;
- `declination_rad` in **[-π/2,π/2]**.

The **right ascension is in the supplied GCRS coordinate axes**;
it is not an RA of date in a true/equinox-of-date system.
The caller must not label this geometric GCRS vector `apparent`.

## Strict input and physical model contract

`geocentric_target_gcrs_km` and `observer_gcrs_km` must be:
- finite Cartesian positions in **kilometers**, relative to the
  *same Earth-centered GCRS origin*;
- defined in the *same GCRS axis realization and physical epoch*;
- geometrically simultaneous positions. Applying true reception
  light-time changes which target epoch must be evaluated and is
  **not** performed here.

Passing raw DE441 barycentric/ICRF states, a barycenter instead of
a planet center, an Earth-relative vector with an inconsistent epoch,
an ecliptic-of-date vector or a heliocentric vector is **wrong**.
The original classic-DE441 reader and its center/target handling
remain separately unvalidated against NASA/JPL kernels.

For the higher-level WGS84 wrapper:
- `Wgs84ObserverSite` requires geodetic longitude, latitude in
  **radians** and WGS84 **ellipsoidal** height in meters.
- `ObserverOrientationInputs` requires an explicitly tagged **UT1**
  split Julian date, polar motion xp/yp, TIO locator s′, CIP X/Y
  (dimensionless) and CIO locator s in radians. The latter quantities
  are externally supplied for the same physical epoch, with CIP/CIO
  modeled at the appropriate **TT** epoch.
- No EOP provider, UTC↔UT1/TT conversion, CIO/CIP calculation or
  undocumented zero-value fallback exists in the wrapper.
- Input validation is fail-closed: invalid target/observer coordinates,
  subtraction overflow, coincident target and observer or
  unrepresentable norm are explicit typed errors; at exact poles only
  the RA is `None`, not the entire direction.

## Physical correction exclusions

The implementation includes **only Euclidean line-of-sight
displacement and geometric angular projection**. It does **not**
include:
- reception/emission light-time solution, observer/target velocity,
  physical gravitational light deflection or stellar aberration;
- an observed/apparent equatorial direction, ecliptic-of-date or
  precession/nutation axis conversion beyond the externally supplied
  GCRS orientation inputs;
- refraction, observer horizon dip, topographic horizon,
  semidiameter, radial velocity, rise/set event time or astrological
  houses;
- JPL/Horizons scientific numerical parity or production Swiss
  Ephemeris compatibility.

The difference between the target's *geocentric* direction and
this *topocentric geometric* direction is a geometric parallax
effect. The code does not claim to model an actual Moon or Sun
observation just from the synthetic distances in its tests.

## Independent mathematical test reference

The published [SOFA/ERFA `t_p2s` numerical oracle](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c)
uses the Cartesian input `[100,-50,25]` and expects the following
spherical direction/length:

```text
signed theta = -0.4636476090008061162 rad
latitude     =  0.2199879773954594463 rad
length       =114.5643923738960002 (units of the input vector)
```

The Rust test uses a target at this exact vector and a zero
observer position, compares RA after normalization to
`2π + theta`, declination and distance within declared
floating-point tolerances, and checks the unit vector.

Eight further integration/analytic groups exercise:
- lunar-distance-scale parallax `atan2(6378.137, 384400)`,
  approximately 0.95°, using an intentionally **synthetic** target;
- an explicitly synthetic 1-AU geometry producing approximately
  8.8 arcseconds of angular shift;
- all four cardinal GCRS RA directions and wrap convention;
- exact ±Z celestial axes with undefined RA but valid declination;
- translation invariance of the observer–target displacement;
- non-finite, coincident and overflow cases;
- the full WGS84/UT1/EOP/CIP/CIO wrapper followed by geometry,
  without pretending that the test's zero EOP/CIP parameters
  are physically valid;
- rejection of a TT-labeled epoch and invalid target in the wrapper.

These fixtures test vector conventions and numerical behavior,
**not real-world physical positions, velocities or apparent
observation accuracy**.

## Rust example

```rust
use virgostars_ephemeris_core::topocentric_geometry::
    geometric_topocentric_direction_gcrs;

// Purely geometric test vectors, not DE441 data.
let target_gcrs_km = [100.0, -50.0, 25.0];
let observer_gcrs_km = [0.0; 3];
let result = geometric_topocentric_direction_gcrs(
    target_gcrs_km,
    observer_gcrs_km,
)?;
println!(
    "Geometric sightline: RA={:?} rad, declination={} rad, distance={} km",
    result.right_ascension_rad,
    result.declination_rad,
    result.distance_km
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Validation and next gates

Run existing authorized **read-only** CI:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

The next scientific increment must supply independently verified,
Earth-centered **GCRS geometric target positions at matched epochs**
from DE441 and a correctly modeled geocenter; it must explicitly
resolve the BCRS/ICRF-vs-GCRS frame/origin contract before feeding
planetary states into this API. Then implement a separately verified
reception-time target/observer solution, rates and apparent corrections.

The original Rust implementation and test source preserve CPAL-1.0
Exhibit A notices, required VirgoStars project attribution and
existing dependencies. No JPL kernels, external SOFA/ERFA source
implementation, package publication, automatic EOP download,
release or production migration is added.

**References:** [IERS Conventions (2010), chapter 5](https://iers-conventions.obspm.fr/conventions_material.php);
[ERFA spherical-vector regression `t_p2s`](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
