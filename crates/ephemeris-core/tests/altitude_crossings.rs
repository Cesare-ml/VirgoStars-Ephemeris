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

//! Analytical, deterministic geometric altitude crossings tests.
//!
//! Added 2026-11-11: the IMCCE formula and spherical invariants are tested,
//! not real-world rise/set times, refraction or external planet positions.

use std::f64::consts::{FRAC_PI_2, PI};

use virgostars_ephemeris_core::altitude_crossings::{
    geometric_altitude_hour_angles, AltitudeCrossingError, AltitudeCrossings,
};

fn radians(degrees: f64) -> f64 {
    degrees.to_radians()
}

fn pair(value: AltitudeCrossings) -> (f64, f64) {
    match value {
        AltitudeCrossings::Two {
            rising_hour_angle_rad,
            setting_hour_angle_rad,
        } => (rising_hour_angle_rad, setting_hour_angle_rad),
        other => panic!("expected two geometric crossings, found {other:?}"),
    }
}

fn near(actual: f64, expected: f64, tol: f64) {
    assert!(
        (actual - expected).abs() < tol,
        "expected {expected}, got {actual}; tolerance {tol}"
    );
}

#[test]
fn equatorial_object_crosses_geometric_horizon_east_and_west() {
    let (rising, setting) = pair(geometric_altitude_hour_angles(0.0, 0.0, 0.0).unwrap());
    near(rising, -FRAC_PI_2, 1.0e-14);
    near(setting, FRAC_PI_2, 1.0e-14);
    assert!(rising < 0.0);
    assert!(setting > 0.0);
}

#[test]
fn midlatitude_zero_threshold_matches_independent_tangent_identity() {
    // At h0 = 0, the independently known equivalent identity is
    // cos(H) = -tan(latitude) * tan(declination).
    let latitude = radians(43.0);
    let declination = radians(23.0);
    let h = (-latitude.tan() * declination.tan()).acos();
    let (rising, setting) = pair(
        geometric_altitude_hour_angles(latitude, declination, 0.0).unwrap(),
    );
    near(rising, -h, 1.0e-14);
    near(setting, h, 1.0e-14);
    assert!(setting > FRAC_PI_2);
}

#[test]
fn custom_altitude_threshold_changes_geometric_crossing_angles() {
    // Useful for modeling a prescribed *geometric* twilight threshold;
    // this does not import atmospheric, terrain or lunar parallax models.
    let (rising, setting) =
        pair(geometric_altitude_hour_angles(0.0, 0.0, radians(-6.0)).unwrap());
    near(rising, -radians(96.0), 1.0e-14);
    near(setting, radians(96.0), 1.0e-14);
}

#[test]
fn polar_day_and_night_do_not_fabricate_crossings() {
    let latitude = radians(70.0);
    let obliquity_like_declination = radians(23.4);
    assert_eq!(
        geometric_altitude_hour_angles(latitude, obliquity_like_declination, 0.0),
        Ok(AltitudeCrossings::AlwaysAbove)
    );
    assert_eq!(
        geometric_altitude_hour_angles(latitude, -obliquity_like_declination, 0.0),
        Ok(AltitudeCrossings::AlwaysBelow)
    );
    // The classification mirrors across hemispheres.
    assert_eq!(
        geometric_altitude_hour_angles(-latitude, -obliquity_like_declination, 0.0),
        Ok(AltitudeCrossings::AlwaysAbove)
    );
}

#[test]
fn polar_and_celestial_pole_degeneracies_are_explicit() {
    assert_eq!(
        geometric_altitude_hour_angles(FRAC_PI_2, 0.0, 0.0),
        Ok(AltitudeCrossings::ConstantAtThreshold)
    );
    assert_eq!(
        geometric_altitude_hour_angles(FRAC_PI_2, radians(20.0), 0.0),
        Ok(AltitudeCrossings::AlwaysAbove)
    );
    assert_eq!(
        geometric_altitude_hour_angles(FRAC_PI_2, radians(-20.0), 0.0),
        Ok(AltitudeCrossings::AlwaysBelow)
    );
    assert_eq!(
        geometric_altitude_hour_angles(0.0, FRAC_PI_2, 0.0),
        Ok(AltitudeCrossings::ConstantAtThreshold)
    );
}

#[test]
fn upper_and_lower_meridian_grazing_are_not_two_crossings() {
    assert_eq!(
        geometric_altitude_hour_angles(0.0, 0.0, FRAC_PI_2),
        Ok(AltitudeCrossings::TangentAtUpperMeridian)
    );
    assert_eq!(
        geometric_altitude_hour_angles(0.0, 0.0, -FRAC_PI_2),
        Ok(AltitudeCrossings::TangentAtLowerMeridian)
    );
}

#[test]
fn crossings_satisfy_original_spherical_altitude_equation() {
    for (phi_deg, dec_deg, h0_deg) in [
        (0.0, 0.0, 0.0),
        (43.0, 23.0, 0.0),
        (-32.0, -15.0, -6.0),
        (52.0, -10.0, -12.0),
        (-15.0, 40.0, 3.0),
    ] {
        let phi = radians(phi_deg);
        let dec = radians(dec_deg);
        let target = radians(h0_deg).sin();
        let (rise, set) =
            pair(geometric_altitude_hour_angles(phi, dec, radians(h0_deg)).unwrap());
        near(rise, -set, 1.0e-14);
        assert!(rise > -PI && rise < 0.0);
        assert!(set > 0.0 && set < PI);
        for hour_angle in [rise, set] {
            let sin_altitude =
                phi.sin() * dec.sin() + phi.cos() * dec.cos() * hour_angle.cos();
            near(sin_altitude, target, 5.0e-15);
        }
    }
}

#[test]
fn invalid_angles_fail_without_silent_normalization() {
    let invalid = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, PI, -PI];
    for value in invalid {
        assert_eq!(
            geometric_altitude_hour_angles(value, 0.0, 0.0),
            Err(AltitudeCrossingError::InvalidLatitude)
        );
        assert_eq!(
            geometric_altitude_hour_angles(0.0, value, 0.0),
            Err(AltitudeCrossingError::InvalidDeclination)
        );
        assert_eq!(
            geometric_altitude_hour_angles(0.0, 0.0, value),
            Err(AltitudeCrossingError::InvalidThresholdAltitude)
        );
    }
}
