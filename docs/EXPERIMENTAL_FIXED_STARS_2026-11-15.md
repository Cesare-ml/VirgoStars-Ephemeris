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
