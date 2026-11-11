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

//! Tests for IERS TIRS->CIRS Earth-spin-only intermediate-frame rotation.
//!
//! Created 2026-11-12. Analytic orientation and finite-difference tests;
//! no complete terrestrial-to-GCRS or JPL/Horizons parity is asserted.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use virgostars_ephemeris_core::tirs_cirs_era::{
    cirs_to_tirs_position, tirs_to_cirs_earth_spin, TirsCirsError,
};

const IAU2000_ERA_RATE_RAD_PER_UT1_DAY: f64 = TAU * 1.002_737_811_911_354_6;

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

#[test]
fn zero_era_aligns_tirs_with_cirs_and_sets_positive_eastward_spin() {
    let r = 6_378.137;
    let state = tirs_to_cirs_earth_spin([r, 0.0, 0.0], 0.0, IAU2000_ERA_RATE_RAD_PER_UT1_DAY)
        .unwrap();
    near_vector(state.position_km, [r, 0.0, 0.0], 1.0e-12);
    near_vector(
        state.spin_velocity_km_per_ut1_day,
        [0.0, IAU2000_ERA_RATE_RAD_PER_UT1_DAY * r, 0.0],
        1.0e-10,
    );
}

#[test]
fn quarter_turn_has_correct_iau_era_sign() {
    // IERS Eq. 5.5: TIRS -> CIRS is passive R3(-ERA).
    // In ordinary column-vector active coordinates, +X rotates into +Y
    // when ERA = +pi/2. The spin rate points toward -X there.
    let state = tirs_to_cirs_earth_spin([1.0, 0.0, 3.0], FRAC_PI_2, 1.0).unwrap();
    near_vector(state.position_km, [0.0, 1.0, 3.0], 2.0e-15);
    near_vector(state.spin_velocity_km_per_ut1_day, [-1.0, 0.0, 0.0], 2.0e-15);
}

#[test]
fn spin_axis_does_not_acquire_an_artificial_velocity() {
    let state = tirs_to_cirs_earth_spin([0.0, 0.0, -6356.752], 1.3, 6.3).unwrap();
    near_vector(state.position_km, [0.0, 0.0, -6356.752], 1.0e-12);
    near_vector(state.spin_velocity_km_per_ut1_day, [0.0, 0.0, 0.0], 0.0);
}

#[test]
fn inverse_position_rotation_closes_without_polar_motion() {
    for position in [
        [6_378.137, 0.0, 0.0],
        [2_180.0, -3_711.0, 4_832.0],
        [-5_700.0, 1_400.0, -2_100.0],
    ] {
        for era in [0.0, 0.3, FRAC_PI_2, PI, TAU - 1.0e-6] {
            let state = tirs_to_cirs_earth_spin(
                position,
                era,
                IAU2000_ERA_RATE_RAD_PER_UT1_DAY,
            )
            .unwrap();
            let recovered = cirs_to_tirs_position(state.position_km, era).unwrap();
            near_vector(recovered, position, 3.0e-12);
        }
    }
}

#[test]
fn rotation_preserves_length_and_spin_is_tangent_to_circle() {
    let pos = [3_300.0, -2_500.0, 3_900.0];
    let original_length = pos.into_iter().map(|v| v * v).sum::<f64>().sqrt();
    let state = tirs_to_cirs_earth_spin(pos, 2.1, IAU2000_ERA_RATE_RAD_PER_UT1_DAY)
        .unwrap();
    let final_length = state
        .position_km
        .into_iter()
        .map(|v| v * v)
        .sum::<f64>()
        .sqrt();
    near(final_length, original_length, 2.0e-12);
    let dot = state
        .position_km
        .into_iter()
        .zip(state.spin_velocity_km_per_ut1_day)
        .map(|(x, v)| x * v)
        .sum::<f64>();
    near(dot, 0.0, 2.0e-8);
}

#[test]
fn analytical_ut1_spin_rate_matches_symmetric_era_difference() {
    let pos = [3_300.0, -2_500.0, 1_900.0];
    let era = 2.1;
    let rate = IAU2000_ERA_RATE_RAD_PER_UT1_DAY;
    let delta_days = 1.0e-5;
    let state = tirs_to_cirs_earth_spin(pos, era, rate).unwrap();
    let before = tirs_to_cirs_earth_spin(pos, era - rate * delta_days, rate)
        .unwrap()
        .position_km;
    let after = tirs_to_cirs_earth_spin(pos, era + rate * delta_days, rate)
        .unwrap()
        .position_km;
    let numeric = std::array::from_fn(|i| (after[i] - before[i]) / (2.0 * delta_days));
    near_vector(numeric, state.spin_velocity_km_per_ut1_day, 6.0e-5);
}

#[test]
fn published_iau_rate_and_wgs84_equatorial_radius_give_expected_spin_speed() {
    // Two independent published physical constants, not a geocentric
    // ephemeris accuracy test. At WGS84 equator the spin speed is about
    // 465.101094 meters per SI day-second, for the ideal TIRS-fixed site.
    let equatorial_radius_km = 6_378.137;
    let state = tirs_to_cirs_earth_spin(
        [equatorial_radius_km, 0.0, 0.0],
        0.0,
        IAU2000_ERA_RATE_RAD_PER_UT1_DAY,
    )
    .unwrap();
    let equatorial_speed_m_per_second =
        state.spin_velocity_km_per_ut1_day[1] * 1_000.0 / 86_400.0;
    near(equatorial_speed_m_per_second, 465.101_094_254_722_2, 1.0e-6);
}

#[test]
fn invalid_angles_rates_and_positions_fail_without_silent_fallback() {
    let valid_pos = [1.0, 2.0, 3.0];
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01, TAU] {
        assert_eq!(
            tirs_to_cirs_earth_spin(valid_pos, invalid, 6.3),
            Err(TirsCirsError::InvalidEarthRotationAngle)
        );
        assert_eq!(
            cirs_to_tirs_position(valid_pos, invalid),
            Err(TirsCirsError::InvalidEarthRotationAngle)
        );
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 0.0] {
        assert_eq!(
            tirs_to_cirs_earth_spin(valid_pos, 1.0, invalid),
            Err(TirsCirsError::InvalidEarthRotationRate)
        );
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let pos = [invalid, 1.0, 2.0];
        assert_eq!(
            tirs_to_cirs_earth_spin(pos, 1.0, 6.3),
            Err(TirsCirsError::NonFinitePosition)
        );
        assert_eq!(
            cirs_to_tirs_position(pos, 1.0),
            Err(TirsCirsError::NonFinitePosition)
        );
    }
    assert_eq!(
        tirs_to_cirs_earth_spin([1.0e308, 1.0e308, 0.0], 1.0, 1.0e308),
        Err(TirsCirsError::NonFiniteResult)
    );
}
