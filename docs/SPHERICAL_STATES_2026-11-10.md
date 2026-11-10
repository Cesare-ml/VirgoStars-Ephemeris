# Geometric spherical state projection — 2026-11-10

`coordinates::project_fixed_axes` converts a classic DE441 geometric ICRF
state into longitude, latitude, distance and their rates. A zero tilt uses the
original ICRF axes. A caller may explicitly provide a fixed rotation around
the X axis, e.g. an adopted obliquity for an approximate fixed ecliptic.
The output retains target, center, JD TDB epoch and the supplied tilt. It uses
radians, km, radians/day and km/day. Longitude is normalized to [0, 2π).

```rust
use virgostars_ephemeris_core::coordinates::project_fixed_axes;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

let mut ephemeris = De441::open("/path/to/de441.eph")?;
let vector = ephemeris.state(Body::Moon, Body::Earth, JulianDateTdb(2451545.0))?;
let coordinates = project_fixed_axes(&vector, 23.4392911_f64.to_radians())?;
println!("longitude: {} degrees", coordinates.longitude_rad.to_degrees());
# Ok::<(), Box<dyn std::error::Error>>(())
```

The example's tilt is an explicitly chosen fixed approximation; the input
epoch is already TDB, and the result is not an apparent coordinate.

This is a **fixed-axis geometric projection**. It does not supply a choice of
ecliptic epoch or implement frame bias, precession, nutation, light-time,
aberration or gravitational deflection. An arbitrary obliquity alone does not
define a validated ecliptic of date. Velocity is the derivative in fixed axes:
do not use it as the rate in a time-dependent frame. Inputs with non-finite
values, numerically unrepresentable outputs, zero radius or a pole return an
explicit error; longitude and longitude speed are undefined at a pole.

Tests cover the fixed-axis derivative, orientation, wrapping, singularities
and metadata. A numerical position/velocity fixture is checked against ERFA's
`t_pv2s` reference values; attribution and license are in
[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md). This checks the projection,
not the astronomical source vectors or final apparent positions. The optional
external DE441 integration test uses `DE441_PATH` and does not bundle a kernel.

CPAL-1.0 applies to the original VirgoStars source; the ERFA reference data has
its separately documented terms. Modification date: 2026-11-10.
