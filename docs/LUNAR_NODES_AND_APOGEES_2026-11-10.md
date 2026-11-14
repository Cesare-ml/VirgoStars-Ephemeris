# Experimental lunar nodes and apogees

This Rust module supplies four mathematical points: mean ascending node,
osculating ascending node, mean apogee and osculating apogee. These are
geocentric orbital constructions, not physical light-emitting bodies. The
`calculateLunarPoints` method of the experimental Node process adapter
returns longitude/latitude in degrees, distance in AU and signed rates per
TT day. Its timestamps use the bounded civil-time model documented in
[the true-ecliptic guide](TRUE_ECLIPTIC_DE441_2026-11-14.md).

## Models and limitations

- Mean node uses the Simon/Chapront polynomial and a nominal 384,400 km
  radius. Mean apogee uses the mean perigee polynomial, projected from a
  lunar orbit of fixed 5.145° inclination, with a nominal 405,500 km radius.
  These are approximate models, not an implementation of every perturbation
  in Swiss Ephemeris. The nominal inclination and distances are described in
  [NASA lunar-orbit data](https://ntrs.nasa.gov/api/citations/20230000854/downloads/Minow_SciTech_2023%20-%20presentation%20-%20final.pdf).
- True node and osculating apogee derive from geometric Earth–Moon DE441
  states in the IAU 2006/2000B true ecliptic of date. The node radius is the
  osculating conic's radius at its ascending-node direction; the apogee uses
  the eccentricity vector and bound conic radius. Invalid or unbound cases
  produce errors, not substitute values.
- TT controls the date-frame orientation; a separate coherent TDB epoch is
  passed to DE441. A one-second check detects gross mismatches but cannot
  verify that the epochs refer to the same instant. Osculating-point rates
  use centered ±300-second samples, assuming dTT/dTDB = 1; apparent mean-node
  and mean-apogee rates include the IAU 2000B nutation variation. Numerical
  sampling rejects epochs too close to the model or kernel coverage limits.
- Osculating points are available only in the true-of-date frame. Mean-point
  distances are nominal, not exact physical distances or claims of Swiss
  numerical equivalence. No light-time, aberration or solar deflection is
  applied to these mathematical points.

## Verification and integration status

The synthetic suite checks rates against position differences, mean values,
undefined planes, unbound orbits, and explicit time/frame errors. An optional
real-file regression compares two TT epochs with locally recorded Swiss
0.5.17/DE441 results under deliberately loose per-field tolerances; this is a
compatibility regression, not independent astronomical accuracy certification.
At 2026-11-10, the mean-apogee longitude differs by about 0.0011° and its
latitude by about 0.00037° from that reference. More epochs, method-specific
references, speed tolerances and independent scientific validation are needed.

An exploratory local comparison at matched TT used the same DE441 file and
Swiss 0.5.17 `swe_calc` with JPL and speed flags for 28 dates (January, April,
July and October in 1975, 1980, 1990, 2000, 2010, 2020 and 2026). Maximum
absolute longitude differences were 0.0000146° (mean node), 0.00000372°
(true node), 0.00110° (mean apogee) and 0.0000720° (osculating apogee).
The largest mean-apogee latitude difference was 0.000407°. Maximum absolute
longitude-speed differences across these samples were 2.9e-8, 7.7e-7,
9.8e-7 and 0.0000215 degrees/day, respectively. This diagnostic does not
establish accuracy over the entire supported interval or identical models.

2026-11-14: reconciled the earlier experimental branch, removed the 1 AU mean
node placeholder and unbound-apogee fallback, projected the mean apogee onto
an inclined orbit, distinguished TT from TDB, and added Node access. No Swiss
source, external kernels or new dependency are bundled. CPAL attribution and
the existing separately licensed coefficient notices remain in force.
