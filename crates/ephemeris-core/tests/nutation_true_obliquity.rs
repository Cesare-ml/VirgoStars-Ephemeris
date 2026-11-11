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

//! Synthetic, deterministic tests for consistent true obliquity and
//! TT derivatives from a SINGLE validated nutation-angle set.
//! No official IAU 2000A coefficient table or external code included.

use std::f64::consts::FRAC_PI_2;

use virgostars_ephemeris_core::nutation_matrix::{
    nutation_matrix_from_angles, nutation_with_true_obliquity, NutationAngles, NutationMatrixError,
};

type Matrix = [[f64; 3]; 3];

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.2e}"
    );
}

fn angles() -> NutationAngles {
    NutationAngles {
        mean_obliquity_rad: 0.409,
        mean_obliquity_rate_rad_per_tt_day: -1.5e-7,
        delta_psi_rad: -8.0e-5,
        delta_psi_rate_rad_per_tt_day: 2.0e-7,
        delta_epsilon_rad: 3.0e-5,
        delta_epsilon_rate_rad_per_tt_day: -3.0e-7,
    }
}

fn passive_x(theta: f64) -> Matrix {
    let (s, c) = theta.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]]
}

fn passive_z(theta: f64) -> Matrix {
    let (s, c) = theta.sin_cos();
    [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]]
}

fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| {
        std::array::from_fn(|j| a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j])
    })
}

fn shift_angles(original: NutationAngles, elapsed_tt_days: f64) -> NutationAngles {
    NutationAngles {
        mean_obliquity_rad: original.mean_obliquity_rad
            + original.mean_obliquity_rate_rad_per_tt_day * elapsed_tt_days,
        delta_psi_rad: original.delta_psi_rad
            + original.delta_psi_rate_rad_per_tt_day * elapsed_tt_days,
        delta_epsilon_rad: original.delta_epsilon_rad
            + original.delta_epsilon_rate_rad_per_tt_day * elapsed_tt_days,
        ..original
    }
}

#[test]
fn emitted_matrix_is_identical_to_existing_nutation_primitive() {
    let source = angles();
    let paired = nutation_with_true_obliquity(source).unwrap();
    assert_eq!(
        paired.nutation,
        nutation_matrix_from_angles(source).unwrap()
    );
    near(paired.true_obliquity_rad, 0.409_03, 1e-15);
    near(paired.true_obliquity_rate_rad_per_tt_day, -4.5e-7, 1e-21);
}

#[test]
fn true_obliquity_rate_is_independently_the_time_derivative_of_angle_sum() {
    let a = angles();
    let present = nutation_with_true_obliquity(a).unwrap();
    let h = 0.25;
    let before = nutation_with_true_obliquity(shift_angles(a, -h)).unwrap();
    let after = nutation_with_true_obliquity(shift_angles(a, h)).unwrap();
    near(
        present.true_obliquity_rate_rad_per_tt_day,
        (after.true_obliquity_rad - before.true_obliquity_rad) / (2.0 * h),
        2e-16,
    );
}

#[test]
fn final_ecliptic_tilt_cancels_all_delta_epsilon_from_nutation_matrix() {
    // E*N = Rx(+epsilon_true)*Rx(-epsilon_true)*Rz(-delta_psi)*Rx(epsilon_mean).
    // The cancellation must hold even when delta-epsilon != 0.
    let a = angles();
    let output = nutation_with_true_obliquity(a).unwrap();
    let applied = multiply(
        passive_x(output.true_obliquity_rad),
        output.nutation.rotation,
    );
    let expected = multiply(passive_z(-a.delta_psi_rad), passive_x(a.mean_obliquity_rad));
    for i in 0..3 {
        for j in 0..3 {
            near(applied[i][j], expected[i][j], 2e-15);
        }
    }
}

#[test]
fn ecliptic_orientation_and_rates_are_independent_of_delta_epsilon_history() {
    // Independent centered differences on the *fully combined*
    // equatorial nutation and ecliptic tilt. The time-varying
    // delta-epsilon and its derivative must cancel identically
    // from E*N and d(E*N)/dTT (in equinox-based convention).
    let a = angles();
    let at = |tt_day: f64| {
        let input = shift_angles(a, tt_day);
        let paired = nutation_with_true_obliquity(input).unwrap();
        multiply(
            passive_x(paired.true_obliquity_rad),
            paired.nutation.rotation,
        )
    };
    let h = 0.1_f64;
    let before = at(-h);
    let after = at(h);
    let finite_rate: Matrix =
        std::array::from_fn(|i| std::array::from_fn(|j| (after[i][j] - before[i][j]) / (2.0 * h)));
    // An independent analytical derivative of the simpler reduced
    // equinox-based product B=Rz(-psi)*Rx(eps_mean), evaluated by
    // centered differences, must match the full pipeline derivative.
    let expected_at = |tt_day: f64| {
        let input = shift_angles(a, tt_day);
        multiply(
            passive_z(-input.delta_psi_rad),
            passive_x(input.mean_obliquity_rad),
        )
    };
    let expected_before = expected_at(-h);
    let expected_after = expected_at(h);
    for i in 0..3 {
        for j in 0..3 {
            near(
                finite_rate[i][j],
                (expected_after[i][j] - expected_before[i][j]) / (2.0 * h),
                2e-15,
            );
        }
    }
}

