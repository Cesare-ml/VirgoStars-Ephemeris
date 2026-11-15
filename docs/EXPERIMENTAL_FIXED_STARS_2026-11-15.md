# Experimental fixed-star inputs and calculation

The experimental Node subset accepts an **explicit caller-supplied catalog**
and exposes `swe_fixstar_ut`- and `swe_fixstar_mag`-shaped methods. No star
catalog or Swiss data is bundled. The caller must establish the catalog's
source, rights, epoch and units; the supplied catalog ID is a label, not proof
of provenance. This is not yet the vetted distributed catalog required by the
MVP and is not a complete Swiss fixed-star replacement.

Each record requires a unique trimmed name, an explicit alias array, ICRS
right ascension and declination in degrees at an explicit TDB Julian epoch,
`properMotionRaCosDecMasPerYear` (μα* = dα/dt × cos δ), declination proper
motion in mas/Julian year, positive parallax in mas, barycentric radial
velocity in km/s and a visual magnitude. A Julian year is 365.25 days. The
catalog and all aliases are snapshotted at construction; lookup is exact apart
from case. Ambiguous duplicate names or aliases, missing values, celestial
poles and parallaxes outside [0.001, 10000] mas are rejected. Zero, negative,
missing or uncertain parallax is never replaced with an invented distance.

```js
const swe = createExperimentalSwissSubset({
  executablePath, kernelPath, convertUt1ToTtTdb,
  catalog: {id:'caller-supplied-catalog-v1', records:[{
    name:'Example', aliases:['Example alias'], epochTdbJulianDay,
    rightAscensionDeg, declinationDeg, properMotionRaCosDecMasPerYear,
    properMotionDecMasPerYear, parallaxMas, radialVelocityKmPerSecond,
    visualMagnitude,
  }]},
});
const magnitude = swe.swe_fixstar_mag('Example').magnitude;
const position = swe.swe_fixstar_ut('Example',ut1JulianDay,swe.SEFLG_JPLEPH|swe.SEFLG_SPEED);
```

The Rust core propagates the barycentric position with **constant rectilinear
motion**, subtracts Earth's DE441 barycentric position for annual parallax,
applies special-relativistic annual aberration and rotates to the IAU
2006/2000B true ecliptic of date. TT and TDB are explicit. Speeds use centered
differences at ±0.01 TDB day, advancing TT at the same rate. Distance is the
model's geometric AU range; numerical distance rates can lose precision at
stellar distances. Reported magnitude is the unchanged catalog value, not a
computed apparent brightness or corrected photometric measurement.

The calculation omits gravitational light deflection, the stellar Roemer and
other light-time/relativistic catalog effects, nonlinear stellar motion,
topocentric corrections and complete nutation. It is not a certified apparent
place and not Swiss parity; near the Sun, for long propagation intervals or
uncertain radial velocities the error can be materially larger. Approximation
labels and warnings are returned. See the [USNO NOVAS guide](https://aa.usno.navy.mil/downloads/novas/NOVAS_C3.1_Guide.pdf)
for the scope of a full apparent stellar place and catalog epoch conventions.

Tests include analytic motion/parallax cases, a comparison of the geometric
direction with a published ERFA `pmpx` fixture to 1 nanoradian (with the
omitted Roemer term acknowledged), invalid inputs, aliases and magnitude
snapshotting, and a synthetic catalog through a real DE441 executable. The
stellar catalog records in the integration test are fictional. No independent
catalog-to-observed-star validation or scientific accuracy bound is claimed.

## Independent near-Sun pipeline check

A second optional test uses the synthetic input and date of ERFA's published
[`t_atci13` fixture](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c):
ICRS RA 2.71 rad, declination 0.174 rad, catalog-epoch J2000, raw RA and
declination proper motions 1e-5 and 5e-6 rad/Julian year, parallax 0.1 arcsec,
radial velocity 55 km/s and TT JD 2456165.901182685. The library interface
converts raw RA motion to μα* by multiplying by cos(declination). For this
fixture only, TDB is approximated as TT; no general time conversion is implied.

The full published ERFA CIRS output, converted to equinox-based RA by
subtracting its equation of origins, includes solar gravitational deflection.
The fixture happens to be very close to the Sun: this implementation's output
differs from the complete ERFA reference by approximately **1.57 arcseconds**.
That is a concrete example of why omitted deflection matters; it is not an
error bound or observed-star comparison. To isolate that omission, an
independent no-solar-deflection reference was generated with pyerfa 2.0.1.5:
`apci13` (Earth astrometry), `pmpx` (catalog motion and parallax), `ab`
(annual aberration), then `pnm06a` (true equator/equinox), deliberately
omitting `ldsun`. The resulting RA/declination in radians are
`2.7130228830832768` and `0.17294471809826534`. The implementation using
DE441, IAU 2000B and no solar bending differs by about 0.4 milliarcseconds;
the regression tolerance is 10 milliarcseconds to allow the differing Earth
ephemeris, nutation, aberration and time approximations. ERFA's annual
aberration includes a small solar-potential term not included in our model.
This isolated numerical agreement is not full apparent-place validation.

Run `cargo test --lib erfa_pipeline_exposes --locked -- --include-ignored`
with `DE441_PATH` set to a separately obtained classic DE441 file. ERFA is
only an external reference generator and is not added as a runtime dependency.
