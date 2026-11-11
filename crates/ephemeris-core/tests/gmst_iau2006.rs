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

//! Independently published ERFA GMST06 reference and strict-contract tests.
//! Added 2026-11-11. No external astronomy library required at test runtime.

use std::f64::consts::TAU;

use virgostars_ephemeris_core::calendar::{CalendarError, JulianDate, TimeScale};
use virgostars_ephemeris_core::gmst_iau2006::{
    iau2006_gmst_correction, iau2006_gmst_from_era, GmstError,
};

fn tt(day_number: i64, fraction: f64) -> JulianDate {
    JulianDate::new(day_number, fraction, TimeScale::Tt).unwrap()
}

#[test]
fn matches_independent_erfa_gmst06_reference() {
    // Independently published ERFA t_gmst06:
    // eraGmst06(2400000.5, 53736.0, 2400000.5, 53736.0)
    // = 1.754174971870091203 radians, tolerance 1e-12.
    // https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c
    //
    // The ERA input at JD(UT1)=2453736.5 is derived independently from
    // the defining IAU 2000 ERA relationship, *not* from this GMST output.
    let era_ut1 = 1.752_833_255_303_073_2;
    let tt_epoch = tt(2_453_737, 0.0);
    let angle = iau2006_gmst_from_era(era_ut1, tt_epoch).unwrap();
    assert!(
        (angle - 1.754_174_971_870_091_2).abs() < 1.0e-12,
        "GMST06 = {angle} rad"
    );
}

#[test]
fn at_j2000_correction_is_iau2006_zero_epoch_bias() {
    let correction = iau2006_gmst_correction(tt(2_451_545, 0.5)).unwrap();
    let arcsec_to_rad = TAU / (360.0 * 3_600.0);
    assert!((correction.offset_rad - 0.014_506 * arcsec_to_rad).abs() < 1.0e-18);
    let expected_rate = 4_612.156_534 * arcsec_to_rad / 36_525.0;
    assert!((correction.offset_rate_rad_per_tt_day - expected_rate).abs() < 1.0e-18);
}

#[test]
fn derivative_agrees_with_symmetric_tt_difference() {
    let center = tt(2_460_000, 0.5);
    let width_days = 0.001;
    let before = tt(2_460_000, 0.5 - width_days);
    let after = tt(2_460_000, 0.5 + width_days);
    let rate = iau2006_gmst_correction(center)
        .unwrap()
        .offset_rate_rad_per_tt_day;
    let slope = (iau2006_gmst_correction(after).unwrap().offset_rad
        - iau2006_gmst_correction(before).unwrap().offset_rad)
        / (2.0 * width_days);
    assert!((slope - rate).abs() < 2.0e-15);
}

#[test]
fn normalization_does_not_confuse_mean_sidereal_time_with_era() {
    let era = TAU - 1.0e-8;
    let j2000 = tt(2_451_545, 0.5);
    let gmst = iau2006_gmst_from_era(era, j2000).unwrap();
    assert!(gmst >= 0.0 && gmst < TAU);
    let correction = iau2006_gmst_correction(j2000).unwrap();
    assert!((gmst - (era + correction.offset_rad).rem_euclid(TAU)).abs() < 1.0e-14);
    assert!((gmst - era).abs() > 1.0);
}

#[test]
fn rejects_invalid_era_and_non_tt_epochs() {
    let j2000 = tt(2_451_545, 0.5);
    for era in [-1.0, TAU, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            iau2006_gmst_from_era(era, j2000),
            Err(GmstError::InvalidEarthRotationAngle)
        );
    }
    for scale in [TimeScale::Tai, TimeScale::Ut1, TimeScale::Tdb] {
        let other = JulianDate::new(2_451_545, 0.5, scale).unwrap();
        assert_eq!(
            iau2006_gmst_correction(other),
            Err(GmstError::RequiresTt(scale))
        );
    }
    assert_eq!(
        iau2006_gmst_correction(tt(1_i64 << 52, 0.5)),
        Err(GmstError::InvalidEpoch(CalendarError::YearOutOfRange))
    );
}
