# DE441 relative-state overflow guard — 2026-11-10

The classic DE441 reader already rejects non-finite coefficients and
interpolated series. This change also rejects non-finite final position or
velocity components after target/center subtraction and Earth/Moon
reconstruction, using `De441Error::InvalidFile`. It prevents a damaged or
extreme file from producing an apparently successful infinite result when
finite series overflow during vector arithmetic.

The regressions use synthetic binaries with individually finite barycentric
positions or velocities whose differences overflow. They do not validate astronomical
accuracy. Units and conventions remain unchanged: JD TDB, geometric ICRF,
km and km/day. This modification is to original VirgoStars-Ephemeris code
under CPAL-1.0 and adds no external data or dependencies.
