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

//! Equatorial-to-horizontal tests: independent ERFA oracle and spherical invariants.
//! Added 2026-11-11. No Swiss implementation or external runtime dependency.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use virgostars_ephemeris_core::equatorial_horizontal::{
    equatorial_to_horizontal, HorizontalError,
};

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} rad, obtained {actual} rad; tolerance {tolerance}"
    );
}

#[test]
fn agrees_with_independently_published_sofa_erfa_hd2ae_fixture() {
    // From liberfa/erfa src/t_erfa_c.c, t_hd2ae:
    // eraHd2ae(hour_angle=1.1, declination=1.2, latitude=0.3).
    // These are independently published output values, not produced by
    // this Rust implementation or the experimental first_sky module.
    let result = equatorial_to_horizontal(1.1, 1.2, 0.3).unwrap();
    near(result.azimuth_rad.unwrap(), 5.916_889_243_730_066, 1.0e-13);
    near(result.altitude_rad, 0.447_218_630_499_048_6, 1.0e-14);
}

#[test]
fn equatorial_horizon_crossings_point_due_east_and_west() {
    let east = equatorial_to_horizontal(-FRAC_PI_2, 0.0, 0.0).unwrap();
    near(east.altitude_rad, 0.0, 1.0e-14);
    near(east.azimuth_rad.unwrap(), FRAC_PI_2, 1.0e-14);

    let west = equatorial_to_horizontal(FRAC_PI_2, 0.0, 0.0).unwrap();
    near(west.altitude_rad, 0.0, 1.0e-14);
    near(west.azimuth_rad.unwrap(), 3.0 * FRAC_PI_2, 1.0e-14);
}

#[test]
fn upper_meridian_points_south_or_north_by_hemisphere() {
    let northern = equatorial_to_horizontal(0.0, 0.0, FRAC_PI_2 / 2.0).unwrap();
    near(northern.altitude_rad, FRAC_PI_2 / 2.0, 1.0e-14);
    near(northern.azimuth_rad.unwrap(), PI, 1.0e-14);

    let southern = equatorial_to_horizontal(0.0, 0.0, -FRAC_PI_2 / 2.0).unwrap();
    near(southern.altitude_rad, FRAC_PI_2 / 2.0, 1.0e-14);
    near(southern.azimuth_rad.unwrap(), 0.0, 1.0e-14);
}

#[test]
fn zenith_and_nadir_are_bearing_singularities() {
    let zenith = equatorial_to_horizontal(0.0, 0.0, 0.0).unwrap();
    near(zenith.altitude_rad, FRAC_PI_2, 1.0e-14);
    assert_eq!(zenith.azimuth_rad, None);

    // The signed-hour-angle convention is [-pi, pi), so use -pi.
    let nadir = equatorial_to_horizontal(-PI, 0.0, 0.0).unwrap();
    near(nadir.altitude_rad, -FRAC_PI_2, 1.0e-14);
    assert_eq!(nadir.azimuth_rad, None);
}

#[test]
fn exact_polar_sites_have_no_unique_true_north_azimuth() {
    let north = equatorial_to_horizontal(1.1, 20.0_f64.to_radians(), FRAC_PI_2).unwrap();
    near(north.altitude_rad, 20.0_f64.to_radians(), 1.0e-14);
    assert_eq!(north.azimuth_rad, None);

    let south = equatorial_to_horizontal(1.1, 20.0_f64.to_radians(), -FRAC_PI_2).unwrap();
    near(south.altitude_rad, -20.0_f64.to_radians(), 1.0e-14);
    assert_eq!(south.azimuth_rad, None);
}

#[test]
fn altitude_matches_spherical_identity_at_arbitrary_latitudes() {
    for (hour_angle, declination, latitude) in [
        (0.3_f64, 0.4_f64, 0.7_f64),
        (-1.1, -0.9, -0.4),
        (2.0, 0.2, -0.7),
        (-2.7, 0.5, 0.2),
    ] {
        let value = equatorial_to_horizontal(hour_angle, declination, latitude).unwrap();
        let sine_altitude = latitude.sin() * declination.sin()
            + latitude.cos() * declination.cos() * hour_angle.cos();
        near(value.altitude_rad.sin(), sine_altitude, 1.0e-14);
        assert!((-FRAC_PI_2..=FRAC_PI_2).contains(&value.altitude_rad));
        assert!(value.azimuth_rad.is_some());
        let az = value.azimuth_rad.unwrap();
        assert!((0.0..TAU).contains(&az));
    }
}

#[test]
fn mirror_symmetry_preserves_altitude_and_reflects_azimuth() {
    let eastward = equatorial_to_horizontal(-0.9, 0.3, 0.6).unwrap();
    let westward = equatorial_to_horizontal(0.9, 0.3, 0.6).unwrap();
    near(eastward.altitude_rad, westward.altitude_rad, 1.0e-14);
    let a = eastward.azimuth_rad.unwrap();
    let b = westward.azimuth_rad.unwrap();
    near((a + b + PI).rem_euclid(TAU) - PI, 0.0, 1.0e-14);
}

#[test]
fn malformed_angles_fail_without_implicit_wrapping() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -PI - 0.01, PI] {
        assert_eq!(
            equatorial_to_horizontal(bad, 0.0, 0.0),
            Err(HorizontalError::InvalidHourAngle)
        );
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -PI, PI] {
        assert_eq!(
            equatorial_to_horizontal(0.0, bad, 0.0),
            Err(HorizontalError::InvalidDeclination)
        );
        assert_eq!(
            equatorial_to_horizontal(0.0, 0.0, bad),
            Err(HorizontalError::InvalidLatitude)
        );
    }
}
