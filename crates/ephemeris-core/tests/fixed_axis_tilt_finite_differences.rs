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
// SPDX-License-Identifier: CPAL-1.0
//
// Modification (2026-11-14): add synthetic finite-difference checks for
// tilted fixed-axis spherical rates and longitude branch-cut continuity.

//! Independent centered-difference oracle for the public fixed-axis rates.
//! This only checks synthetic linear motion in a fixed ICRF Cartesian frame;
//! it does not validate a real DE441 kernel, an ecliptic-of-date model,
//! apparent positions, or JPL/Horizons numerical accuracy.

use std::f64::consts::TAU;

use virgostars_ephemeris_core::coordinates::project_fixed_axes;
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};

fn synthetic_state(position_km: [f64; 3], velocity_km_per_day: [f64; 3]) -> StateVector {
    StateVector {
        target: Body::Moon,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        position_km,
        velocity_km_per_day,
    }
}

fn shifted(mut state: StateVector, days: f64) -> StateVector {
    for (position, velocity) in state
        .position_km
        .iter_mut()
        .zip(state.velocity_km_per_day.iter())
    {
        *position += velocity * days;
    }
    state.epoch = JulianDateTdb(state.epoch.0 + days);
    state
}

fn centered_rates(state: StateVector, tilt_rad: f64, half_step_days: f64) -> [f64; 3] {
    let before = project_fixed_axes(&shifted(state, -half_step_days), tilt_rad).unwrap();
    let after = project_fixed_axes(&shifted(state, half_step_days), tilt_rad).unwrap();
    // atan2(sin(dlambda), cos(dlambda)) is the signed wrapped difference;
    // direct subtraction fails at the 0/2pi longitude branch cut.
    let delta_longitude = after.longitude_rad - before.longitude_rad;
    [
        delta_longitude.sin().atan2(delta_longitude.cos()) / (2.0 * half_step_days),
        (after.latitude_rad - before.latitude_rad) / (2.0 * half_step_days),
        (after.distance_km - before.distance_km) / (2.0 * half_step_days),
    ]
}

fn assert_near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.15e}, expected={expected:.15e}, tolerance={tolerance:.3e}"
    );
}

#[test]
fn nonzero_positive_and_negative_tilts_preserve_cartesian_motion_derivatives() {
    // The distance scales are illustrative, not real lunar or planetary states.
    // This independently differentiates a linear XYZ trajectory rather than
    // reconstructing velocities from the spherical result itself.
    for (position, velocity, tilt) in [
        (
            [137_000_000.0, -220_000_000.0, 39_000_000.0],
            [4_321.0, -6_543.0, 1_278.0],
            0.409_092_8,
        ),
        (
            [-380_000.0, 123_000.0, 34_000.0],
            [100.0, -350.0, 29.0],
            -0.27,
        ),
    ] {
        let input = synthetic_state(position, velocity);
        let projection = project_fixed_axes(&input, tilt).unwrap();
        let numeric = centered_rates(input, tilt, 0.01);
        assert_near(projection.longitude_rate_rad_per_day, numeric[0], 2.0e-11);
        assert_near(projection.latitude_rate_rad_per_day, numeric[1], 2.0e-11);
        assert_near(projection.distance_rate_km_per_day, numeric[2], 5.0e-4);
        assert_eq!(projection.target, input.target);
        assert_eq!(projection.center, input.center);
        assert_eq!(projection.epoch, input.epoch);
        assert_eq!(projection.tilt_rad, tilt);
    }
}

#[test]
fn tilted_longitude_rate_remains_continuous_across_zero_and_two_pi() {
    // The tiny negative Y component starts just below longitude 2pi.
    // With a positive Y velocity, the two finite-difference points straddle 0.
    let state = synthetic_state([1_000_000.0, -0.1, 0.0], [0.0, 100.0, 0.0]);
    let tilt = 0.409_092_8;
    let half_step_days = 0.01;
    let before = project_fixed_axes(&shifted(state, -half_step_days), tilt).unwrap();
    let after = project_fixed_axes(&shifted(state, half_step_days), tilt).unwrap();
    assert!(before.longitude_rad > TAU - 0.001);
    assert!(after.longitude_rad < 0.001);

    let projection = project_fixed_axes(&state, tilt).unwrap();
    let numeric = centered_rates(state, tilt, half_step_days);
    assert!(projection.longitude_rate_rad_per_day > 0.0);
    assert_near(projection.longitude_rate_rad_per_day, numeric[0], 2.0e-11);
    assert_near(projection.latitude_rate_rad_per_day, numeric[1], 2.0e-11);
    assert_near(projection.distance_rate_km_per_day, numeric[2], 5.0e-4);
}
