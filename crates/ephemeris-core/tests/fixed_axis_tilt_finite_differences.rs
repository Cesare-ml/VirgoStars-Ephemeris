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
// tilted spherical rates, branch cuts, XYZ reconstruction, scaling, analytic circle, and epoch invariance.

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
        *position += *velocity * days;
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
    // Check both wrap directions: increasing longitude crosses 2pi -> 0,
    // while decreasing longitude crosses 0 -> 2pi. A raw subtraction would
    // spuriously report a large jump of opposite sign in both cases.
    let half_step_days = 0.01;
    for (initial_y, transverse_rate, increasing) in [(-0.1, 100.0, true), (0.1, -100.0, false)] {
        for tilt in [0.409_092_8, -0.27] {
            let state = synthetic_state([1_000_000.0, initial_y, 0.0], [0.0, transverse_rate, 0.0]);
            let before = project_fixed_axes(&shifted(state, -half_step_days), tilt).unwrap();
            let after = project_fixed_axes(&shifted(state, half_step_days), tilt).unwrap();
            if increasing {
                assert!(before.longitude_rad > TAU - 0.001);
                assert!(after.longitude_rad < 0.001);
            } else {
                assert!(before.longitude_rad < 0.001);
                assert!(after.longitude_rad > TAU - 0.001);
            }

            let projection = project_fixed_axes(&state, tilt).unwrap();
            let numeric = centered_rates(state, tilt, half_step_days);
            assert_eq!(
                projection.longitude_rate_rad_per_day.signum(),
                transverse_rate.signum(),
            );
            assert_near(projection.longitude_rate_rad_per_day, numeric[0], 2.0e-11);
            assert_near(projection.latitude_rate_rad_per_day, numeric[1], 2.0e-11);
            assert_near(projection.distance_rate_km_per_day, numeric[2], 5.0e-4);
        }
    }
}

#[test]
fn tilted_spherical_state_reconstructs_icrf_position_and_velocity() {
    // Invert the published spherical coordinate/rate equations, then invert
    // the fixed passive X rotation. This checks all six output fields against
    // independent input XYZ data, including a longitude branch cut and a
    // nearly polar direction. These values are synthetic geometric states.
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
        ([1_000_000.0, -0.1, 0.0], [0.0, 100.0, 0.0], 0.409_092_8),
        ([-500_000.0, 1.0, 210_000.0], [400.0, -12.0, 91.0], -0.27),
        ([1_000.0, 1_200.0, 10_000_000.0], [10.0, -11.0, 2.0], 0.0),
    ] {
        let state = synthetic_state(position, velocity);
        let spherical = project_fixed_axes(&state, tilt).unwrap();
        let (sin_lon, cos_lon) = spherical.longitude_rad.sin_cos();
        let (sin_lat, cos_lat) = spherical.latitude_rad.sin_cos();

        // Derivative of (r*cos(lat)*cos(lon), r*cos(lat)*sin(lon),
        //                r*sin(lat)) in the tilted fixed-axis frame.
        let horizontal = spherical.distance_km * cos_lat;
        let horizontal_rate = spherical.distance_rate_km_per_day * cos_lat
            - spherical.distance_km * sin_lat * spherical.latitude_rate_rad_per_day;
        let vertical_rate = spherical.distance_rate_km_per_day * sin_lat
            + spherical.distance_km * cos_lat * spherical.latitude_rate_rad_per_day;
        let x_prime = horizontal * cos_lon;
        let y_prime = horizontal * sin_lon;
        let z_prime = spherical.distance_km * sin_lat;
        let vx_prime =
            horizontal_rate * cos_lon - horizontal * sin_lon * spherical.longitude_rate_rad_per_day;
        let vy_prime =
            horizontal_rate * sin_lon + horizontal * cos_lon * spherical.longitude_rate_rad_per_day;

        // Inverse of the library's fixed passive +X rotation.
        let (sin_tilt, cos_tilt) = tilt.sin_cos();
        let reconstructed_position = [
            x_prime,
            cos_tilt * y_prime - sin_tilt * z_prime,
            sin_tilt * y_prime + cos_tilt * z_prime,
        ];
        let reconstructed_velocity = [
            vx_prime,
            cos_tilt * vy_prime - sin_tilt * vertical_rate,
            sin_tilt * vy_prime + cos_tilt * vertical_rate,
        ];
        let position_scale = position
            .iter()
            .fold(1.0_f64, |largest, component| largest.max(component.abs()));
        let velocity_scale = velocity
            .iter()
            .fold(1.0_f64, |largest, component| largest.max(component.abs()));
        for axis in 0..3 {
            assert_near(
                reconstructed_position[axis],
                position[axis],
                position_scale * 5.0e-13,
            );
            assert_near(
                reconstructed_velocity[axis],
                velocity[axis],
                velocity_scale * 2.0e-12,
            );
        }
    }
}

