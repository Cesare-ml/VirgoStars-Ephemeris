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

//! WGS84 observer-position checks against the independent ERFA t_gd2gc.
//! Created 2026-11-11. These do not validate an Earth-orientation matrix.

use std::f64::consts::{FRAC_PI_2, PI};

use virgostars_ephemeris_core::wgs84_ecef::{
    wgs84_geodetic_to_ecef, Wgs84Error, WGS84_EQUATORIAL_RADIUS_M, WGS84_INVERSE_FLATTENING,
};

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} meters, obtained {actual} meters (tol {tolerance})"
    );
}

#[test]
fn independent_erfa_wgs84_geodetic_reference_agrees() {
    // ERFA src/t_erfa_c.c, t_gd2gc:
    // eraGd2gc(ERFA_WGS84, e=3.1, p=-0.5, h=2500.0, xyz).
    // The independent published XYZ values are expressed in meters.
    // https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c
    let xyz = wgs84_geodetic_to_ecef(3.1, -0.5, 2500.0).unwrap();
    near(xyz.x_m, -5_599_000.557_704_994_7, 1.0e-7);
    near(xyz.y_m, 233_011.672_234_792_03, 1.0e-7);
    near(xyz.z_m, -3_040_909.470_698_336_3, 1.0e-7);
}

#[test]
fn equator_and_cardinal_longitudes_match_ellipsoid_axes() {
    let greenwich = wgs84_geodetic_to_ecef(0.0, 0.0, 0.0).unwrap();
    near(greenwich.x_m, WGS84_EQUATORIAL_RADIUS_M, 1.0e-9);
    near(greenwich.y_m, 0.0, 1.0e-9);
    near(greenwich.z_m, 0.0, 1.0e-9);

    let east = wgs84_geodetic_to_ecef(FRAC_PI_2, 0.0, 0.0).unwrap();
    near(east.x_m, 0.0, 1.0e-8);
    near(east.y_m, WGS84_EQUATORIAL_RADIUS_M, 1.0e-9);
    near(east.z_m, 0.0, 1.0e-9);

    let west = wgs84_geodetic_to_ecef(-FRAC_PI_2, 0.0, 0.0).unwrap();
    near(west.y_m, -WGS84_EQUATORIAL_RADIUS_M, 1.0e-9);
    near(west.x_m, 0.0, 1.0e-8);
}

#[test]
fn poles_have_semi_minor_axis_radius_and_arbitrary_longitudes() {
    let flattening = 1.0 / WGS84_INVERSE_FLATTENING;
    let polar_radius = WGS84_EQUATORIAL_RADIUS_M * (1.0 - flattening);
    for longitude in [0.0, 1.3, -PI, PI] {
        let north = wgs84_geodetic_to_ecef(longitude, FRAC_PI_2, 0.0).unwrap();
        let south = wgs84_geodetic_to_ecef(longitude, -FRAC_PI_2, 0.0).unwrap();
        near(north.x_m, 0.0, 1.0e-8);
        near(north.y_m, 0.0, 1.0e-8);
        near(north.z_m, polar_radius, 1.0e-8);
        near(south.z_m, -polar_radius, 1.0e-8);
    }
}

#[test]
fn ellipsoid_height_changes_ecef_position_along_geodetic_normal() {
    let lon: f64 = 1.1;
    let lat: f64 = 0.6;
    let origin = wgs84_geodetic_to_ecef(lon, lat, 0.0).unwrap();
    let raised = wgs84_geodetic_to_ecef(lon, lat, 1000.0).unwrap();
    near(
        raised.x_m - origin.x_m,
        1000.0 * lat.cos() * lon.cos(),
        2.0e-9,
    );
    near(
        raised.y_m - origin.y_m,
        1000.0 * lat.cos() * lon.sin(),
        2.0e-9,
    );
    near(raised.z_m - origin.z_m, 1000.0 * lat.sin(), 2.0e-9);
}

#[test]
fn geocentric_latitude_differs_from_geodetic_latitude() {
    let latitude: f64 = 0.65;
    let point = wgs84_geodetic_to_ecef(0.0, latitude, 0.0).unwrap();
    let geocentric_latitude = point.z_m.atan2(point.x_m.hypot(point.y_m));
    assert!(geocentric_latitude > 0.0);
    assert!(geocentric_latitude < latitude);

    let south = wgs84_geodetic_to_ecef(0.0, -latitude, 0.0).unwrap();
    near(south.x_m, point.x_m, 1.0e-9);
    near(south.y_m, point.y_m, 1.0e-9);
    near(south.z_m, -point.z_m, 1.0e-9);
}

#[test]
fn output_kilometers_retain_same_earth_fixed_axes() {
    let xyz = wgs84_geodetic_to_ecef(-1.2, 0.4, -100.0).unwrap();
    let km = xyz.kilometers();
    near(km[0] * 1000.0, xyz.x_m, 1.0e-9);
    near(km[1] * 1000.0, xyz.y_m, 1.0e-9);
    near(km[2] * 1000.0, xyz.z_m, 1.0e-9);
    let low = wgs84_geodetic_to_ecef(0.0, 0.0, -12_000.0).unwrap();
    let high = wgs84_geodetic_to_ecef(0.0, 0.0, 100_000.0).unwrap();
    near(low.x_m, WGS84_EQUATORIAL_RADIUS_M - 12_000.0, 1.0e-9);
    near(high.x_m, WGS84_EQUATORIAL_RADIUS_M + 100_000.0, 1.0e-9);
}

#[test]
fn invalid_angles_and_heights_are_rejected_explicitly() {
    for invalid in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        PI + 0.01,
        -PI - 0.01,
    ] {
        assert_eq!(
            wgs84_geodetic_to_ecef(invalid, 0.0, 0.0),
            Err(Wgs84Error::InvalidLongitude)
        );
    }
    for invalid in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        FRAC_PI_2 + 0.01,
        -FRAC_PI_2 - 0.01,
    ] {
        assert_eq!(
            wgs84_geodetic_to_ecef(0.0, invalid, 0.0),
            Err(Wgs84Error::InvalidLatitude)
        );
    }
    for invalid in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -12_000.1,
        100_000.1,
    ] {
        assert_eq!(
            wgs84_geodetic_to_ecef(0.0, 0.0, invalid),
            Err(Wgs84Error::InvalidHeight)
        );
    }
}
