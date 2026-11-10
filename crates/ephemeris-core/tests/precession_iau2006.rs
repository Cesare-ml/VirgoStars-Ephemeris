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

//! Regression tests for IAU 2006 GCRS-to-mean-of-date bias/precession.
//!
//! The nine reference matrix values are independent published ERFA test
//! fixtures for eraPmat06(2400000.5, 50123.9999), not generated from this
//! implementation. No ERFA program code or ephemeris data is used.

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::mean_obliquity::MeanObliquityError;
use virgostars_ephemeris_core::precession_iau2006::precession_bias_iau2006;

fn tt_jd(value: f64) -> JulianDate {
    JulianDate::from_f64(value, TimeScale::Tt).unwrap()
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
fn matches_all_nine_independent_erfa_pmat06_reference_elements() {
    // ERFA t_pmat06, t_erfa_c.c. Absolute matrices operate in the sense
    // v(mean equator/equinox of date) = R * v(GCRS).
    // https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c
    const EXPECTED: [[f64; 3]; 3] = [
        [
            0.999_999_550_517_600_7,
            0.000_869_540_461_734_820_8,
            0.000_377_973_520_186_558_9,
        ],
        [
            -0.000_869_540_472_377_203_1,
            0.999_999_621_949_602_7,
            -0.000_000_136_175_249_708_027,
        ],
        [
            -0.000_377_973_495_703_408_95,
            -0.000_000_192_488_084_789_446,
            0.999_999_928_567_997_2,
        ],
    ];
    let actual = precession_bias_iau2006(tt_jd(2_400_000.5 + 50_123.9999)).unwrap();
    for (i, row) in EXPECTED.iter().enumerate() {
        for (j, expected) in row.iter().enumerate() {
            near(actual.rotation[i][j], *expected, 1e-13);
        }
    }
}

#[test]
fn analytic_rotation_derivative_agrees_with_independent_finite_differences() {
    // Different TT epochs spanning historical/modern/near-future use.
    // Finite differences recompute entire R at displaced epochs: the check
    // is sensitive to omitted terms in the Fukushima-Williams product rule.
    let half_step_tt_days = 0.75;
    for epoch in [2_415_000.0, 2_451_545.0, 2_459_000.0, 2_477_000.0] {
        let current = precession_bias_iau2006(tt_jd(epoch)).unwrap();
        let earlier = precession_bias_iau2006(tt_jd(epoch - half_step_tt_days)).unwrap();
        let later = precession_bias_iau2006(tt_jd(epoch + half_step_tt_days)).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                let numerical =
                    (later.rotation[i][j] - earlier.rotation[i][j]) / (2.0 * half_step_tt_days);
                near(current.rate_per_tt_day[i][j], numerical, 5e-13);
            }
        }
    }
}

#[test]
fn matrix_is_right_handed_and_derivative_is_tangent_to_so3() {
    for epoch in [2_415_000.0, 2_451_545.0, 2_459_000.0, 2_477_000.0] {
        let state = precession_bias_iau2006(tt_jd(epoch)).unwrap();
        let rotation = state.rotation;
        let derivative = state.rate_per_tt_day;
        for i in 0..3 {
            for j in 0..3 {
                near(
                    dot(rotation[i], rotation[j]),
                    if i == j { 1.0 } else { 0.0 },
                    2e-14,
                );
                near(
                    dot(derivative[i], rotation[j]) + dot(rotation[i], derivative[j]),
                    0.0,
                    1e-15,
                );
            }
        }
        let cross = [
            rotation[1][1] * rotation[2][2] - rotation[1][2] * rotation[2][1],
            rotation[1][2] * rotation[2][0] - rotation[1][0] * rotation[2][2],
            rotation[1][0] * rotation[2][1] - rotation[1][1] * rotation[2][0],
        ];
        near(dot(rotation[0], cross), 1.0, 2e-14);
    }
}

#[test]
fn frame_bias_is_not_erased_at_j2000() {
    let at_j2000 = precession_bias_iau2006(tt_jd(2_451_545.0)).unwrap();
    // Frame bias persists at the nominal precession reference epoch, so the
    // returned orientation must NOT incorrectly be set to exact identity.
    let sum_deviations: f64 = (0..3)
        .flat_map(|i| (0..3).map(move |j| (i, j)))
        .map(|(i, j)| {
            let identity = if i == j { 1.0 } else { 0.0 };
            (at_j2000.rotation[i][j] - identity).abs()
        })
        .sum();
    assert!(sum_deviations > 1e-7);
    assert!(at_j2000.rate_per_tt_day[0][1].abs() > 1e-8);
}

#[test]
fn rejects_non_tt_time_scales_and_unrepresentable_epoch() {
    for wrong in [TimeScale::Tai, TimeScale::Tdb, TimeScale::Ut1] {
        let date = JulianDate::new(2_451_545, 0.5, wrong).unwrap();
        assert_eq!(
            precession_bias_iau2006(date),
            Err(MeanObliquityError::WrongTimeScale),
        );
    }
    let huge = JulianDate::new(1_i64 << 52, 0.5, TimeScale::Tt).unwrap();
    assert_eq!(
        precession_bias_iau2006(huge),
        Err(MeanObliquityError::UnrepresentableEpoch),
    );
}

#[test]
fn refuses_extrapolation_beyond_current_engineering_test_window() {
    for sign in [-1.0, 1.0] {
        let boundary = 2_451_545.0 + sign * 73_050.0;
        assert!(precession_bias_iau2006(tt_jd(boundary)).is_ok());
        assert_eq!(
            precession_bias_iau2006(tt_jd(boundary + sign)),
            Err(MeanObliquityError::OutsideTestedInterval),
        );
    }
}