#[test]
fn position_and_velocity_scaling_respects_rate_units() {
    // Independent scaling is a metamorphic check on the units of the output.
    // Position scales longitude/latitude not at all, radius linearly and
    // angular rates inversely. Velocity scales both rate types linearly.
    // Negative and zero velocity multipliers test signed and stationary rates.
    let position = [137_000_000.0, -220_000_000.0, 39_000_000.0];
    let velocity = [4_321.0, -6_543.0, 1_278.0];
    for tilt in [0.0, 0.409_092_8, -0.27] {
        let baseline = project_fixed_axes(&synthetic_state(position, velocity), tilt).unwrap();
        for position_scale in [1.0e-6, 1.0, 1.0e6] {
            for velocity_scale in [-2.0, 0.0, 1.0, 3.0] {
                let scaled_position = position.map(|component| component * position_scale);
                let scaled_velocity = velocity.map(|component| component * velocity_scale);
                let result =
                    project_fixed_axes(&synthetic_state(scaled_position, scaled_velocity), tilt)
                        .unwrap();

                assert_near(result.longitude_rad, baseline.longitude_rad, 2.0e-12);
                assert_near(result.latitude_rad, baseline.latitude_rad, 2.0e-12);
                let expected_distance = baseline.distance_km * position_scale;
                assert_near(
                    result.distance_km,
                    expected_distance,
                    expected_distance * 2.0e-12,
                );

                let angular_multiplier = velocity_scale / position_scale;
                for (actual, original) in [
                    (
                        result.longitude_rate_rad_per_day,
                        baseline.longitude_rate_rad_per_day,
                    ),
                    (
                        result.latitude_rate_rad_per_day,
                        baseline.latitude_rate_rad_per_day,
                    ),
                ] {
                    let expected = original * angular_multiplier;
                    assert_near(actual, expected, expected.abs() * 2.0e-12 + 1.0e-12);
                }
                let expected_radial_rate = baseline.distance_rate_km_per_day * velocity_scale;
                assert_near(
                    result.distance_rate_km_per_day,
                    expected_radial_rate,
                    expected_radial_rate.abs() * 2.0e-12 + 1.0e-12,
                );
            }
        }
    }
}

