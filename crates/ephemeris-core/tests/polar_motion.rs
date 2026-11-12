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

//! IERS polar motion: independently published SOFA/ERFA matrix fixture.
//!
//! Created 2026-11-12. Tests are for Earth-fixed position rotations only,
//! not the complete observer-to-DE441 transformation.

use virgostars_ephemeris_core::polar_motion::{
    itrs_to_tirs_polar_motion_matrix, itrs_to_tirs_position, tirs_to_itrs_position,
    PolarMotionError, MAX_POLAR_MOTION_ANGLE_RAD,
};

fn near(value: f64, expected: f64, tolerance: f64) {
    assert!(
        (value - expected).abs() <= tolerance,
        "actual {value} versus expected {expected} (tol {tolerance})"
    );
}

fn near_vector(value: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (x, y) in value.into_iter().zip(expected) {
        near(x, y, tolerance);
    }
}

#[test]
fn matches_sofa_erfa_pom00_transposed_matrix_reference() {
    // Independently published: ERFA src/t_erfa_c.c, t_pom00.
    // eraPom00(xp=2.55060238e-7, yp=1.860359247e-6,
    //          sp=-0.1367174580728891460e-10)
    // reports the TIRS-to-ITRS matrix, which MUST BE TRANSPOSED to
    // compare with this API's ITRS-to-TIRS matrix.
    let xp = 2.550_602_38e-7;
    let yp = 1.860_359_247e-6;
    let sp = -0.136_717_458_072_889_15e-10;
    let w = itrs_to_tirs_polar_motion_matrix(xp, yp, sp).unwrap();
    let published_rpom = [
        [
            0.999_999_999_999_967_5,
            -1.367_174_580_728_847e-11,
            2.550_602_379_999_972e-7,
        ],
        [
            1.414_624_947_957_029_8e-11,
            0.999_999_999_998_269_5,
            -1.860_359_246_998_866_4e-6,
        ],
        [
            -2.550_602_379_741_215e-7,
            1.860_359_247_002_414e-6,
            0.999_999_999_998_237,
        ],
    ];
    for (i, row) in w.into_iter().enumerate() {
        for (j, value) in row.into_iter().enumerate() {
            near(value, published_rpom[j][i], 1.0e-15);
        }
    }
}

#[test]
fn zero_eop_is_identity_not_arbitrary_terrestrial_rotation() {
    let w = itrs_to_tirs_polar_motion_matrix(0.0, 0.0, 0.0).unwrap();
    assert_eq!(w, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    let site = [6_378.137, 10.0, -30.0];
    assert_eq!(itrs_to_tirs_position(site, 0.0, 0.0, 0.0), Ok(site));
    assert_eq!(tirs_to_itrs_position(site, 0.0, 0.0, 0.0), Ok(site));
}

#[test]
fn pole_offsets_have_correct_iers_rotation_signs() {
    let xp = 4.0e-6;
    let yp = -3.0e-6;
    let w = itrs_to_tirs_polar_motion_matrix(xp, yp, 0.0).unwrap();
    // ITRS +Z projected into TIRS X,Y from first-order polar motion.
    near(w[0][2], -xp, 1.0e-12);
    near(w[1][2], yp, 1.0e-12);
    // The CIP in ITRS is W^T (0,0,1) = (xp,-yp,1) to first order.
    near(w[2][0], xp, 1.0e-12);
    near(w[2][1], -yp, 1.0e-12);
}

#[test]
fn tio_locator_rotates_equatorial_reference_without_pole_motion() {
    let sp = -2.0e-7;
    let site = itrs_to_tirs_position([1.0, 0.0, 0.0], 0.0, 0.0, sp).unwrap();
    near(site[0], sp.cos(), 1.0e-15);
    near(site[1], sp.sin(), 1.0e-15);
    near(site[2], 0.0, 1.0e-15);
}

#[test]
fn roundtrip_recovers_independent_geocentric_vectors() {
    for site in [
        [6_378.137, 0.0, 0.0],
        [-5_599.000_557_705, 233.011_672_234_792, -3_040.909_470_698],
        [0.0, 0.0, -6_356.752],
    ] {
        for (xp, yp, sp) in [
            (0.0, 0.0, 0.0),
            (2.550_602_38e-7, 1.860_359_247e-6, -1.367_174_580_728_89e-11),
            (0.009, -0.008, 0.001),
        ] {
            let tirs = itrs_to_tirs_position(site, xp, yp, sp).unwrap();
            let recovered = tirs_to_itrs_position(tirs, xp, yp, sp).unwrap();
            near_vector(recovered, site, 6.0e-12);
        }
    }
}

#[test]
fn matrix_is_orthogonal_and_has_right_handed_orientation() {
    for (xp, yp, sp) in [
        (0.0, 0.0, 0.0),
        (2.550_602_38e-7, 1.860_359_247e-6, -1.367_174_580_728_89e-11),
        (0.008, -0.006, 0.004),
    ] {
        let w = itrs_to_tirs_polar_motion_matrix(xp, yp, sp).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                let dot = (0..3).map(|k| w[i][k] * w[j][k]).sum::<f64>();
                near(dot, if i == j { 1.0 } else { 0.0 }, 5.0e-16);
            }
        }
        let determinant = w[0][0] * (w[1][1] * w[2][2] - w[1][2] * w[2][1])
            - w[0][1] * (w[1][0] * w[2][2] - w[1][2] * w[2][0])
            + w[0][2] * (w[1][0] * w[2][1] - w[1][1] * w[2][0]);
        near(determinant, 1.0, 1.0e-15);
    }
}

#[test]
fn strict_input_units_and_nonfinite_values_are_rejected() {
    for bad in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -MAX_POLAR_MOTION_ANGLE_RAD - 0.01,
        MAX_POLAR_MOTION_ANGLE_RAD + 0.01,
    ] {
        assert_eq!(
            itrs_to_tirs_polar_motion_matrix(bad, 0.0, 0.0),
            Err(PolarMotionError::InvalidXp)
        );
        assert_eq!(
            itrs_to_tirs_polar_motion_matrix(0.0, bad, 0.0),
            Err(PolarMotionError::InvalidYp)
        );
        assert_eq!(
            itrs_to_tirs_polar_motion_matrix(0.0, 0.0, bad),
            Err(PolarMotionError::InvalidTioLocator)
        );
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            itrs_to_tirs_position([bad, 0.0, 0.0], 0.0, 0.0, 0.0),
            Err(PolarMotionError::NonFinitePosition)
        );
        assert_eq!(
            tirs_to_itrs_position([bad, 0.0, 0.0], 0.0, 0.0, 0.0),
            Err(PolarMotionError::NonFinitePosition)
        );
    }
    assert!(itrs_to_tirs_polar_motion_matrix(0.01, -0.01, 0.01).is_ok());
}

#[test]
fn large_finite_vectors_do_not_return_nonfinite_results() {
    let maximum = f64::MAX;
    assert_eq!(
        itrs_to_tirs_position([maximum, maximum, maximum], -0.01, -0.01, -0.01),
        Err(PolarMotionError::NonFiniteResult)
    );
}
