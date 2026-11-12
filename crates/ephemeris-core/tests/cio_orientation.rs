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

//! Independent ERFA c2ixys fixture and IERS GCRS/CIRS geometric tests.
//!
//! Added 2026-11-12. This validates the supplied X/Y/s positional
//! orientation only; it does not derive CIP/CIO series from a TT epoch.

use std::f64::consts::{FRAC_PI_2, PI};

use virgostars_ephemeris_core::cio_orientation::{
    cirs_to_gcrs_position, gcrs_to_cirs_cio_matrix, gcrs_to_cirs_position,
    CioOrientationError,
};

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}, tolerance {tolerance}"
    );
}

fn near_vector(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (a, b) in actual.into_iter().zip(expected) {
        near(a, b, tolerance);
    }
}

#[test]
fn independent_erfa_c2ixys_nine_element_reference_agrees() {
    // ERFA src/t_erfa_c.c t_c2ixys, 2013-08-07:
    // X=0.5791308486706011000e-3,
    // Y=0.4020579816732961219e-4,
    // s=-0.1220040848472271978e-7.
    // Expected values independently published by SOFA/ERFA.
    // The expected constants below are rounded to representable f64.
    let x = 0.579_130_848_670_601_1e-3;
    let y = 0.402_057_981_673_296_1e-4;
    let s = -0.122_004_084_847_227_2e-7;
    let expected = [
        [
            0.999_999_832_303_715_7,
            0.558_198_486_916_849_9e-9,
            -0.579_130_849_161_128_2e-3,
        ],
        [
            -0.238_426_164_267_044e-7,
            0.999_999_999_191_746_9,
            -0.402_057_911_016_966_9e-4,
        ],
        [
            0.579_130_848_670_601_1e-3,
            0.402_057_981_673_296_1e-4,
            0.999_999_831_495_462_8,
        ],
    ];
    let actual = gcrs_to_cirs_cio_matrix(x, y, s).unwrap();
    for i in 0..3 {
        for j in 0..3 {
            near(actual[i][j], expected[i][j], 2.0e-14);
        }
    }
}

#[test]
fn cip_unit_vector_maps_exactly_to_intermediate_positive_pole() {
    for (x, y, s) in [
        (0.0, 0.0, 0.0),
        (0.000_579_13, 0.000_040_205, -1.22e-8),
        (-0.12, 0.08, 0.2),
    ] {
        let z = (1.0_f64 - x * x - y * y).sqrt();
        let cip = [x, y, z];
        near_vector(gcrs_to_cirs_position(cip, x, y, s).unwrap(), [0.0, 0.0, 1.0], 4.0e-15);
        near_vector(cirs_to_gcrs_position([0.0, 0.0, 1.0], x, y, s).unwrap(), cip, 4.0e-15);
    }
}

#[test]
fn zero_cip_and_s_produce_identity_transform() {
    let identity = gcrs_to_cirs_cio_matrix(0.0, 0.0, 0.0).unwrap();
    assert_eq!(
        identity,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    );
    let position = [6_378.137, -2_150.0, 1_000.0];
    assert_eq!(gcrs_to_cirs_position(position, 0.0, 0.0, 0.0), Ok(position));
    assert_eq!(cirs_to_gcrs_position(position, 0.0, 0.0, 0.0), Ok(position));
}

#[test]
fn positive_cio_locator_rotates_first_two_intermediate_axes() {
    let s = 0.3;
    let direction = gcrs_to_cirs_position([1.0, 0.0, 0.0], 0.0, 0.0, s).unwrap();
    near_vector(direction, [s.cos(), s.sin(), 0.0], 2.0e-15);
    let opposite = cirs_to_gcrs_position([1.0, 0.0, 0.0], 0.0, 0.0, s).unwrap();
    near_vector(opposite, [s.cos(), -s.sin(), 0.0], 2.0e-15);
}

#[test]
fn ci_rotation_and_transpose_roundtrip_positions_at_multiple_cip_values() {
    for (x, y, s) in [
        (0.0, 0.0, 0.0),
        (0.000_579_13, -0.000_040_205, -1.22e-8),
        (-0.07, 0.10, 0.25),
        (0.45, -0.3, -1.5),
    ] {
        for original in [
            [6_378.137, 0.0, 0.0],
            [3_300.0, -2_500.0, 1_900.0],
            [-5_700.0, 1_400.0, -2_100.0],
        ] {
            let intermediate = gcrs_to_cirs_position(original, x, y, s).unwrap();
            let recovered = cirs_to_gcrs_position(intermediate, x, y, s).unwrap();
            near_vector(recovered, original, 4.0e-12);
        }
    }
}

#[test]
fn orientation_is_right_handed_and_preserves_distances() {
    for (x, y, s) in [
        (0.000_579_13, 0.000_040_205, -1.22e-8),
        (-0.2, 0.08, 0.5),
        (0.0, 0.0, -FRAC_PI_2),
    ] {
        let matrix = gcrs_to_cirs_cio_matrix(x, y, s).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                let dot = (0..3).map(|k| matrix[i][k] * matrix[j][k]).sum::<f64>();
                near(dot, if i == j { 1.0 } else { 0.0 }, 7.0e-16);
            }
        }
        let det = matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
            - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
            + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]);
        near(det, 1.0, 8.0e-16);
        let vector = [1_550.0, -3_330.0, 4_250.0];
        let transformed = gcrs_to_cirs_position(vector, x, y, s).unwrap();
        let norm_before = vector.into_iter().map(|v| v * v).sum::<f64>().sqrt();
        let norm_after = transformed.into_iter().map(|v| v * v).sum::<f64>().sqrt();
        near(norm_before, norm_after, 3.0e-12);
    }
}

#[test]
fn invalid_cip_locator_and_positions_are_strictly_rejected() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0, -1.0] {
        assert_eq!(
            gcrs_to_cirs_cio_matrix(invalid, 0.0, 0.0),
            Err(CioOrientationError::InvalidCipCoordinates)
        );
        assert_eq!(
            gcrs_to_cirs_cio_matrix(0.0, invalid, 0.0),
            Err(CioOrientationError::InvalidCipCoordinates)
        );
    }
    assert_eq!(
        gcrs_to_cirs_cio_matrix(0.8, 0.8, 0.0),
        Err(CioOrientationError::InvalidCipCoordinates)
    );
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, PI + 0.01, -PI - 0.01] {
        assert_eq!(
            gcrs_to_cirs_cio_matrix(0.0, 0.0, invalid),
            Err(CioOrientationError::InvalidCioLocator)
        );
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let point = [1.0, invalid, 0.0];
        assert_eq!(
            gcrs_to_cirs_position(point, 0.0, 0.0, 0.0),
            Err(CioOrientationError::NonFinitePosition)
        );
        assert_eq!(
            cirs_to_gcrs_position(point, 0.0, 0.0, 0.0),
            Err(CioOrientationError::NonFinitePosition)
        );
    }
}

#[test]
fn overlarge_finite_positions_cannot_silently_overflow() {
    let point = [f64::MAX, f64::MAX, 0.0];
    assert_eq!(
        gcrs_to_cirs_position(point, 0.0, 0.0, FRAC_PI_2 / 2.0),
        Err(CioOrientationError::NonFiniteResult)
    );
    assert_eq!(
        cirs_to_gcrs_position(point, 0.0, 0.0, -FRAC_PI_2 / 2.0),
        Err(CioOrientationError::NonFiniteResult)
    );
}