#[test]
fn inclined_circular_motion_matches_closed_form_angles_and_rates() {
    // A uniform circle in the original ICRF XY plane is rotated into fixed
    // axes. Independently differentiated closed-form angular functions are:
    //
    //   lambda = atan2(cos(tilt)*sin(phase), cos(phase))
    //   beta   = asin(-sin(tilt)*sin(phase))
    //   lambda_dot = omega*cos(tilt) /
    //                (cos(phase)^2 + cos(tilt)^2*sin(phase)^2)
    //   beta_dot = -omega*sin(tilt)*cos(phase) /
    //              sqrt(1 - sin(tilt)^2*sin(phase)^2)
    //
    // These are fixed-axis *geometric* rates, not date-frame or apparent
    // derivatives. The source circle has constant radius and zero radial rate.
    const RADIUS_KM: f64 = 2_000_000.0;
    for tilt in [0.0_f64, 0.409_092_8, -0.61] {
        let (sin_tilt, cos_tilt) = tilt.sin_cos();
        for phase in [0.0_f64, 0.31, 1.25, 2.73, -2.9] {
            let (sin_phase, cos_phase) = phase.sin_cos();
            let position = [RADIUS_KM * cos_phase, RADIUS_KM * sin_phase, 0.0];
            let expected_longitude = (cos_tilt * sin_phase).atan2(cos_phase).rem_euclid(TAU);
            let expected_latitude = (-sin_tilt * sin_phase).asin();
            let planar_sq = cos_phase * cos_phase + cos_tilt * cos_tilt * sin_phase * sin_phase;
            let latitude_denominator = (1.0 - sin_tilt * sin_tilt * sin_phase * sin_phase).sqrt();
            for omega in [-0.035, 0.035] {
                let velocity = [
                    -RADIUS_KM * omega * sin_phase,
                    RADIUS_KM * omega * cos_phase,
                    0.0,
                ];
                let output =
                    project_fixed_axes(&synthetic_state(position, velocity), tilt).unwrap();
                let expected_longitude_rate = omega * cos_tilt / planar_sq;
                let expected_latitude_rate = -omega * sin_tilt * cos_phase / latitude_denominator;

                assert_near(output.longitude_rad, expected_longitude, 5.0e-15);
                assert_near(output.latitude_rad, expected_latitude, 5.0e-15);
                assert_near(output.distance_km, RADIUS_KM, 1.0e-8);
                assert_near(
                    output.longitude_rate_rad_per_day,
                    expected_longitude_rate,
                    5.0e-15,
                );
                assert_near(
                    output.latitude_rate_rad_per_day,
                    expected_latitude_rate,
                    5.0e-15,
                );
                assert_near(output.distance_rate_km_per_day, 0.0, 1.0e-8);
            }
        }
    }
}

#[test]
fn fixed_tilt_projection_is_epoch_and_body_metadata_invariant() {
    // This API projects an already specified geometric ICRF state. It must
    // neither advance the orbit nor silently substitute a date-dependent
    // ecliptic when only the epoch/target/center labels change.
    // The deliberately ancient/future epochs are NOT ephemeris coverage claims.
    let position = [43_000_000.0, -79_000_000.0, 21_000_000.0];
    let velocity = [3_500.0, 800.0, -900.0];
    for tilt in [-0.27, 0.0, 0.409_092_8] {
        let baseline = project_fixed_axes(&synthetic_state(position, velocity), tilt).unwrap();
        for (epoch, target, center) in [
            (
                JulianDateTdb(-2_451_545.5),
                Body::Sun,
                Body::SolarSystemBarycenter,
            ),
            (JulianDateTdb(0.0), Body::Earth, Body::Sun),
            (JulianDateTdb(2_451_545.0), Body::Moon, Body::Earth),
            (
                JulianDateTdb(8_000_000.25),
                Body::MarsBarycenter,
                Body::SolarSystemBarycenter,
            ),
        ] {
            let mut input = synthetic_state(position, velocity);
            input.epoch = epoch;
            input.target = target;
            input.center = center;
            let result = project_fixed_axes(&input, tilt).unwrap();

            // Exact equality is expected: neither body IDs nor TDB epoch
            // participates in the fixed-axis Cartesian arithmetic.
            for (actual, expected) in [
                (result.longitude_rad, baseline.longitude_rad),
                (result.latitude_rad, baseline.latitude_rad),
                (result.distance_km, baseline.distance_km),
                (
                    result.longitude_rate_rad_per_day,
                    baseline.longitude_rate_rad_per_day,
                ),
                (
                    result.latitude_rate_rad_per_day,
                    baseline.latitude_rate_rad_per_day,
                ),
                (result.distance_rate_km_per_day, baseline.distance_rate_km_per_day),
            ] {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
            assert_eq!(result.epoch, epoch);
            assert_eq!(result.target, target);
            assert_eq!(result.center, center);
            assert_eq!(result.tilt_rad, tilt);
        }
    }
}
