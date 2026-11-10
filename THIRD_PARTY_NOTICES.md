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
