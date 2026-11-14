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

//! Independent mathematical oracle for the interior of Chebyshev records.
//! These deterministic identity checks do not validate real NASA/JPL data.

use virgostars_ephemeris_core::evaluate_chebyshev;

fn assert_near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual} (tolerance {tolerance})"
    );
}

#[test]
fn basis_values_and_derivatives_match_trigonometric_identity_inside_record() {
    // T_n(cos(theta)) = cos(n * theta).
    // T'_n(cos(theta)) = n * sin(n * theta) / sin(theta), for -1 < x < 1.
    // Unlike the production evaluator, this oracle does not use Clenshaw.
    let degrees = [0_usize, 1, 2, 3, 4, 8, 16, 32, 64];
    let positions: [f64; 7] = [-0.99, -0.8, -0.25, 0.0, 0.25, 0.8, 0.99];

    for degree in degrees {
        let mut coefficients = vec![0.0; degree + 1];
        coefficients[degree] = 1.0;
        for x in positions {
            let theta = x.acos();
            let n = degree as f64;
            let expected_value = (n * theta).cos();
            let expected_derivative = if degree == 0 {
                0.0
            } else {
                n * (n * theta).sin() / theta.sin()
            };
            let actual = evaluate_chebyshev(&coefficients, x).unwrap();
            assert_near(actual.value, expected_value, 2e-12);
            assert_near(actual.derivative_dx, expected_derivative, 2e-9);
        }
    }
}

#[test]
fn mixed_series_matches_direct_trigonometric_sum_for_position_and_slope() {
    // Distinct nonzero terms catch index-order/sign errors obscured by a
    // single-basis fixture. Derivative is with respect to normalized x.
    let coefficients: Vec<f64> = (0..=24)
        .map(|degree| {
            let sign = if degree % 2 == 0 { 1.0 } else { -1.0 };
            sign / (degree + 1) as f64
        })
        .collect();

    for x in [-0.93_f64, -0.42, 0.0, 0.31, 0.91] {
        let theta = x.acos();
        let expected_value: f64 = coefficients
            .iter()
            .enumerate()
            .map(|(degree, coefficient)| coefficient * (degree as f64 * theta).cos())
            .sum();
        let expected_derivative: f64 = coefficients
            .iter()
            .enumerate()
            .skip(1)
            .map(|(degree, coefficient)| {
                coefficient * degree as f64 * (degree as f64 * theta).sin() / theta.sin()
            })
            .sum();

        let actual = evaluate_chebyshev(&coefficients, x).unwrap();
        assert_near(actual.value, expected_value, 2e-12);
        assert_near(actual.derivative_dx, expected_derivative, 5e-10);
    }
}
