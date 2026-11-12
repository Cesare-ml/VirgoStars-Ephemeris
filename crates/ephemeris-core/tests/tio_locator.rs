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

//! Independent SOFA/ERFA SP00 fixture and secular-model guard tests.
//!
//! This validates the conventional **approximate** s′(TT) drift,
//! NOT the measured/fully modeled terrestrial-origin position.

use std::f64::consts::PI;

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::tio_locator::{
    approximate_tio_locator_sp00, TioLocatorError,
};

fn tt(day_number: i64, midnight_fraction: f64) -> JulianDate {
    JulianDate::new(day_number, midnight_fraction, TimeScale::Tt).unwrap()
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}, tolerance {tolerance}"
    );
}

#[test]
fn independent_erfa_sp00_fixture_matches_published_value() {
    // ERFA t_sp00, src/t_erfa_c.c:
    // TT = 2400000.5 + 52541.0 = 2452541.5 JD,
    // s' = -0.6216698469981019309e-11 radians.
    let result = approximate_tio_locator_sp00(tt(2_452_542, 0.0)).unwrap();
    close(result.sp_rad, -0.621_669_846_998_101_9e-11, 1.0e-21);
}

#[test]
fn j2000_tt_noon_is_exactly_zero_without_an_implicit_ut1_epoch() {
    let result = approximate_tio_locator_sp00(tt(2_451_545, 0.5)).unwrap();
    assert_eq!(result.sp_rad, 0.0);
    assert!(result.secular_rate_rad_per_tt_day < 0.0);
}

#[test]
fn century_drift_is_negative_47_microarcseconds_per_tt_century() {
    let radians_per_arcsecond = PI / (180.0 * 3_600.0);
    let drift = -47.0e-6 * radians_per_arcsecond;
    let past = approximate_tio_locator_sp00(tt(2_451_545 - 36_525, 0.5)).unwrap();
    let future = approximate_tio_locator_sp00(tt(2_451_545 + 36_525, 0.5)).unwrap();
    close(future.sp_rad, drift, 1.0e-22);
    close(past.sp_rad, -drift, 1.0e-22);
    close(future.sp_rad, -past.sp_rad, 1.0e-22);
}

#[test]
fn reported_tt_day_rate_matches_finite_difference_of_approximation() {
    let before = approximate_tio_locator_sp00(tt(2_451_540, 0.5)).unwrap();
    let after = approximate_tio_locator_sp00(tt(2_451_550, 0.5)).unwrap();
    let mid = approximate_tio_locator_sp00(tt(2_451_545, 0.5)).unwrap();
    close(
        (after.sp_rad - before.sp_rad) / 10.0,
        mid.secular_rate_rad_per_tt_day,
        1.0e-28,
    );
}

#[test]
fn near_midnight_precision_survives_split_julian_day_arithmetic() {
    let midnight = approximate_tio_locator_sp00(tt(2_452_542, 0.0)).unwrap();
    let half_day_later = approximate_tio_locator_sp00(tt(2_452_542, 0.5)).unwrap();
    close(
        half_day_later.sp_rad - midnight.sp_rad,
        0.5 * midnight.secular_rate_rad_per_tt_day,
        1.0e-28,
    );
}

#[test]
fn explicit_tt_label_is_required_without_silent_scale_conversion() {
    for wrong in [TimeScale::Ut1, TimeScale::Tdb, TimeScale::Tai] {
        let epoch = JulianDate::new(2_451_545, 0.5, wrong).unwrap();
        assert_eq!(
            approximate_tio_locator_sp00(epoch),
            Err(TioLocatorError::RequiresTerrestrialTime(wrong))
        );
    }
}

#[test]
fn approximation_range_is_explicit_and_closed_at_two_centuries() {
    let past_boundary = tt(2_451_545 - 2 * 36_525, 0.5);
    let future_boundary = tt(2_451_545 + 2 * 36_525, 0.5);
    assert!(approximate_tio_locator_sp00(past_boundary).is_ok());
    assert!(approximate_tio_locator_sp00(future_boundary).is_ok());
    for epoch in [
        tt(2_451_545 - 2 * 36_525, 0.49),
        tt(2_451_545 + 2 * 36_525, 0.51),
        tt(2_451_545 + 500 * 36_525, 0.5),
    ] {
        assert_eq!(
            approximate_tio_locator_sp00(epoch),
            Err(TioLocatorError::BeyondApproximationRange)
        );
    }
}

#[test]
fn extreme_julian_day_integer_is_rejected_before_float_cast() {
    let extreme_negative = tt(i64::MIN, 0.5);
    let extreme_positive = tt(i64::MAX, 0.5);
    assert_eq!(
        approximate_tio_locator_sp00(extreme_negative),
        Err(TioLocatorError::InvalidEpoch)
    );
    assert_eq!(
        approximate_tio_locator_sp00(extreme_positive),
        Err(TioLocatorError::BeyondApproximationRange)
    );
}
