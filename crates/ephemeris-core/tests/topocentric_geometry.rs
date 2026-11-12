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

//! Geometric GCRS topocentric projection with independent SOFA/ERFA oracle.
//!
//! Original independent Rust tests added 2026-11-12; these validate
//! Euclidean geometry and integration, not JPL ephemeris or apparent sky.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::earth_rotation::EarthRotationError;
use virgostars_ephemeris_core::observer_gcrs::{
    wgs84_observer_to_gcrs, ObserverGcrsError, ObserverOrientationInputs, Wgs84ObserverSite,
};
use virgostars_ephemeris_core::topocentric_geometry::{
    geometric_topocentric_direction_gcrs, wgs84_topocentric_geometric_direction_gcrs,
    TopocentricGeometryError,
};

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}, tolerance {tolerance}"
    );
}

fn near_vector(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (a, b) in actual.into_iter().zip(expected) {
        near(a, b, tolerance);
    }
}

fn synthetic_orientation() -> ObserverOrientationInputs {
    // Synthetic zero CIP/EOP is explicit in this test, never a default.
    ObserverOrientationInputs {
        ut1_epoch: JulianDate::new(2_451_545, 0.5, TimeScale::Ut1).unwrap(),
        polar_xp_rad: 0.0,
        polar_yp_rad: 0.0,
        tio_locator_sp_rad: 0.0,
        cip_x: 0.0,
        cip_y: 0.0,
        cio_locator_s_rad: 0.0,
    }
}

fn equatorial_site() -> Wgs84ObserverSite {
    Wgs84ObserverSite {
        longitude_rad_east: 0.0,
        latitude_rad_north: 0.0,
        ellipsoidal_height_m: 0.0,
    }
}

#[test]
fn agrees_with_independent_sofa_erfa_cartesian_to_spherical_reference() {
    // Independently published SOFA/ERFA t_p2s in src/t_erfa_c.c:
    // input [100,-50,25]; theta=-0.4636476090008061162,
    // phi=0.2199879773954594463, r=114.5643923738960002.
    // VirgoStars uses RA normalized to [0,2pi), unlike signed theta.
    let v = geometric_topocentric_direction_gcrs([100.0, -50.0, 25.0], [0.0; 3]).unwrap();
    near(
        v.right_ascension_rad.unwrap(),
        TAU - 0.463_647_609_000_806_1,
        1.0e-14,
    );
    near(v.declination_rad, 0.219_987_977_395_459_45, 1.0e-14);
    near(v.distance_km, 114.564_392_373_896, 1.0e-12);
    near_vector(v.displacement_gcrs_km, [100.0, -50.0, 25.0], 0.0);
    let norm = v.unit_direction_gcrs.into_iter().map(|x| x * x).sum::<f64>();
    near(norm, 1.0, 5.0e-16);
}

#[test]
fn analytic_lunar_distance_scale_has_expected_geometric_parallax() {
    // Illustrative geometry ONLY, not a claim about an actual Moon epoch.
    // A geocentric target is on +X at 384400 km, observer is 6378.137 km
    // on +Y. At the geocenter RA=0, while from the site RA is westward
    // by atan2(observer_y, target_x) ~0.95 degrees.
    let distance_km = 384_400.0_f64;
    let radius_km = 6_378.137_f64;
    let result =
        geometric_topocentric_direction_gcrs([distance_km, 0.0, 0.0], [0.0, radius_km, 0.0])
            .unwrap();
    let parallax_rad = radius_km.atan2(distance_km);
    near(result.right_ascension_rad.unwrap(), TAU - parallax_rad, 1.0e-14);
    near(result.declination_rad, 0.0, 1.0e-14);
    near(result.distance_km, distance_km.hypot(radius_km), 1.0e-10);
    assert!(parallax_rad.to_degrees() > 0.9);
    assert!(parallax_rad.to_degrees() < 1.0);
}

#[test]
fn analytic_one_au_scale_produces_small_parallax_without_model_defaults() {
    // This is a geometric 1 AU target vector, not the real Sun position
    // or a light-time-corrected apparent observation.
    let one_au_km = 149_597_870.7_f64;
    let radius_km = 6_378.137_f64;
    let result =
        geometric_topocentric_direction_gcrs([one_au_km, 0.0, 0.0], [0.0, radius_km, 0.0])
            .unwrap();
    let angle_rad = radius_km.atan2(one_au_km);
    near(result.right_ascension_rad.unwrap(), TAU - angle_rad, 1.0e-14);
    // Convert to arcseconds only for this analytic test, not in core API.
    let arcsec = angle_rad.to_degrees() * 3_600.0;
    assert!(arcsec > 8.7);
    assert!(arcsec < 8.9);
}

