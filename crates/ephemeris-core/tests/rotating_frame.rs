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

use std::f64::consts::{FRAC_PI_4, PI, TAU};
use virgostars_ephemeris_core::coordinates::project_fixed_axes;
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::rotating_frame::{RotatingFrame, RotatingFrameError};

fn state(position_km: [f64; 3], velocity_km_per_day: [f64; 3]) -> StateVector {
    StateVector {
        target: Body::Moon,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        position_km,
        velocity_km_per_day,
    }
}

fn passive_z(angle: f64, angular_rate: f64) -> RotatingFrame {
    let (sin, cos) = angle.sin_cos();
    RotatingFrame::new(
        [[cos, sin, 0.0], [-sin, cos, 0.0], [0.0, 0.0, 1.0]],
        [
            [-angular_rate * sin, angular_rate * cos, 0.0],
            [-angular_rate * cos, -angular_rate * sin, 0.0],
            [0.0, 0.0, 0.0],
        ],
    )
    .unwrap()
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual {actual:.14e}, expected {expected:.14e}, tolerance {tolerance:.4e}",
    );
}

#[test]
fn identity_axes_preserve_cartesian_state_and_metadata() {
    let original = state([120.0, 25.0, -7.0], [-8.0, 3.0, 0.7]);
    let actual = passive_z(0.0, 0.0).rotate_state(&original).unwrap();
    assert_eq!(actual, original);
}

#[test]
fn stationary_inertial_source_has_nonzero_rotating_longitude_rate() {
    // Inertially stationary source; axes rotate at +0.1 radians/TDB day.
    // A fixed source necessarily moves towards negative longitude in axes.
    let original = state([384_400.0, 0.0, 0.0], [0.0; 3]);
    let actual = passive_z(0.0, 0.1).rotate_state(&original).unwrap();
    assert_eq!(actual.position_km, original.position_km);
    near(actual.velocity_km_per_day[0], 0.0, 1e-12);
    near(actual.velocity_km_per_day[1], -38_440.0, 1e-10);
    let spherical = project_fixed_axes(&actual, 0.0).unwrap();
    near(spherical.longitude_rate_rad_per_day, -0.1, 1e-15);
    near(spherical.distance_rate_km_per_day, 0.0, 1e-10);
    // The incorrect fixed-axis rule R*v would return zero for this input.
}

#[test]
fn moving_source_and_axes_match_independent_centered_differences() {
    // Propagate constant-velocity ICRF geometry and turn the axes separately
    // at each sample. This reference intentionally does NOT use Rdot.
    let original = state([100.0, -220.0, 75.0], [2.0, 6.0, -0.8]);
    let theta0 = 2.7;
    let rate = 0.4;
    let at = |t: f64| {
        let mut sample = original;
        sample.position_km = std::array::from_fn(|i| {
            original.position_km[i] + original.velocity_km_per_day[i] * t
        });
        passive_z(theta0 + rate * t, rate)
            .rotate_state(&sample)
            .unwrap()
    };
    let now = at(0.0);
    let h = 1e-5;
    let before = at(-h);
    let after = at(h);
    for i in 0..3 {
        near(
            now.velocity_km_per_day[i],
            (after.position_km[i] - before.position_km[i]) / (2.0 * h),
            5e-9,
        );
    }

    let spherical_now = project_fixed_axes(&now, 0.0).unwrap();
    let spherical_before = project_fixed_axes(&before, 0.0).unwrap();
    let spherical_after = project_fixed_axes(&after, 0.0).unwrap();
    let signed_delta = (spherical_after.longitude_rad - spherical_before.longitude_rad + PI)
        .rem_euclid(TAU)
        - PI;
    near(
        spherical_now.longitude_rate_rad_per_day,
        signed_delta / (2.0 * h),
        2e-9,
    );
    near(
        spherical_now.latitude_rate_rad_per_day,
        (spherical_after.latitude_rad - spherical_before.latitude_rad) / (2.0 * h),
        2e-9,
    );
    near(
        spherical_now.distance_rate_km_per_day,
        (spherical_after.distance_km - spherical_before.distance_km) / (2.0 * h),
        2e-8,
    );
}

#[test]
fn rotating_axes_preserve_radius_and_physical_radial_speed() {
    let original = state([50.0, -200.0, 80.0], [20.0, -3.0, 1.0]);
    let source = project_fixed_axes(&original, 0.0).unwrap();
    for angle in [-1.0, 0.0, 1.0, PI] {
        let actual = passive_z(angle, 0.07).rotate_state(&original).unwrap();
        let sphere = project_fixed_axes(&actual, 0.0).unwrap();
        near(sphere.distance_km, source.distance_km, 1e-12);
        near(sphere.distance_rate_km_per_day, source.distance_rate_km_per_day, 1e-12);
    }
}

#[test]
fn rejects_reflections_nonorthogonal_matrices_and_incompatible_derivatives() {
    assert_eq!(
        RotatingFrame::new(
            [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [[0.0; 3]; 3],
        ),
        Err(RotatingFrameError::InvalidRotation),
    );
    assert_eq!(
        RotatingFrame::new(
            [[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [[0.0; 3]; 3],
        ),
        Err(RotatingFrameError::InvalidRotation),
    );
    assert_eq!(
        RotatingFrame::new(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [[0.1, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        ),
        Err(RotatingFrameError::InconsistentDerivative),
    );
}

#[test]
fn rejects_nonfinite_matrix_epoch_or_state_and_overflowed_output() {
    assert_eq!(
        RotatingFrame::new(
            [[f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [[0.0; 3]; 3],
        ),
        Err(RotatingFrameError::NonFinite),
    );
    assert_eq!(
        RotatingFrame::new(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [[0.0, f64::INFINITY, 0.0], [0.0; 3], [0.0; 3]],
        ),
        Err(RotatingFrameError::NonFinite),
    );
    let frame = passive_z(0.0, 0.0);
    let mut invalid_epoch = state([1.0, 0.0, 0.0], [0.0; 3]);
    invalid_epoch.epoch = JulianDateTdb(f64::NAN);
    assert_eq!(
        frame.rotate_state(&invalid_epoch),
        Err(RotatingFrameError::NonFinite),
    );
    let mut invalid_velocity = state([1.0, 0.0, 0.0], [0.0; 3]);
    invalid_velocity.velocity_km_per_day[2] = f64::INFINITY;
    assert_eq!(
        frame.rotate_state(&invalid_velocity),
        Err(RotatingFrameError::NonFinite),
    );
    let overflowed = state([f64::MAX, f64::MAX, 0.0], [0.0; 3]);
    assert_eq!(
        passive_z(FRAC_PI_4, 0.0).rotate_state(&overflowed),
        Err(RotatingFrameError::ArithmeticOverflow),
    );
}
