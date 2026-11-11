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

//! IAU 2000 ERA reference, split-date precision and failure-mode checks.
//!
//! Added 2026-11-11. The external SOFA/ERFA fixture is independently
//! published; synthetic invariants below are not ephemeris validation.

use std::f64::consts::TAU;

use virgostars_ephemeris_core::calendar::{CalendarError, JulianDate, TimeScale};
use virgostars_ephemeris_core::earth_rotation::{
    earth_rotation_angle, EarthRotationError,
};

fn epoch(day_number: i64, fraction: f64) -> JulianDate {
    JulianDate::new(day_number, fraction, TimeScale::Ut1).unwrap()
}

fn circular_difference(a: f64, b: f64) -> f64 {
    (a - b + TAU / 2.0).rem_euclid(TAU) - TAU / 2.0
}

#[test]
fn era_matches_independent_sofa_erfa_reference() {
    // SOFA/ERFA t_era00: eraEra00(2400000.5, 54388.0).
    // See https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c
    // The UT1 JD is 2454388.5 = day number 2454389 at midnight.
    let result = earth_rotation_angle(epoch(2_454_389, 0.0)).unwrap();
    let reference = 0.402_283_724_002_815_8;
    assert!(
        (result.angle_rad - reference).abs() < 1.0e-12,
        "ERA = {}, independently published reference = {reference}",
        result.angle_rad
    );
}

#[test]
fn j2000_noon_and_full_day_advancement_match_iau_definition() {
    let j2000 = earth_rotation_angle(epoch(2_451_545, 0.5)).unwrap();
    assert!((j2000.angle_rad - TAU * 0.779_057_273_264_0).abs() < 1.0e-14);
    assert!(j2000.angle_rad >= 0.0 && j2000.angle_rad < TAU);

    let next_day = earth_rotation_angle(epoch(2_451_546, 0.5)).unwrap();
    let expected = TAU * 0.002_737_811_911_354_48;
    assert!((circular_difference(next_day.angle_rad, j2000.angle_rad) - expected).abs() < 1.0e-13);

    let expected_rate = TAU * 1.002_737_811_911_354_48;
    assert!((next_day.rate_rad_per_ut1_day - expected_rate).abs() < 1.0e-14);
    assert_eq!(next_day.rate_rad_per_ut1_day, j2000.rate_rad_per_ut1_day);
}

#[test]
fn subsecond_split_epoch_is_not_rounded_to_full_julian_date() {
    let base = epoch(2_451_545, 0.5);
    let shifted = epoch(2_451_545, 0.5 + 0.000_001 / 86_400.0);
    let before = earth_rotation_angle(base).unwrap();
    let after = earth_rotation_angle(shifted).unwrap();
    let expected_delta =
        before.rate_rad_per_ut1_day * (shifted.midnight_fraction() - base.midnight_fraction());
    let measured_delta = circular_difference(after.angle_rad, before.angle_rad);
    assert!(measured_delta > 0.0);
    assert!((measured_delta - expected_delta).abs() < 2.0e-15);

    // The separate single-f64 Julian day discards this microsecond offset.
    let (first, second) = shifted.parts().unwrap();
    assert_eq!(first + second, 2_451_545.0);
}

#[test]
fn midnight_boundary_and_negative_epochs_are_continuous_and_normalized() {
    let just_before = epoch(2_451_545, 1.0 - 0.000_01 / 86_400.0);
    let midnight = epoch(2_451_546, 0.0);
    let a = earth_rotation_angle(just_before).unwrap();
    let b = earth_rotation_angle(midnight).unwrap();
    assert!(a.angle_rad >= 0.0 && a.angle_rad < TAU);
    assert!(b.angle_rad >= 0.0 && b.angle_rad < TAU);
    let expected_delta = a.rate_rad_per_ut1_day
        * (1.0 - just_before.midnight_fraction());
    assert!((circular_difference(b.angle_rad, a.angle_rad) - expected_delta).abs() < 2.0e-14);

    for (day_number, fraction) in [(0, 0.0), (-1_000_000, 0.75), (1_000_000, 0.25)] {
        let result = earth_rotation_angle(epoch(day_number, fraction)).unwrap();
        assert!(result.angle_rad.is_finite());
        assert!(result.angle_rad >= 0.0 && result.angle_rad < TAU);
        assert!(result.rate_rad_per_ut1_day.is_finite());
    }
}

#[test]
fn refuses_mislabeled_scales_and_unrepresentable_dates() {
    for scale in [TimeScale::Tdb, TimeScale::Tt, TimeScale::Tai] {
        let non_ut1 = JulianDate::new(2_451_545, 0.5, scale).unwrap();
        assert_eq!(
            earth_rotation_angle(non_ut1),
            Err(EarthRotationError::RequiresUt1(scale))
        );
    }
    let huge = epoch(1_i64 << 52, 0.5);
    assert_eq!(
        earth_rotation_angle(huge),
        Err(EarthRotationError::InvalidDate(CalendarError::YearOutOfRange))
    );
}
