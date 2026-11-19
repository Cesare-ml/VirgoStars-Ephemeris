// SPDX-License-Identifier: CPAL-1.0
//! ERFA 2.0.1.5 long-term mean-frame fixtures, not physical accuracy claims.
use virgostars_ephemeris_core::earth_orientation::{
    experimental_extrapolated_nutation, iau2000b_nutation, OrientationError,
};
use virgostars_ephemeris_core::long_term_precession::mean_date_matrices;

#[test]
fn long_term_poles_agree_with_erfa_at_package_range_samples() {
    // ERFA ltp / ltpecl using epj = 2000 + (JD_TT - 2451545) / 365.25.
    let cases = [
        (
            1721060.0,
            [0.8844534484891358, 0.4272533968515494, 0.18760765531952134],
            [
                -0.000023370227055237718,
                -0.40200878220345493,
                0.9156358110543338,
            ],
        ),
        (
            2561118.0,
            [
                0.9973229698438997,
                -0.06708070821544884,
                -0.029104508363103056,
            ],
            [
                0.00006950784091492624,
                -0.39715433635106706,
                0.9177518337144326,
            ],
        ),
    ];
    for (tt, expected_equinox, expected_ecliptic_pole) in cases {
        let (eq, ecl) = mean_date_matrices(tt).unwrap();
        for i in 0..3 {
            assert!((eq[0][i] - expected_equinox[i]).abs() < 1e-12);
            assert!((ecl[2][i] - expected_ecliptic_pole[i]).abs() < 1e-12);
        }
        assert!(experimental_extrapolated_nutation(tt).is_ok());
        assert!(matches!(
            iau2000b_nutation(tt),
            Err(OrientationError::UnsupportedEpoch { .. })
        ));
    }
}
