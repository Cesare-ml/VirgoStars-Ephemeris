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

//! Nutation orientation regression against the independent ERFA Numat
//! numerical fixture. No ERFA software code is copied or linked.

use virgostars_ephemeris_core::nutation_matrix::{
    nutation_matrix_from_angles, NutationAngles, NutationMatrixError,
};

fn example() -> NutationAngles {
    NutationAngles {
        mean_obliquity_rad: 0.409_078_976_335_650_99,
        mean_obliquity_rate_rad_per_tt_day: 0.0,
        delta_psi_rad: -0.963_090_910_711_558_2e-5,
        delta_psi_rate_rad_per_tt_day: 0.0,
        delta_epsilon_rad: 0.406_323_917_400_167_9e-4,
        delta_epsilon_rate_rad_per_tt_day: 0.0,
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.3e}"
    );
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

#[test]
fn matches_all_nine_independent_erfa_numat_matrix_elements() {
    // Independent ERFA t_numat fixture:
    // eraNumat(epsa=0.4090789763356509900,
    //          dpsi=-0.9630909107115582393e-5,
    //          deps=0.4063239174001678826e-4).
    // https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c
    const EXPECTED: [[f64; 3]; 3] = [
        [
            0.999_999_999_953_622_8,
            0.883_623_932_023_625_1e-5,
            0.383_083_344_745_825_2e-5,
        ],
        [
            -0.883_608_365_701_668_9e-5,
            0.999_999_999_135_465_5,
            -0.406_324_086_536_185_8e-4,
        ],
        [
            -0.383_119_248_183_338_5e-5,
            0.406_323_748_021_693_4e-4,
            0.999_999_999_167_166,
        ],
    ];
    let actual = nutation_matrix_from_angles(example()).unwrap();
    for (row, expected_row) in EXPECTED.iter().enumerate() {
        for (col, expected) in expected_row.iter().enumerate() {
            near(actual.rotation[row][col], *expected, 2e-15);
        }
    }
}

#[test]
fn zero_nutation_is_identity_even_if_mean_obliquity_changes() {
    let mut angles = example();
    angles.delta_psi_rad = 0.0;
    angles.delta_epsilon_rad = 0.0;
    angles.mean_obliquity_rate_rad_per_tt_day = -7e-8;
    let matrix = nutation_matrix_from_angles(angles).unwrap();
    for i in 0..3 {
        for j in 0..3 {
            near(matrix.rotation[i][j], if i == j { 1.0 } else { 0.0 }, 2e-15);
            near(matrix.rate_per_tt_day[i][j], 0.0, 2e-17);
        }
    }
}

#[test]
fn time_derivatives_match_recomputed_angles_at_shifted_tt_dates() {
    let mut initial = example();
    initial.mean_obliquity_rate_rad_per_tt_day = -1.1e-7;
    initial.delta_psi_rate_rad_per_tt_day = 2.2e-7;
    initial.delta_epsilon_rate_rad_per_tt_day = -3.2e-7;

    // Independent numerical derivative: rebuild the full orientation at
    // two neighboring TT epochs with all three angles evolving linearly.
    let at = |days: f64| {
        let angles = NutationAngles {
            mean_obliquity_rad: initial.mean_obliquity_rad
                + initial.mean_obliquity_rate_rad_per_tt_day * days,
            delta_psi_rad: initial.delta_psi_rad + initial.delta_psi_rate_rad_per_tt_day * days,
            delta_epsilon_rad: initial.delta_epsilon_rad
                + initial.delta_epsilon_rate_rad_per_tt_day * days,
            ..initial
        };
        nutation_matrix_from_angles(angles).unwrap()
    };
    let present = at(0.0);
    let step = 0.25;
    let earlier = at(-step);
    let later = at(step);
    for i in 0..3 {
        for j in 0..3 {
            let finite = (later.rotation[i][j] - earlier.rotation[i][j]) / (2.0 * step);
            near(present.rate_per_tt_day[i][j], finite, 3e-13);
        }
    }
}

#[test]
fn rotation_stays_right_handed_and_rate_is_skew_tangent() {
    let mut angles = example();
    angles.mean_obliquity_rate_rad_per_tt_day = -1e-7;
    angles.delta_psi_rate_rad_per_tt_day = 2e-7;
    angles.delta_epsilon_rate_rad_per_tt_day = -3e-7;
    let state = nutation_matrix_from_angles(angles).unwrap();
    let rotation = state.rotation;
    let rate = state.rate_per_tt_day;
    for i in 0..3 {
        for j in 0..3 {
            near(
                dot(rotation[i], rotation[j]),
                if i == j { 1.0 } else { 0.0 },
                3e-15,
            );
            near(
                dot(rate[i], rotation[j]) + dot(rotation[i], rate[j]),
                0.0,
                2e-19,
            );
        }
    }
    let cross = [
        rotation[1][1] * rotation[2][2] - rotation[1][2] * rotation[2][1],
        rotation[1][2] * rotation[2][0] - rotation[1][0] * rotation[2][2],
        rotation[1][0] * rotation[2][1] - rotation[1][1] * rotation[2][0],
    ];
    near(dot(rotation[0], cross), 1.0, 3e-15);
}

#[test]
fn nonfinite_inputs_do_not_produce_plausible_looking_rotations() {
    let mut angles = example();
    angles.delta_psi_rad = f64::NAN;
    assert_eq!(
        nutation_matrix_from_angles(angles),
        Err(NutationMatrixError::NonFiniteInput),
    );
    let mut angles = example();
    angles.delta_epsilon_rate_rad_per_tt_day = f64::INFINITY;
    assert_eq!(
        nutation_matrix_from_angles(angles),
        Err(NutationMatrixError::NonFiniteInput),
    );
}

#[test]
fn rejects_nonastronomical_angles_without_clamping_or_fallback() {
    let mut angles = example();
    angles.mean_obliquity_rad = 0.0;
    assert_eq!(
        nutation_matrix_from_angles(angles),
        Err(NutationMatrixError::AngleOutsideGuardedRange),
    );
    let mut angles = example();
    angles.delta_psi_rad = 0.1;
    assert_eq!(
        nutation_matrix_from_angles(angles),
        Err(NutationMatrixError::AngleOutsideGuardedRange),
    );
    let mut angles = example();
    angles.delta_epsilon_rad = -1.0;
    assert_eq!(
        nutation_matrix_from_angles(angles),
        Err(NutationMatrixError::AngleOutsideGuardedRange),
    );
}
