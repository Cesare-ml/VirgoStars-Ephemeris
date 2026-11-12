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

//! Integration tests for the opted-in IERS TT-based approximate s′
//! and the independently authored ITRS→TIRS polar-motion matrix.
//!
//! Numerical references are published ERFA *test values*, not code.
//! These tests do NOT validate live EOP sourcing or physical epochs.

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::polar_motion::{itrs_to_tirs_polar_motion_matrix, PolarMotionError};
use virgostars_ephemeris_core::tio_locator::{approximate_tio_locator_sp00, TioLocatorError};
use virgostars_ephemeris_core::tio_polar_motion::{
    itrs_to_tirs_polar_motion_with_approximate_sp00, ApproximateTioPolarMotionError,
};

const XP: f64 = 2.550_602_38e-7;
const YP: f64 = 1.860_359_247e-6;

fn tt(day_number: i64, fraction: f64) -> JulianDate {
    JulianDate::new(day_number, fraction, TimeScale::Tt).unwrap()
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, actual {actual}, tolerance {tolerance}"
    );
}

#[test]
fn published_erfa_sp00_and_pom00_fixtures_match_in_one_real_composition() {
    // ERFA t_pom00, src/t_erfa_c.c, independently published output
    // for xp,yp and s′=-0.1367174580728891460e-10 radians.
    //
    // That conventional s′ value is generated from
    // TT JD=2453736.5 (=2453737-0.5), exactly T=0.06 centuries.
    // SOFA/ERFA rpom is TIRS->ITRS. Our W is ITRS->TIRS,
    // so published rpom MUST be transposed.
    let output =
        itrs_to_tirs_polar_motion_with_approximate_sp00(tt(2_453_737, 0.0), XP, YP).unwrap();
    near(
        output.locator.sp_rad,
        -0.136_717_458_072_889_15e-10,
        1.0e-22,
    );
    let rpom = [
        [
            0.999_999_999_999_967_5,
            -0.136_717_458_072_884_7e-10,
            0.255_060_237_999_997_2e-6,
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
    for (i, row) in output.itrs_to_tirs.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            near(*value, rpom[j][i], 1.0e-15);
        }
    }
}

#[test]
fn another_independent_erfa_sp00_epoch_preserves_matrix_signs() {
    // ERFA t_sp00: TT=2400000.5+52541 = JD2452541.5.
    let result =
        itrs_to_tirs_polar_motion_with_approximate_sp00(tt(2_452_542, 0.0), 0.0, 0.0).unwrap();
    near(result.locator.sp_rad, -0.621_669_846_998_101_9e-11, 1.0e-21);
    let s = result.locator.sp_rad;
    near(result.itrs_to_tirs[0][0], s.cos(), 2.0e-16);
    near(result.itrs_to_tirs[1][0], s.sin(), 2.0e-20);
    near(result.itrs_to_tirs[0][1], -s.sin(), 2.0e-20);
    near(result.itrs_to_tirs[2][2], 1.0, 0.0);
}

#[test]
fn j2000_synthetic_zero_poles_have_identity_without_hidden_models() {
    // Only these explicitly supplied *synthetic* xp/yp values are zero.
    let output =
        itrs_to_tirs_polar_motion_with_approximate_sp00(tt(2_451_545, 0.5), 0.0, 0.0).unwrap();
    assert_eq!(output.locator.sp_rad, 0.0);
    assert_eq!(
        output.itrs_to_tirs,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    );
}

#[test]
fn opt_in_wrapper_matches_two_separately_implemented_modules() {
    for epoch in [
        tt(2_451_545, 0.5),
        tt(2_452_542, 0.0),
        tt(2_453_737, 0.0),
        tt(2_451_545 + 36_525, 0.123_456),
    ] {
        for (xp, yp) in [(0.0, 0.0), (XP, YP), (-0.005, 0.002)] {
            let output = itrs_to_tirs_polar_motion_with_approximate_sp00(epoch, xp, yp).unwrap();
            let from_tt = approximate_tio_locator_sp00(epoch).unwrap();
            let from_poles = itrs_to_tirs_polar_motion_matrix(xp, yp, from_tt.sp_rad).unwrap();
            assert_eq!(output.locator, from_tt);
            assert_eq!(output.itrs_to_tirs, from_poles);
        }
    }
}

#[test]
fn rejects_other_time_scales_without_implicit_tt_conversion() {
    for invalid in [TimeScale::Tai, TimeScale::Tdb, TimeScale::Ut1] {
        let epoch = JulianDate::new(2_453_737, 0.0, invalid).unwrap();
        assert_eq!(
            itrs_to_tirs_polar_motion_with_approximate_sp00(epoch, XP, YP),
            Err(ApproximateTioPolarMotionError::Locator(
                TioLocatorError::RequiresTerrestrialTime(invalid)
            ))
        );
    }
}

#[test]
fn refuses_unbounded_tt_extrapolation() {
    for epoch in [
        tt(2_451_545 - 2 * 36_525, 0.499),
        tt(2_451_545 + 2 * 36_525, 0.501),
    ] {
        assert_eq!(
            itrs_to_tirs_polar_motion_with_approximate_sp00(epoch, XP, YP),
            Err(ApproximateTioPolarMotionError::Locator(
                TioLocatorError::BeyondApproximationRange
            ))
        );
    }
}

#[test]
fn polar_motion_eop_values_still_require_finite_radian_units() {
    let epoch = tt(2_453_737, 0.0);
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0] {
        assert_eq!(
            itrs_to_tirs_polar_motion_with_approximate_sp00(epoch, bad, YP),
            Err(ApproximateTioPolarMotionError::PolarMotion(
                PolarMotionError::InvalidXp
            ))
        );
        assert_eq!(
            itrs_to_tirs_polar_motion_with_approximate_sp00(epoch, XP, bad),
            Err(ApproximateTioPolarMotionError::PolarMotion(
                PolarMotionError::InvalidYp
            ))
        );
    }
}

#[test]
fn composed_polar_matrix_preserves_position_norm_and_inverse_transpose() {
    let output =
        itrs_to_tirs_polar_motion_with_approximate_sp00(tt(2_453_737, 0.0), XP, YP).unwrap();
    let w = output.itrs_to_tirs;
    for input in [
        [6_378.137, -500.0, 2_000.0],
        [
            -5_599.000_557_704_994,
            233.011_672_234_792,
            -3_040.909_470_698,
        ],
    ] {
        let rotated: [f64; 3] = w.map(|row| row.iter().zip(input).map(|(x, y)| x * y).sum());
        let recovered: [f64; 3] =
            std::array::from_fn(|j| (0..3).map(|i| w[i][j] * rotated[i]).sum());
        for (value, expected) in recovered.into_iter().zip(input) {
            near(value, expected, 5.0e-12);
        }
        let norm = |position: [f64; 3]| position.into_iter().map(|x| x * x).sum::<f64>().sqrt();
        near(norm(rotated), norm(input), 5.0e-12);
    }
}
