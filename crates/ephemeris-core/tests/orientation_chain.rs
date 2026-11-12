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

//! Matrix composition against independent SOFA/ERFA c2tcio output.
//!
//! Added 2026-11-12. Tests cover only supplied fixed-epoch rotations;
//! they do not independently validate JPL/Horizons, the X/Y/s or EOP
//! time series, their epoch provenance, or true inertial velocity.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use virgostars_ephemeris_core::orientation_chain::{
    compose_gcrs_to_itrs, GcrsToItrsMatrix, OrientationChainError,
};

const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}; tolerance {tolerance}"
    );
}

fn close_vector(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (a, b) in actual.into_iter().zip(expected) {
        close(a, b, tolerance);
    }
}

fn transpose(matrix: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| matrix[j][i]))
}

#[test]
fn agrees_with_nine_independently_published_erfa_c2tcio_values() {
    // Independent ERFA src/t_erfa_c.c, t_c2tcio (not copied algorithms).
    // Inputs: rc2i GCRS->CIRS, rpom TIRS->ITRS, ERA(UT1) radians;
    // expected output: GCRS->ITRS. Our input W is rpom TRANSPOSE.
    // Literals are rounded to representable f64, as required by Clippy.
    let c: [[f64; 3]; 3] = [
        [
            0.999_999_832_303_716_5,
            0.558_152_627_171_430_4e-9,
            -0.579_130_847_707_344_4e-3,
        ],
        [
            -0.238_426_622_752_472_2e-7,
            0.999_999_999_191_740_4,
            -0.402_059_495_503_070_4e-4,
        ],
        [
            0.579_130_847_216_815_3e-3,
            0.402_059_566_159_399_4e-4,
            0.999_999_831_495_457_2,
        ],
    ];
    let rpom: [[f64; 3]; 3] = [
        [
            0.999_999_999_999_967_5,
            -0.136_717_458_072_884_7e-10,
            0.255_060_237_999_997_3e-6,
        ],
        [
            0.141_462_494_795_703e-10,
            0.999_999_999_998_269_5,
            -0.186_035_924_699_886_6e-5,
        ],
        [
            -0.255_060_237_974_121_5e-6,
            0.186_035_924_700_241_4e-5,
            0.999_999_999_998_237,
        ],
    ];
    let expected: [[f64; 3]; 3] = [
        [
            -0.181_033_212_830_711_05,
            0.983_476_980_693_847,
            0.655_553_563_868_546_7e-4,
        ],
        [
            -0.983_476_813_413_599_7,
            -0.181_033_220_364_944_85,
            0.574_980_111_614_110_7e-3,
        ],
        [
            0.577_347_401_408_140_7e-3,
            0.396_183_239_177_265_9e-4,
            0.999_999_832_550_169_2,
        ],
    ];
    let calculated = compose_gcrs_to_itrs(transpose(rpom), 1.752_833_255_303_07, c)
        .unwrap()
        .gcrs_to_itrs;
    for i in 0..3 {
        for j in 0..3 {
            close(calculated[i][j], expected[i][j], 1.0e-12);
        }
    }
}

#[test]
fn complete_identity_has_no_hidden_rotation_or_default_eop() {
    let chain = compose_gcrs_to_itrs(IDENTITY, 0.0, IDENTITY).unwrap();
    assert_eq!(chain.gcrs_to_itrs, IDENTITY);
    let v = [6_378.137, -2_300.0, 4_000.0];
    assert_eq!(chain.gcrs_to_itrs_position(v), Ok(v));
    assert_eq!(chain.itrs_to_gcrs_position(v), Ok(v));
}

#[test]
fn pure_era_has_correct_westward_celestial_to_terrestrial_sign() {
    // For C2I=W=identity, GCRS +X maps toward ITRS -Y as ERA grows.
    // Reversing the chain maps ITRS +X toward GCRS +Y.
    let chain = compose_gcrs_to_itrs(IDENTITY, FRAC_PI_2, IDENTITY).unwrap();
    close_vector(
        chain.gcrs_to_itrs_position([1.0, 0.0, 0.0]).unwrap(),
        [0.0, -1.0, 0.0],
        2.0e-15,
    );
    close_vector(
        chain.itrs_to_gcrs_position([1.0, 0.0, 0.0]).unwrap(),
        [0.0, 1.0, 0.0],
        2.0e-15,
    );
}

#[test]
fn input_w_is_itrs_to_tirs_not_erfa_rpom() {
    let angle = 0.01_f64;
    let (s, c) = angle.sin_cos();
    // W maps ITRS +X toward TIRS +Y.
    let w = [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]];
    let chain = compose_gcrs_to_itrs(w, 0.0, IDENTITY).unwrap();
    // Inverse W^T maps GCRS +X toward ITRS -Y.
    close_vector(
        chain.gcrs_to_itrs_position([1.0, 0.0, 0.0]).unwrap(),
        [c, -s, 0.0],
        2.0e-15,
    );
}

#[test]
fn gcrs_and_itrs_positions_roundtrip_without_changing_norm() {
    let cw = compose_gcrs_to_itrs(IDENTITY, 1.7, IDENTITY).unwrap();
    let original = [3_400.0, -4_500.0, 5_600.0];
    let terrestrial = cw.gcrs_to_itrs_position(original).unwrap();
    let recovered = cw.itrs_to_gcrs_position(terrestrial).unwrap();
    close_vector(recovered, original, 2.0e-12);
    let a = original.into_iter().map(|x| x * x).sum::<f64>().sqrt();
    let b = terrestrial.into_iter().map(|x| x * x).sum::<f64>().sqrt();
    close(a, b, 2.0e-12);
}

#[test]
fn invalid_matrices_cannot_silently_pass_as_physical_orientation() {
    let reflection = [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let shear = [[1.0, 0.2, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for invalid in [reflection, shear] {
        assert_eq!(
            compose_gcrs_to_itrs(invalid, 1.0, IDENTITY),
            Err(OrientationChainError::InvalidPolarMotionMatrix)
        );
        assert_eq!(
            compose_gcrs_to_itrs(IDENTITY, 1.0, invalid),
            Err(OrientationChainError::InvalidCioOrientationMatrix)
        );
    }
    let mut invalid: GcrsToItrsMatrix = IDENTITY;
    invalid[2][0] = f64::NAN;
    assert_eq!(
        compose_gcrs_to_itrs(invalid, 0.0, IDENTITY),
        Err(OrientationChainError::InvalidPolarMotionMatrix)
    );
}

#[test]
fn invalid_era_is_not_wrapped_and_does_not_assume_utc() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01, TAU] {
        assert_eq!(
            compose_gcrs_to_itrs(IDENTITY, value, IDENTITY),
            Err(OrientationChainError::InvalidEarthRotationAngle)
        );
    }
    assert!(compose_gcrs_to_itrs(IDENTITY, PI, IDENTITY).is_ok());
}

#[test]
fn nonfinite_and_overflow_positions_are_deterministic_errors() {
    let chain = compose_gcrs_to_itrs(IDENTITY, FRAC_PI_2 / 2.0, IDENTITY).unwrap();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let point = [bad, 1.0, 0.0];
        assert_eq!(
            chain.gcrs_to_itrs_position(point),
            Err(OrientationChainError::InvalidPosition)
        );
        assert_eq!(
            chain.itrs_to_gcrs_position(point),
            Err(OrientationChainError::InvalidPosition)
        );
    }
    assert_eq!(
        chain.gcrs_to_itrs_position([f64::MAX, f64::MAX, 0.0]),
        Err(OrientationChainError::NonFiniteResult)
    );
}
