# WGS84 geodetic observer to Earth-fixed Cartesian coordinates — 2026-11-11

**Status:** experimental, independent Rust core component proposed for
`dev/ephemeris`. Does **not** implement topocentric apparent planetary
positions, the IERS Earth-orientation pipeline or a Swiss API.

## Purpose and scope

The VirgoStars ephemeris backend needs a physical observer location for
topocentric directions, parallax, altitude/azimuth and event calculations.
The current experimental `first_sky.rs` internally constructs a
WGS84-like site vector already rotated by an approximate sidereal angle.
This PR separately implements **one strict, reusable Earth-fixed WGS84
geodetic → geocentric Cartesian transform**, so clients can validate
an observer position in a specified terrestrial frame independently
from the future time-dependent ITRS→GCRS/ICRF rotation.

This is a foundational *observer geometry* contribution, not
the final parallax or house calculation. It does not duplicate the
open ERA, GMST, LMST, horizontal-coordinate, altitude-crossing or
planetary-pipeline PRs.

## Model and units

WGS84 ellipsoid constants, from the World Geodetic System 1984:

- Equatorial (semi-major) radius `a = 6378137.0 m`.
- Inverse flattening `1/f = 298.257223563`.
- Squared eccentricity `e² = f(2-f)`.
- Prime vertical radius `N = a / sqrt(1 - e² sin²φ)`.

Given **geodetic latitude φ**, **east-positive longitude λ** and
ellipsoidal height `h`, the Earth-fixed position is:

```text
x = (N+h)             cosφ cosλ
y = (N+h)             cosφ sinλ
z = (N(1-e²)+h)       sinφ
```

**Input contract:**

- `longitude_rad_east`: finite **radians** in `[-π, +π]`.
- `latitude_rad_north`: finite **geodetic radians** in `[-π/2, +π/2]`.
- `height_m`: finite **WGS84 ellipsoidal height in meters**
  in `[-12000, +100000]`, matching the current experimental
  `first_sky::Observer` validation range. This is intentionally
  narrower than the mathematical range of the WGS84 formulas.
- Invalid/nonfinite fields return explicitly typed `Wgs84Error`.
  No silent longitude normalization or geoid conversion occurs.

**Output contract:** `Wgs84EcefPosition { x_m, y_m, z_m }`,
three finite **meters** in WGS84 Earth-fixed geocentric axes:
X is the equator/Greenwich meridian, Y is equator/90° east,
Z is the north terrestrial rotation axis.
`kilometers()` returns identical Earth-fixed coordinates in km.

Earth-fixed **ECEF** geometry must not be confused with an inertial,
J2000, GCRS, ICRF, geocentric apparent, or planetary ephemeris frame.
The input provides no reference epoch/realization or plate-motion
metadata. Neither time, Earth rotation (UT1), precession/nutation,
polar motion nor observer velocity is modeled. The output must
**not** be subtracted directly from a DE441 state before an explicitly
validated physical Earth-orientation transform and unit conversion.

Geodetic latitude is defined against the ellipsoid normal and generally
differs from geocentric latitude. Ellipsoidal height differs from
geoid/mean-sea-level or orthometric height; no geoid model is used.
At exact north/south poles, longitude is physically arbitrary; X/Y
may contain sub-nanometer floating-point residuals.

## Rust example

```rust
use virgostars_ephemeris_core::wgs84_ecef::wgs84_geodetic_to_ecef;

let site_ecef = wgs84_geodetic_to_ecef(
    12.5_f64.to_radians(),  // λ: longitude east, radians
    43.7_f64.to_radians(),  // φ: geodetic latitude north, radians
    200.0,                 // WGS84 ellipsoidal height, meters
)?;
let fixed_km = site_ecef.kilometers();
println!("Earth-fixed position [km]: {fixed_km:?}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Independent scientific reference and verification

The numerical oracle is independently published in the open-source
SOFA/ERFA test `t_gd2gc` (the **ERFA_WGS84** ellipsoid case).
Its input is `longitude=3.1 rad`, `geodetic_latitude=-0.5 rad`,
`ellipsoidal_height=2500.0 m`. Published reference output:

```text
x = -5599000.5577049947 m
y =   233011.67223479203 m
z = -3040909.4706983363 m
```

Each Cartesian component is tested to `1e-7 m`. The independent
ERFA fixture validates the ellipsoid and convention for this one
point, not the physical origin, reference epoch, or observer's
position in a celestial inertial frame.

Six further deterministic test groups cover equatorial/cardinal
meridians, ellipsoid polar radius and pole signs, change with height
along the geodetic normal, geodetic-vs-geocentric latitude,
meters→kilometers and height boundaries, and bad inputs. Those are
analytic invariants, **not** external astronomical-position parity.

Source references:
- [NGA WGS84 reference information](https://earth-info.nga.mil/index.php?dir=wgs84&action=wgs84)
- [SOFA/ERFA `t_gd2gc` published numeric fixture](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c)
- [SOFA geodetic/geocentric coordinate contract](https://juliaastro.org/SOFA/stable/api/geocentric/)

## Integration and acceptance gates

Existing read-only CI should execute:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

The Rust core can use the model in future IERS/EOP orientation,
topocentric and parallax modules, with the chosen reference realization,
epoch, geographical input type, height datum, units and accuracy
properly declared and independently tested.

No new runtime dependency, external WGS84/ERFA/SOFA source code, data,
JPL kernel, package publication or production change is added.
The source notice and CPAL-1.0 VirgoStars attribution are preserved.
