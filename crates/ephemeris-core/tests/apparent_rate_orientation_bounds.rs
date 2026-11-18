// SPDX-License-Identifier: CPAL-1.0
//! Boundary contract for the matrix and its centered derivative; no DE441 data.
use virgostars_ephemeris_core::apparent_ecliptic_of_date::ecliptic_of_date_rotation_with_rate;
use virgostars_ephemeris_core::earth_orientation::{
    true_ecliptic_of_date_matrix, OrientationError, JD_TT_MAX, JD_TT_MIN,
};

#[test]
fn matrix_rate_stencil_requires_an_interior_tt_margin() {
    for tt in [JD_TT_MIN, JD_TT_MIN + 0.005, JD_TT_MAX - 0.005] {
        assert!(true_ecliptic_of_date_matrix(tt).is_ok());
        assert!(matches!(
            ecliptic_of_date_rotation_with_rate(tt),
            Err(OrientationError::UnsupportedEpoch { .. })
        ));
    }
    for tt in [JD_TT_MIN + 0.0101, JD_TT_MAX - 0.0101] {
        let (matrix, rate) = ecliptic_of_date_rotation_with_rate(tt).unwrap();
        assert!(matrix
            .iter()
            .flatten()
            .chain(rate.iter().flatten())
            .all(|v| v.is_finite()));
    }
    assert!(matches!(
        true_ecliptic_of_date_matrix(JD_TT_MAX),
        Err(OrientationError::UnsupportedEpoch { .. })
    ));
}