#[test]
fn cardinal_quadrants_and_zero_ra_are_normalized() {
    for (position, expected_ra) in [
        ([1.0, 0.0, 0.0], 0.0),
        ([0.0, 1.0, 0.0], FRAC_PI_2),
        ([-1.0, 0.0, 0.0], PI),
        ([0.0, -1.0, 0.0], 3.0 * FRAC_PI_2),
    ] {
        let result = geometric_topocentric_direction_gcrs(position, [0.0; 3]).unwrap();
        near(result.right_ascension_rad.unwrap(), expected_ra, 1.0e-14);
        near(result.declination_rad, 0.0, 0.0);
        near(result.distance_km, 1.0, 0.0);
    }
}

#[test]
fn exact_celestial_poles_return_undefined_ra_but_defined_declination() {
    for (target, expected_dec) in [([0.0, 0.0, 10.0], FRAC_PI_2), ([0.0, 0.0, -5.0], -FRAC_PI_2)] {
        let result = geometric_topocentric_direction_gcrs(target, [0.0; 3]).unwrap();
        assert_eq!(result.right_ascension_rad, None);
        near(result.declination_rad, expected_dec, 0.0);
        near(result.distance_km, target[2].abs(), 0.0);
    }
}

#[test]
fn origin_translation_preserves_sightline_geometry() {
    let first =
        geometric_topocentric_direction_gcrs([100.0, -50.0, 25.0], [0.0; 3]).unwrap();
    let second =
        geometric_topocentric_direction_gcrs([1100.0, 1950.0, 3025.0], [1000.0, 2000.0, 3000.0])
            .unwrap();
    assert_eq!(first, second);
}

#[test]
fn rejects_coincident_invalid_and_overflow_vectors_without_nan_outputs() {
    assert_eq!(
        geometric_topocentric_direction_gcrs([2.0, 3.0, 4.0], [2.0, 3.0, 4.0]),
        Err(TopocentricGeometryError::CoincidentTargetAndObserver)
    );
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            geometric_topocentric_direction_gcrs([bad, 0.0, 0.0], [0.0; 3]),
            Err(TopocentricGeometryError::NonFiniteTarget)
        );
        assert_eq!(
            geometric_topocentric_direction_gcrs([1.0, 0.0, 0.0], [bad, 0.0, 0.0]),
            Err(TopocentricGeometryError::NonFiniteObserver)
        );
    }
    assert_eq!(
        geometric_topocentric_direction_gcrs(
            [f64::MAX, 0.0, 0.0],
            [-f64::MAX, 0.0, 0.0]
        ),
        Err(TopocentricGeometryError::NonFiniteDisplacement)
    );
    assert_eq!(
        geometric_topocentric_direction_gcrs([f64::MAX, f64::MAX, 0.0], [0.0; 3]),
        Err(TopocentricGeometryError::InvalidDistance)
    );
}

#[test]
fn wgs84_wrapper_uses_integrated_site_gcrs_at_supplied_ut1_epoch() {
    let params = synthetic_orientation();
    let site = equatorial_site();
    let observer = wgs84_observer_to_gcrs(site, params).unwrap();
    // Supply a synthetic Earth-centered target exactly 100,-50,25 km
    // from this site's GCRS observer, rather than inventing an ephemeris.
    let target = std::array::from_fn(|i| observer.position_gcrs_km[i] + [100.0, -50.0, 25.0][i]);
    let result = wgs84_topocentric_geometric_direction_gcrs(site, params, target).unwrap();
    assert_eq!(result.observer, observer);
    near_vector(result.direction.displacement_gcrs_km, [100.0, -50.0, 25.0], 2.0e-12);
    near(result.direction.distance_km, 114.564_392_373_896, 5.0e-12);
    near(
        result.direction.right_ascension_rad.unwrap(),
        TAU - 0.463_647_609_000_806_1,
        5.0e-14,
    );
}

#[test]
fn high_level_wrapper_rejects_bad_epoch_and_nonfinite_target() {
    let mut params = synthetic_orientation();
    params.ut1_epoch = JulianDate::new(2_451_545, 0.5, TimeScale::Tt).unwrap();
    assert_eq!(
        wgs84_topocentric_geometric_direction_gcrs(
            equatorial_site(),
            params,
            [1.0, 0.0, 0.0]
        ),
        Err(TopocentricGeometryError::Observer(
            ObserverGcrsError::EarthRotation(EarthRotationError::RequiresUt1(TimeScale::Tt))
        ))
    );
    assert_eq!(
        wgs84_topocentric_geometric_direction_gcrs(
            equatorial_site(),
            synthetic_orientation(),
            [f64::NAN, 0.0, 0.0]
        ),
        Err(TopocentricGeometryError::NonFiniteTarget)
    );
}
