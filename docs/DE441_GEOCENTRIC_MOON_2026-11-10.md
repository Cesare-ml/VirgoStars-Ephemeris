# Direct geometric Earth–Moon vectors — 2026-11-10

The classic DE441 reader now returns the geocentric Moon coefficient series
directly for Moon relative to Earth, and its negation for Earth relative to
Moon. Previously it reconstructed two solar-system-barycentric states and
subtracted them, unnecessarily losing precision when the common barycentric
offset was much larger than their difference. Other target/center combinations
continue to use barycentric states.

Dates remain Julian Date TDB; coordinates remain geometric equatorial ICRF;
position and velocity remain km and km/day. The direct path requires the
geocentric Moon series, but no longer requires the Earth–Moon barycenter
series solely to obtain a Moon/Earth relative state. Barycentric states still
require both. Missing or invalid Moon series, I/O failures and coverage errors
remain explicit errors.

Synthetic tests demonstrate loss of a small lunar displacement when an
artificially large common barycenter is subtracted; they also cover direction,
velocity and missing series. The local external DE441 file is tested
separately. Neither is independent scientific validation against JPL Horizons.
See [JPL planetary ephemeris export information](https://ssd.jpl.nasa.gov/planets/eph_export.html).

This modification is to the original VirgoStars-Ephemeris source under
CPAL-1.0. No external data, code or dependencies are bundled.
