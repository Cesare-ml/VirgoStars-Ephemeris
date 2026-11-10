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

use std::f64::consts::PI;

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::mean_obliquity::{mean_obliquity_iau2006, MeanObliquityError};

fn tt_jd(julian_date: f64) -> JulianDate {
    JulianDate::from_f64(julian_date, TimeScale::Tt).unwrap()
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.3e}"
    );
}

#[test]
fn matches_independent_erfa_obl06_numerical_reference() {
    // ERFA's independent C test for eraObl06(2400000.5, 54388.0).
    // Source: https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c
    const ERFA_REFERENCE_RADIANS: f64 = 0.409_074_922_938_725_8;
    let actual = mean_obliquity_iau2006(tt_jd(2_400_000.5 + 54_388.0)).unwrap();
    near(actual.radians, ERFA_REFERENCE_RADIANS, 1e-14);
    assert!(actual.rate_radians_per_tt_day < 0.0);
}

#[test]
fn j2000_mean_obliquity_and_analytical_rate_match_iers_coefficients() {
    // IERS Technical Note 36, Eq. (5.40), 84381.406 arcseconds at J2000.
    let at_j2000 =
        mean_obliquity_iau2006(JulianDate::new(2_451_545, 0.5, TimeScale::Tt).unwrap()).unwrap();
    let arcsec_to_radians = PI / (180.0 * 3_600.0);
    near(at_j2000.radians, 84_381.406 * arcsec_to_radians, 1e-15);
    near(
        at_j2000.rate_radians_per_tt_day,
        -46.836_769 * arcsec_to_radians / 36_525.0,
        1e-20,
    );
}

#[test]
fn analytic_tt_rate_matches_centered_date_differences_across_centuries() {
    // Independently evaluate the obliquity at displaced TT dates rather than
    // calling the derivative recurrence again. This tests the chain rule
    // from TT Julian centuries to radians per TT day.
    for epoch in [2_451_545.0, 2_454_388.5, 2_470_000.0, 2_400_000.0] {
        let h = 5.0;
        let current = mean_obliquity_iau2006(tt_jd(epoch)).unwrap();
        let earlier = mean_obliquity_iau2006(tt_jd(epoch - h)).unwrap();
        let later = mean_obliquity_iau2006(tt_jd(epoch + h)).unwrap();
        near(
            current.rate_radians_per_tt_day,
            (later.radians - earlier.radians) / (2.0 * h),
            1e-14,
        );
    }
}

#[test]
fn split_julian_date_preserves_subsecond_angle_change() {
    let noon = JulianDate::new(2_451_545, 0.5, TimeScale::Tt).unwrap();
    let quarter_second_later =
        JulianDate::new(2_451_545, 0.5 + 0.25 / 86_400.0, TimeScale::Tt).unwrap();
    let baseline = mean_obliquity_iau2006(noon).unwrap();
    let later = mean_obliquity_iau2006(quarter_second_later).unwrap();
    assert!(later.radians < baseline.radians);
    let change = (later.radians - baseline.radians).abs();
    assert!(
        change > 1e-14 && change < 3e-14,
        "subsecond change = {change}"
    );
}

#[test]
fn rejects_wrong_time_scales_without_silent_tt_conversions() {
    for wrong in [TimeScale::Tdb, TimeScale::Tai, TimeScale::Ut1] {
        let date = JulianDate::new(2_451_545, 0.5, wrong).unwrap();
        assert_eq!(
            mean_obliquity_iau2006(date),
            Err(MeanObliquityError::WrongTimeScale),
        );
    }
}

#[test]
fn rejects_epochs_outside_conservative_scientific_test_window() {
    // ±2 TT Julian centuries after J2000 is a deliberately bounded initial
    // engineering scope, NOT a claim that the IAU formula stops being valid.
    for direction in [-1.0, 1.0] {
        assert!(mean_obliquity_iau2006(tt_jd(2_451_545.0 + direction * 73_050.0)).is_ok());
        assert_eq!(
            mean_obliquity_iau2006(tt_jd(2_451_545.0 + direction * 73_051.0)),
            Err(MeanObliquityError::OutsideTestedInterval),
        );
    }
}

#[test]
fn rejects_unrepresentable_split_tt_day() {
    let huge = JulianDate::new(1_i64 << 52, 0.5, TimeScale::Tt).unwrap();
    assert_eq!(
        mean_obliquity_iau2006(huge),
        Err(MeanObliquityError::UnrepresentableEpoch),
    );
}
