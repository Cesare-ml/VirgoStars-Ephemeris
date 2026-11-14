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

use std::f64::consts::{FRAC_PI_2, PI};

use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::geometry::AngularSeparationError;
use virgostars_ephemeris_core::lunar_phase::{
    geometric_lunar_illumination, LunarIlluminationError,
};

fn state(target: Body, center: Body, epoch: f64, position_km: [f64; 3]) -> StateVector {
    StateVector {
        target,
        center,
        epoch: JulianDateTdb(epoch),
        position_km,
        velocity_km_per_day: [0.0; 3],
    }
}

fn pair(sun_position: [f64; 3], moon_position: [f64; 3]) -> (StateVector, StateVector) {
    (
        state(Body::Sun, Body::Earth, 2_451_545.0, sun_position),
        state(Body::Moon, Body::Earth, 2_451_545.0, moon_position),
    )
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:?}, expected={expected:?}, tolerance={tolerance:?}"
    );
}

#[test]
fn geometric_new_full_and_quarter_moon() {
    let (sun, moon) = pair([2.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    let new = geometric_lunar_illumination(&sun, &moon).unwrap();
    near(new.phase_angle_rad, PI, 0.0);
    near(new.illuminated_fraction, 0.0, 0.0);
    assert_eq!(new.epoch, sun.epoch);

    let (sun, moon) = pair([2.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);
    let full = geometric_lunar_illumination(&sun, &moon).unwrap();
    near(full.phase_angle_rad, 0.0, 0.0);
    near(full.illuminated_fraction, 1.0, 0.0);

    // Phase angle at the Moon is 90 degrees, although observer-centered
    // Sun–Moon elongation is only 45 degrees in this synthetic triangle.
    let (sun, moon) = pair([1.0, 1.0, 0.0], [1.0, 0.0, 0.0]);
    let quarter = geometric_lunar_illumination(&sun, &moon).unwrap();
    near(quarter.phase_angle_rad, FRAC_PI_2, 1e-15);
    near(quarter.illuminated_fraction, 0.5, 1e-15);
}

#[test]
fn near_new_moon_retains_tiny_positive_fraction() {
    let eps = 1e-6;
    let (sun, moon) = pair([2.0, eps, 0.0], [1.0, 0.0, 0.0]);
    let result = geometric_lunar_illumination(&sun, &moon).unwrap();
    // Analytic geometry: cos(alpha) = -1 / sqrt(1 + eps^2).
    // For eps << 1, illuminated fraction is approximately eps^2 / 4.
    near(result.illuminated_fraction, eps * eps / 4.0, 1e-16);
    assert!(result.illuminated_fraction > 0.0);
}

#[test]
fn rejects_incompatible_target_center_and_epoch() {
    let (mut sun, mut moon) = pair([2.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    sun.target = Body::Mercury;
    assert_eq!(
        geometric_lunar_illumination(&sun, &moon),
        Err(LunarIlluminationError::WrongTargetOrCenter)
    );
    sun.target = Body::Sun;
    moon.center = Body::SolarSystemBarycenter;
    assert_eq!(
        geometric_lunar_illumination(&sun, &moon),
        Err(LunarIlluminationError::WrongTargetOrCenter)
    );
    moon.center = Body::Earth;
    moon.epoch = JulianDateTdb(2_451_545.000_001);
    assert_eq!(
        geometric_lunar_illumination(&sun, &moon),
        Err(LunarIlluminationError::MismatchedEpoch)
    );
}

#[test]
fn rejects_nonfinite_and_undefined_input_geometries() {
    let (mut sun, mut moon) = pair([2.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    sun.epoch = JulianDateTdb(f64::NAN);
    assert_eq!(
        geometric_lunar_illumination(&sun, &moon),
        Err(LunarIlluminationError::NonFiniteInput)
    );
    sun.epoch = moon.epoch;
    sun.position_km[2] = f64::INFINITY;
    assert_eq!(
        geometric_lunar_illumination(&sun, &moon),
        Err(LunarIlluminationError::NonFiniteInput)
    );
    sun.position_km = [2.0, 0.0, 0.0];
    moon.position_km = [0.0; 3];
    assert_eq!(
        geometric_lunar_illumination(&sun, &moon),
        Err(LunarIlluminationError::Geometry(
            AngularSeparationError::UndefinedDirection
        ))
    );
    moon.position_km = sun.position_km;
    assert_eq!(
        geometric_lunar_illumination(&sun, &moon),
        Err(LunarIlluminationError::Geometry(
            AngularSeparationError::UndefinedDirection
        ))
    );
}

#[test]
fn rejects_finite_state_differences_that_overflow_f64() {
    let (sun, moon) = pair([f64::MAX, 0.0, 0.0], [-f64::MAX, 0.0, 0.0]);
    assert_eq!(
        geometric_lunar_illumination(&sun, &moon),
        Err(LunarIlluminationError::UnrepresentableDifference)
    );
}

#[test]
fn illumination_is_not_based_on_velocity_or_observer_elongation() {
    let (mut sun, mut moon) = pair([1.0, 1.0, 0.0], [1.0, 0.0, 0.0]);
    let expected = geometric_lunar_illumination(&sun, &moon).unwrap();
    sun.velocity_km_per_day = [1e5, 1e5, 1e5];
    moon.velocity_km_per_day = [-1e5, -1e5, -1e5];
    assert_eq!(geometric_lunar_illumination(&sun, &moon).unwrap(), expected);
    near(expected.illuminated_fraction, 0.5, 1e-15);
}