#[test]
fn zero_nutation_keeps_mean_obliquity_and_its_rate_as_the_true_obliquity() {
    let mut a = angles();
    a.delta_psi_rad = 0.0;
    a.delta_psi_rate_rad_per_tt_day = 0.0;
    a.delta_epsilon_rad = 0.0;
    a.delta_epsilon_rate_rad_per_tt_day = 0.0;
    let paired = nutation_with_true_obliquity(a).unwrap();
    near(paired.true_obliquity_rad, a.mean_obliquity_rad, 0.0);
    near(
        paired.true_obliquity_rate_rad_per_tt_day,
        a.mean_obliquity_rate_rad_per_tt_day,
        0.0,
    );
    let expected_identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for (row, expected) in expected_identity.iter().enumerate() {
        for (column, value) in expected.iter().enumerate() {
            near(paired.nutation.rotation[row][column], *value, 2e-15);
            near(paired.nutation.rate_per_tt_day[row][column], 0.0, 2e-19);
        }
    }
}

#[test]
fn derived_true_obliquity_must_remain_strictly_positive_and_below_half_pi() {
    let mut a = angles();
    a.mean_obliquity_rad = 0.01;
    a.delta_epsilon_rad = -0.02;
    // Legacy matrix input guards alone accept these values; the new
    // paired output must not quietly return a negative true tilt.
    assert!(nutation_matrix_from_angles(a).is_ok());
    assert_eq!(
        nutation_with_true_obliquity(a),
        Err(NutationMatrixError::InvalidTrueObliquity)
    );

    a.mean_obliquity_rad = FRAC_PI_2 - 0.005;
    a.delta_epsilon_rad = 0.01;
    assert_eq!(
        nutation_with_true_obliquity(a),
        Err(NutationMatrixError::InvalidTrueObliquity)
    );
}

#[test]
fn original_invalid_angle_and_nonfinite_validation_is_reused() {
    let mut a = angles();
    a.delta_psi_rad = 0.1;
    assert_eq!(
        nutation_with_true_obliquity(a),
        Err(NutationMatrixError::AngleOutsideGuardedRange)
    );
    a = angles();
    a.delta_epsilon_rate_rad_per_tt_day = f64::NAN;
    assert_eq!(
        nutation_with_true_obliquity(a),
        Err(NutationMatrixError::NonFiniteInput)
    );
}

#[test]
fn extreme_but_finite_tt_rates_fail_closed_on_arithmetic_overflow() {
    let mut a = angles();
    a.mean_obliquity_rate_rad_per_tt_day = f64::MAX;
    a.delta_epsilon_rate_rad_per_tt_day = f64::MAX;
    assert_eq!(
        nutation_matrix_from_angles(a),
        Err(NutationMatrixError::ArithmeticOverflow)
    );
    assert_eq!(
        nutation_with_true_obliquity(a),
        Err(NutationMatrixError::ArithmeticOverflow)
    );
}

#[test]
fn analytic_equatorial_to_ecliptic_rate_matches_independent_time_differences() {
    let a = angles();
    let output = nutation_with_true_obliquity(a).unwrap();
    let eps = output.true_obliquity_rad;
    let eps_rate = output.true_obliquity_rate_rad_per_tt_day;
    let (s, c) = eps.sin_cos();
    let e_dot: Matrix = [
        [0.0; 3],
        [0.0, -eps_rate * s, eps_rate * c],
        [0.0, -eps_rate * c, -eps_rate * s],
    ];
    let e = passive_x(eps);
    let from_e_dot = multiply(e_dot, output.nutation.rotation);
    let from_n_dot = multiply(e, output.nutation.rate_per_tt_day);
    let combined_rate: Matrix =
        std::array::from_fn(|i| std::array::from_fn(|j| from_e_dot[i][j] + from_n_dot[i][j]));
    let h = 0.1_f64;
    let at = |dt: f64| {
        let shifted = nutation_with_true_obliquity(shift_angles(a, dt)).unwrap();
        multiply(
            passive_x(shifted.true_obliquity_rad),
            shifted.nutation.rotation,
        )
    };
    let before = at(-h);
    let after = at(h);
    for i in 0..3 {
        for j in 0..3 {
            near(
                combined_rate[i][j],
                (after[i][j] - before[i][j]) / (2.0 * h),
                4e-15,
            );
        }
    }
}
