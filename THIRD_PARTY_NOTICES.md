# Third-party scientific reference data

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

`crates/ephemeris-core/src/earth_orientation.rs` and its associated tests in
`crates/ephemeris-core/tests/earth_orientation_contract.rs` use the 77-term
IAU 2000B luni-solar nutation series table and numerical conformance values
from the ERFA `nut00b` and `num00b` routines. Reference:
<https://github.com/liberfa/erfa/blob/master/src/nut00b.c>.
Subject to the same NumFOCUS copyright and BSD-3 license in
[third_party/ERFA_LICENSE.txt](third_party/ERFA_LICENSE.txt).
Only numerical coefficient constants and verification values are used; no
ERFA C implementation functions are included.
