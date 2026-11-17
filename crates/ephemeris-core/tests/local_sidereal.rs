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

//! Local mean sidereal meridian and signed hour-angle regression tests.
//!
//! Created 2026-11-11: analytical geometry checks. These are not tests
//! of an independently implemented GMST time model or Swiss API parity.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use virgostars_ephemeris_core::local_sidereal::{
    local_mean_hour_angle, local_mean_sidereal_time, LocalSiderealError,
};

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 2.0e-14,
        "expected {expected} rad, obtained {actual} rad"
    );
}

#[test]
fn greenwich_and_east_west_meridians_follow_signed_longitude() {
    let gmst = 1.234_567_890_123;
    close(local_mean_sidereal_time(gmst, 0.0).unwrap(), gmst);
    // 15 degrees of east longitude is exactly 1/24 of a full turn,
    // or one sidereal hour in angular units.
    let quarter_hour = PI / 12.0;
    close(
        local_mean_sidereal_time(gmst, 15.0).unwrap(),
        gmst + quarter_hour,
    );
    close(
        local_mean_sidereal_time(gmst, -15.0).unwrap(),
        gmst - quarter_hour,
    );
}

#[test]
fn date_line_meridians_are_same_angle_and_wrap_to_zero() {
    close(
        local_mean_sidereal_time(0.0, 180.0).unwrap(),
        local_mean_sidereal_time(0.0, -180.0).unwrap(),
    );
    close(
        local_mean_sidereal_time(TAU - PI / 12.0, 15.0).unwrap(),
        0.0,
    );
    close(
        local_mean_sidereal_time(0.0, -15.0).unwrap(),
        TAU - PI / 12.0,
    );
}

#[test]
fn signed_hour_angle_is_zero_on_mean_meridian_transit() {
    let lmst = local_mean_sidereal_time(3.456, 35.0).unwrap();
    close(local_mean_hour_angle(lmst, lmst).unwrap(), 0.0);
    // Hour angles are positive west of the local meridian, negative east.
    close(local_mean_hour_angle(FRAC_PI_2, 0.0).unwrap(), FRAC_PI_2);
    close(local_mean_hour_angle(0.0, FRAC_PI_2).unwrap(), -FRAC_PI_2);
}

#[test]
fn signed_hour_angle_wraps_across_zero_and_has_explicit_opposition() {
    let small = 0.000_001;
    close(
        local_mean_hour_angle(small, TAU - small).unwrap(),
        2.0 * small,
    );
    close(
        local_mean_hour_angle(TAU - small, small).unwrap(),
        -2.0 * small,
    );
    close(local_mean_hour_angle(0.0, PI).unwrap(), -PI);
    close(local_mean_hour_angle(PI, 0.0).unwrap(), -PI);
}

#[test]
fn algebraic_consistency_holds_across_east_and_west_hemispheres() {
    for (gmst, longitude_east, mean_ra) in [
        (0.2, -175.0, 3.1),
        (TAU - 0.01, 179.0, 0.1),
        (1.5, -90.0, 2.1),
        (3.4, 120.0, 4.5),
        (5.5, 0.0, 2.0),
    ] {
        let local = local_mean_sidereal_time(gmst, longitude_east).unwrap();
        let hour_angle = local_mean_hour_angle(local, mean_ra).unwrap();
        assert!((-PI..PI).contains(&hour_angle));
        let reconstructed_local = (mean_ra + hour_angle).rem_euclid(TAU);
        close(reconstructed_local, local);
    }
}

#[test]
fn nonfinite_and_out_of_range_inputs_fail_explicitly() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.001, TAU] {
        assert_eq!(
            local_mean_sidereal_time(bad, 0.0),
            Err(LocalSiderealError::InvalidGreenwichMeanSiderealAngle)
        );
        assert_eq!(
            local_mean_hour_angle(bad, 1.0),
            Err(LocalSiderealError::InvalidLocalMeanSiderealAngle)
        );
        assert_eq!(
            local_mean_hour_angle(1.0, bad),
            Err(LocalSiderealError::InvalidMeanRightAscension)
        );
    }

    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -181.0, 181.0] {
        assert_eq!(
            local_mean_sidereal_time(1.0, bad),
            Err(LocalSiderealError::InvalidObserverLongitude)
        );
    }
}
