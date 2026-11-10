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

use std::f64::consts::{FRAC_PI_2, TAU};

use virgostars_ephemeris_core::coordinates::{project_fixed_axes, ProjectionError};
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};

fn state(position_km: [f64; 3], velocity_km_per_day: [f64; 3]) -> StateVector {
    StateVector {
        target: Body::Moon,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        position_km,
        velocity_km_per_day,
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:?}, expected={expected:?}, tolerance={tolerance:?}"
    );
}

#[test]
fn circular_equatorial_motion_has_analytic_angular_rate() {
    let radius = 384_400.0;
    let angular_rate = 0.23;
    let projected = project_fixed_axes(
        &state([radius, 0.0, 0.0], [0.0, radius * angular_rate, 0.0]),
        0.0,
    )
    .unwrap();
    near(projected.longitude_rad, 0.0, 1e-15);
    near(projected.latitude_rad, 0.0, 1e-15);
    near(projected.distance_km, radius, 1e-9);
    near(projected.longitude_rate_rad_per_day, angular_rate, 1e-15);
    near(projected.latitude_rate_rad_per_day, 0.0, 1e-15);
    near(projected.distance_rate_km_per_day, 0.0, 1e-9);
    assert_eq!(projected.target, Body::Moon);
    assert_eq!(projected.center, Body::Earth);
}

#[test]
fn fixed_rotation_preserves_distance_and_radial_velocity() {
    let s = state([120_000.0, -280_000.0, 40_000.0], [800.0, 1_400.0, -500.0]);
    let reference = project_fixed_axes(&s, 0.0).unwrap();
    for tilt in [-1.2, -0.409, 0.0, 0.409, 1.2] {
        let rotated = project_fixed_axes(&s, tilt).unwrap();
        near(rotated.distance_km, reference.distance_km, 1e-9);
        near(
            rotated.distance_rate_km_per_day,
            reference.distance_rate_km_per_day,
            1e-11,
        );
    }
}

#[test]
fn finite_difference_matches_analytic_spherical_derivatives() {
    let position = [160_000.0, 240_000.0, 80_000.0];
    let velocity = [-1_200.0, 700.0, 300.0];
    let tilt = 0.409;
    let h = 1e-3;
    let at = |t: f64| {
        let shifted = std::array::from_fn(|i| position[i] + velocity[i] * t);
        project_fixed_axes(&state(shifted, velocity), tilt).unwrap()
    };
    let current = at(0.0);
    let before = at(-h);
    let after = at(h);
    let longitude_delta = ((after.longitude_rad - before.longitude_rad + std::f64::consts::PI)
        .rem_euclid(TAU))
        - std::f64::consts::PI;
    near(
        current.longitude_rate_rad_per_day,
        longitude_delta / (2.0 * h),
        1e-10,
    );
    near(
        current.latitude_rate_rad_per_day,
        (after.latitude_rad - before.latitude_rad) / (2.0 * h),
        1e-10,
    );
    near(
        current.distance_rate_km_per_day,
        (after.distance_km - before.distance_km) / (2.0 * h),
        1e-6,
    );
}

#[test]
fn rejects_undefined_poles_and_nonfinite_inputs() {
    assert_eq!(
        project_fixed_axes(&state([0.0, 0.0, 10.0], [0.0, 0.0, 1.0]), 0.0),
        Err(ProjectionError::UndefinedDirection)
    );
    assert_eq!(
        project_fixed_axes(&state([0.0, 0.0, 0.0], [0.0; 3]), 0.0),
        Err(ProjectionError::UndefinedDirection)
    );
    assert_eq!(
        project_fixed_axes(&state([1.0, 0.0, 0.0], [0.0; 3]), FRAC_PI_2 * f64::INFINITY),
        Err(ProjectionError::NonFinite)
    );
    assert_eq!(
        project_fixed_axes(&state([f64::NAN, 1.0, 0.0], [0.0; 3]), 0.0),
        Err(ProjectionError::NonFinite)
    );
}
