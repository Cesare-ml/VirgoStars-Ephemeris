# First time-and-place planet positions — 2026-11-10

This is an **experimental first working geometric sky**, not a Swiss
Ephemeris-compatible or production-validated result. It takes a classic
little-endian JPL DE441 binary supplied by the user, a date/time with an
explicit numeric UTC offset, and a WGS84 location. No data is downloaded or
included in the repository.

Run from the repository root (PowerShell example):

```powershell
cargo run -p virgostars-ephemeris-core --example planets_at --locked -- `
  'D:\GitHub\virgo-stars\database\effemeridi\astrodienst\de441.eph' `
  '2026-11-10T13:30:00+01:00' 45.4642 9.19 120
```

Arguments are: DE441 file; timestamp with `Z` or `±HH:MM`; geodetic latitude
in degrees north; geodetic longitude in degrees east; ellipsoid height in
meters. The example is Milan at 13:30 UTC+01:00. A numeric offset is mandatory:
the tool does not guess a time zone or daylight-saving rules from the location.
It rejects leap-second labels (`:60`) rather than silently interpreting them.

The Rust API is `first_sky::calculate(&mut De441, CivilTimestamp, Observer)`;
the CLI prints the Sun, Moon, Mercury, Venus and the Mars–Pluto **system
barycenters**, their mean ecliptic longitude/latitude of date, altitude,
azimuth east from north, and topocentric distance. The API also provides mean
equatorial right ascension/declination and a machine-readable list of model
approximations. Geocentric versus topocentric and
planet-center versus system-barycenter distinctions are material: Pluto and
the outer planet barycenters must not be silently labeled exact body centers.

## Scientific contract and limitations

- UTC is normalized from the explicit offset; TAI–UTC uses IERS Bulletin C 72
  values. This embedded table is accepted from **1972-01-01 UTC until
  2027-06-28 UTC exclusive**, its published expiry; other dates return an error.
  The table and expiry must be updated from a later IERS bulletin before use
  beyond that date. UTC leap-second labels are not yet supported.
- TT is TAI + 32.184 s. TDB–TT uses the seven-term approximation in USNO
  Circular 179. DE441 still receives a single-f64 Julian date, with associated
  sub-millisecond quantization.
- IAU 2006 Fukushima–Williams bias/precession transforms ICRF vectors to mean
  equator/equinox of date; mean obliquity rotates to mean ecliptic of date.
  UT1 is explicitly approximated as UTC for the USNO GMST approximation.
  Observer coordinates use WGS84 and no polar motion. Inputs accept latitude
  ±90°, longitude ±180°, and height from -12000 to 100000 meters.
- All vectors are **geometric at the observation epoch**. There is no
  light-time, aberration, gravitational deflection, nutation or refraction.
  There are no date-frame velocities, lunar nodes, houses, sidereal zodiac,
  fixed stars or other Swiss functions in this slice. Azimuth is undefined
  within 1e-14 radians of zenith/nadir and is returned as `None`.
- Expect differences from apparent tropical positions and from Swiss
  Ephemeris, especially the Moon's parallax and fast-moving objects. There is
  no independent end-to-end numerical accuracy claim yet. The Earth-orientation
  approximation, model choices and use of barycenters are explicit rather
  than silent fallbacks.

## Verification and provenance

Synthetic tests cover UTC offsets, the 2016 leap-second boundary, data expiry,
invalid inputs, GMST and WGS84 cardinal geometry. The IAU 2006 precession
matrix is compared with ERFA's `t_pmat06` reference (see
[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)). An ignored integration
test and the CLI were executed with the user's external DE441 file, checking
time/location sensitivity and plausible geometric results. This does **not**
replace independent JPL Horizons reference-vector or topocentric comparisons.

Sources: [IERS leap-second data](https://hpiers.obspm.fr/eoppc/bul/bulc/Leap_Second.dat),
[USNO TT and TAI](https://aa.usno.navy.mil/faq/TT),
[USNO Circular 179](https://aa.usno.navy.mil/downloads/reports/Kaplan2005b.pdf),
[USNO sidereal-time approximation](https://aa.usno.navy.mil/faq/GAST),
[USNO WGS84 observer geometry](https://aa.usno.navy.mil/downloads/Circular_179_bw.pdf),
and the project IAU 2006 prototype in the repository's `main` history.
Original Rust source is CPAL-1.0 with existing VirgoStars attribution. The
ERFA numerical reference retains its separate terms. Modification date:
2026-11-10.
