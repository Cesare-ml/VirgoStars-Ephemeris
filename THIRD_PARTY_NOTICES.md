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

Modification (2026-11-14): the same `first_sky.rs` test module also uses
three numeric WGS84 Earth-fixed XYZ reference values from ERFA's `t_gd2gc`
conformance test (longitude 3.1 radians, latitude −0.5 radians, height
2500 meters), converting ERFA meters to VirgoStars kilometers. It does
not copy ERFA algorithms or bundle the library; the ERFA license at
[third_party/ERFA_LICENSE.txt](third_party/ERFA_LICENSE.txt) applies to
these numerical references as described above.
