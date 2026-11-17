# Third-party scientific reference data

Modification (2026-11-17): `long_term_precession.rs` uses Vondrak et al.
long-term pole coefficient tables and a conformance numerical matrix from ERFA
`ltpecl`, `ltpequ` and `t_ltp`; copyright 2013–2023 NumFOCUS Foundation,
subject to [ERFA's BSD-3 license](third_party/ERFA_LICENSE.txt). Reference:
<https://github.com/liberfa/erfa>. The model is separately implemented in Rust.
The experimental extrapolated nutation path also uses the IERS 2003
fundamental-argument numerical coefficients, checked against ERFA `fal03`,
`falp03`, `faf03`, `fad03` and `faom03`, under the same notice and license.

`crates/ephemeris-core/tests/geometric_projection.rs` uses a numerical
position/velocity example and expected spherical results from the ERFA
`t_pv2s` conformance test. ERFA is copyright 2013–2021 NumFOCUS Foundation;
its license is reproduced in [third_party/ERFA_LICENSE.txt](third_party/ERFA_LICENSE.txt).
Reference: <https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c>.
Only numerical test values are used; no ERFA implementation or library is
included. The ERFA terms remain separate from VirgoStars' CPAL-1.0 license.

`crates/ephemeris-core/tests/calendar_contract.rs` also uses the numeric
Gregorian date/JD reference from the ERFA `t_cal2jd` conformance test, under
the same license and source attribution.

`crates/ephemeris-core/src/first_sky.rs` tests the IAU 2006 precession
matrix against the ERFA `t_pmat06` numerical conformance reference. These
reference values are subject to the same attribution and license above.

Modification (2026-11-14): the same `first_sky.rs` test module also uses
three numeric WGS84 Earth-fixed XYZ reference values from ERFA's `t_gd2gc`
conformance test (longitude 3.1 radians, latitude −0.5 radians, height
2500 meters), converting ERFA meters to VirgoStars kilometers. It does
not copy ERFA algorithms or bundle the library; the ERFA license at
[third_party/ERFA_LICENSE.txt](third_party/ERFA_LICENSE.txt) applies to
these numerical references as described above.

Modification (2026-11-14): `crates/ephemeris-core/src/first_sky.rs`
also uses ERFA `t_hd2ae` conformance *numerical reference results*
(hour angle 1.1 rad, declination 1.2 rad, latitude 0.3 rad, expected
azimuth 5.916889243730066194 rad and altitude 0.4472186304990486228 rad)
to check geometric equatorial-to-horizontal angles. The original Rust
transformation remains independent; no ERFA implementation or library is
bundled. The numerical reference is attributed under the separate
[ERFA license](third_party/ERFA_LICENSE.txt) and original
[conformance test](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).

`crates/ephemeris-core/src/earth_orientation.rs` and its associated tests in
`crates/ephemeris-core/tests/earth_orientation_contract.rs` use the 77-term
IAU 2000B luni-solar nutation series table and numerical conformance values
from the ERFA `nut00b` and `num00b` routines. Reference:
<https://github.com/liberfa/erfa/blob/master/src/nut00b.c>.
Subject to the same NumFOCUS copyright and BSD-3 license in
[third_party/ERFA_LICENSE.txt](third_party/ERFA_LICENSE.txt).
Only numerical coefficient constants and verification values are used; no
ERFA C implementation functions are included.

`crates/ephemeris-core/src/fixed_star.rs` tests use numerical conformance
values from ERFA `t_pmpx`, `t_ldsun` and `t_atci13` (ERFA 2.0.1.5), and its
solar-deflection vector follows the distant-source limit of Kaplan (1998),
USNO. The values keep the NumFOCUS copyright and BSD-3 terms in
[third_party/ERFA_LICENSE.txt](third_party/ERFA_LICENSE.txt); no ERFA
implementation is included.
