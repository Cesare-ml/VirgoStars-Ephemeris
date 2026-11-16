// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

#![allow(clippy::excessive_precision, clippy::needless_range_loop)]

use core::f64::consts::TAU;
use virgostars_ephemeris_core::earth_orientation::{
    ecliptic_nutation, era_iau2000, gast_iau2006, gmst_iau2006, iau2000b_nutation,
    iau2000b_nutation_matrix, iau2006_bias_precession_matrix, iau2006_mean_obliquity,
    matrix_multiply, matrix_transpose, matrix_vector_multiply, true_ecliptic_of_date_matrix,
    true_equator_of_date_matrix, true_obliquity, x_rotation, OrientationError, JD_TT_MAX,
    JD_TT_MIN,
};

fn near(a: f64, b: f64, tolerance: f64) {
    assert!(
        (a - b).abs() <= tolerance,
        "{a} != {b} (diff: {}, max tol: {tolerance})",
        (a - b).abs()
    );
}

fn determinant(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

#[test]
fn matches_published_erfa_nut00b_test_vector() {
    // Reference oracle from ERFA t_erfa_c.c / nut00b:
    // TT Julian Date = 2453736.5 (2006-01-15 00:00:00 TT).
    let jd = 2_453_736.5;
    let nut = iau2000b_nutation(jd).expect("valid epoch in range");

    let expected_d_psi = -0.9632552291148362783e-5;
    let expected_d_eps = 0.4063197106621159367e-4;

    near(nut.d_psi_rad, expected_d_psi, 1e-13);
    near(nut.d_epsilon_rad, expected_d_eps, 1e-13);
    assert_eq!(nut.terms, 77);
}

#[test]
fn matches_published_erfa_num00b_matrix() {
    // Reference oracle from ERFA t_erfa_c.c / num00b:
    let jd = 2_453_736.5;
    let ref_mean_obliquity = 0.4090791789404229916;
    let matrix = iau2000b_nutation_matrix(jd, Some(ref_mean_obliquity)).expect("valid matrix");

    let expected = [
        [
            0.9999999999536069682,
            0.8837746144871248011e-5,
            0.3831488838252202945e-5,
        ],
        [
            -0.8837590456632304720e-5,
            0.9999999991354692733,
            -0.4063198798559591654e-4,
        ],
        [
            -0.3831847930134941271e-5,
            0.4063195412258168380e-4,
            0.9999999991671806225,
        ],
    ];

    for i in 0..3 {
        for j in 0..3 {
            near(matrix[i][j], expected[i][j], 1e-13);
        }
    }
}

#[test]
fn matrix_orthogonality_and_invariants() {
    for jd in [2_451_545.0, 2_453_736.5, 2_460_000.0] {
        let nut_m = iau2000b_nutation_matrix(jd, None).unwrap();
        let bp_m = iau2006_bias_precession_matrix(jd).unwrap();
        let eq_m = true_equator_of_date_matrix(jd).unwrap();
        let ecl_m = true_ecliptic_of_date_matrix(jd).unwrap();

        for m in [nut_m, bp_m, eq_m, ecl_m] {
            near(determinant(m), 1.0, 1e-12);
            let mt = matrix_transpose(m);
            let identity = matrix_multiply(m, mt);
            for i in 0..3 {
                for j in 0..3 {
                    let expected = if i == j { 1.0 } else { 0.0 };
                    near(identity[i][j], expected, 1e-12);
                }
            }

            // Norm preservation test
            let v: [f64; 3] = [1234.56, -7890.12, 3456.78];
            let v_len = (v[0].powi(2) + v[1].powi(2) + v[2].powi(2)).sqrt();
            let rotated = matrix_vector_multiply(m, v);
            let rot_len = (rotated[0].powi(2) + rotated[1].powi(2) + rotated[2].powi(2)).sqrt();
            near(rot_len, v_len, 1e-11);
        }
    }
}

#[test]
fn true_of_date_ecliptic_tilt_consistency() {
    let jd = 2_453_736.5;
    let nut = iau2000b_nutation(jd).unwrap();
    let eps_mean = iau2006_mean_obliquity(jd).unwrap();
    let eps_true = true_obliquity(eps_mean, nut.d_epsilon_rad);

    let eq_mat = true_equator_of_date_matrix(jd).unwrap();
    let ecl_mat = true_ecliptic_of_date_matrix(jd).unwrap();

    let v = [1.0e6, 2.0e5, 3.0e5];
    let eq_vec = matrix_vector_multiply(eq_mat, v);
    let ecl_vec = matrix_vector_multiply(ecl_mat, v);

    // Ecliptic vector must equal eq_vec rotated by +eps_true about +X
    let expected_ecl = matrix_vector_multiply(x_rotation(eps_true), eq_vec);
    for i in 0..3 {
        near(ecl_vec[i], expected_ecl[i], 1e-10);
    }
}

#[test]
fn sidereal_time_strictly_distinguishes_ut1_from_tt() {
    let jd_ut1 = 2_453_736.5;
    let jd_tt = 2_453_736.500_74; // ~64 seconds offset

    let era = era_iau2000(jd_ut1).unwrap();
    assert!((0.0..TAU).contains(&era));

    let gmst = gmst_iau2006(jd_ut1, jd_tt).unwrap();
    assert!((0.0..TAU).contains(&gmst));

    let gast = gast_iau2006(jd_ut1, jd_tt).unwrap();
    assert!((0.0..TAU).contains(&gast));

    let nut = iau2000b_nutation(jd_tt).unwrap();
    let eps = iau2006_mean_obliquity(jd_tt).unwrap();
    let ee = nut.d_psi_rad * eps.cos();

    near(gast, (gmst + ee).rem_euclid(TAU), 1e-14);
}

#[test]
fn swiss_ecl_nut_contract() {
    let jd = 2_453_736.5;
    let ecl = ecliptic_nutation(jd).expect("valid ecliptic nutation");
    let swiss = ecl.to_swiss_ecl_nut();

    assert_eq!(swiss.len(), 6);
    near(swiss[0], ecl.true_obliquity_deg(), 1e-12);
    near(swiss[1], ecl.mean_obliquity_deg(), 1e-12);
    near(swiss[2], ecl.nutation_longitude_deg(), 1e-12);
    near(swiss[3], ecl.nutation_obliquity_deg(), 1e-12);
    near(swiss[4], ecl.equation_of_equinoxes_deg(), 1e-12);
    assert_eq!(swiss[5], 0.0);

    // True obliquity must be strictly equal to mean obliquity + nutation in obliquity.
    near(swiss[0], swiss[1] + swiss[3], 1e-12);
}

#[test]
fn explicit_domain_errors_and_nonfinite_rejection() {
    // Pre-1900 boundary
    assert!(matches!(
        iau2000b_nutation(JD_TT_MIN - 0.001),
        Err(OrientationError::UnsupportedEpoch { .. })
    ));
    assert!(iau2000b_nutation(JD_TT_MIN).is_ok());

    // Post-2100 boundary
    assert!(matches!(
        iau2000b_nutation(JD_TT_MAX),
        Err(OrientationError::UnsupportedEpoch { .. })
    ));
    assert!(iau2000b_nutation(JD_TT_MAX - 0.001).is_ok());

    // Non-finite values fail closed
    assert!(matches!(
        iau2000b_nutation(f64::NAN),
        Err(OrientationError::NonFinite { field: "tt_jd" })
    ));
    assert!(matches!(
        iau2000b_nutation(f64::INFINITY),
        Err(OrientationError::NonFinite { field: "tt_jd" })
    ));
    assert!(matches!(
        iau2000b_nutation_matrix(2_453_736.5, Some(f64::NAN)),
        Err(OrientationError::InvalidObliquity)
    ));
    assert!(matches!(
        era_iau2000(f64::NAN),
        Err(OrientationError::NonFinite { field: "ut1_jd" })
    ));
}
