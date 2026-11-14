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

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use virgostars_ephemeris_core::coordinates::{
    project_fixed_axes, project_spk_fixed_axes, ProjectionError,
};
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::spk_relative_state::SpkRelativeState;

fn state(position_km: [f64; 3], velocity_km_per_second: [f64; 3]) -> SpkRelativeState {
    SpkRelativeState {
        target_id: 301,
        center_id: 399,
        common_ancestor_id: 3,
        tdb_seconds_past_j2000: 12345.125,
        position_km,
        velocity_km_per_second,
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:?}, expected={expected:?}, tolerance={tolerance:?}"
    );
}

// Modification (2026-11-14): verify identical spatial projection for
// classic DE441 km/day and SPK km/s input without confusing rate units.
#[test]
fn spk_and_de441_adapters_share_angles_and_scale_rates_by_seconds_per_day() {
    let position = [160_000.0, -240_000.0, 45_000.0];
    let velocity_s = [-0.4, 0.25, 0.05];
    let spk = state(position, velocity_s);
    let de441 = StateVector {
        target: Body::Moon,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        position_km: position,
        velocity_km_per_day: velocity_s.map(|component| component * 86_400.0),
    };
    for tilt in [-FRAC_PI_2, -0.409, 0.0, 0.409, FRAC_PI_2] {
        let spk_sphere = project_spk_fixed_axes(&spk, tilt).unwrap();
        let classic = project_fixed_axes(&de441, tilt).unwrap();
        assert_eq!(spk_sphere.target_id, 301);
        assert_eq!(spk_sphere.center_id, 399);
        assert_eq!(spk_sphere.common_ancestor_id, 3);
        assert_eq!(spk_sphere.tdb_seconds_past_j2000, 12345.125);
        near(spk_sphere.longitude_rad, classic.longitude_rad, 1e-14);
        near(spk_sphere.latitude_rad, classic.latitude_rad, 1e-14);
        near(spk_sphere.distance_km, classic.distance_km, 1e-9);
        near(
            spk_sphere.longitude_rate_rad_per_second * 86_400.0,
            classic.longitude_rate_rad_per_day,
            1e-14,
        );
        near(
            spk_sphere.latitude_rate_rad_per_second * 86_400.0,
            classic.latitude_rate_rad_per_day,
            1e-14,
        );
        near(
            spk_sphere.distance_rate_km_per_second * 86_400.0,
            classic.distance_rate_km_per_day,
            1e-10,
        );
    }
}

#[test]
fn circular_motion_and_negative_longitude_rate_have_analytic_oracles() {
    let radius = 384_400.0;
    let rate = 1.0e-5;
    let prograde =
        project_spk_fixed_axes(&state([radius, 0.0, 0.0], [0.0, radius * rate, 0.0]), 0.0).unwrap();
    near(prograde.longitude_rad, 0.0, 1e-15);
    near(prograde.latitude_rad, 0.0, 1e-15);
    near(prograde.longitude_rate_rad_per_second, rate, 1e-15);
    near(prograde.distance_km, radius, 1e-9);
    near(prograde.distance_rate_km_per_second, 0.0, 1e-12);
    let retrograde =
        project_spk_fixed_axes(&state([1.0, -1.0, 0.0], [-1.0, -1.0, 0.0]), 0.0).unwrap();
    near(retrograde.longitude_rad, 7.0 * PI / 4.0, 1e-15);
    near(retrograde.longitude_rate_rad_per_second, -1.0, 1e-15);
    assert!((0.0..TAU).contains(&prograde.longitude_rad));
}

#[test]
fn centered_difference_agrees_with_per_second_analytic_rates_in_tilted_axes() {
    let original = [1_000.0, 2_000.0, 300.0];
    let velocity = [-2.0, 1.5, 0.4];
    let tilt = 0.409;
    let dt = 0.05; // TDB seconds
    let at = |seconds: f64| {
        let translated = std::array::from_fn(|axis| original[axis] + velocity[axis] * seconds);
        project_spk_fixed_axes(&state(translated, velocity), tilt).unwrap()
    };
    let middle = at(0.0);
    let before = at(-dt);
    let after = at(dt);
    let dlongitude = (after.longitude_rad - before.longitude_rad + PI).rem_euclid(TAU) - PI;
    near(
        middle.longitude_rate_rad_per_second,
        dlongitude / (2.0 * dt),
        1e-10,
    );
    near(
        middle.latitude_rate_rad_per_second,
        (after.latitude_rad - before.latitude_rad) / (2.0 * dt),
        1e-10,
    );
    near(
        middle.distance_rate_km_per_second,
        (after.distance_km - before.distance_km) / (2.0 * dt),
        1e-8,
    );
}

#[test]
fn missing_geometric_direction_or_nonfinite_inputs_fail_closed() {
    for position in [[0.0, 0.0, 0.0], [0.0, 0.0, 10.0], [0.0, 0.0, -10.0]] {
        assert_eq!(
            project_spk_fixed_axes(&state(position, [0.0; 3]), 0.0),
            Err(ProjectionError::UndefinedDirection)
        );
    }
    assert_eq!(
        project_spk_fixed_axes(&state([1.0, 0.0, 0.0], [0.0; 3]), f64::NAN),
        Err(ProjectionError::NonFinite)
    );
    assert_eq!(
        project_spk_fixed_axes(&state([f64::INFINITY, 1.0, 0.0], [0.0; 3]), 0.0),
        Err(ProjectionError::NonFinite)
    );
    assert_eq!(
        project_spk_fixed_axes(&state([1.0, 0.0, 0.0], [0.0, f64::NAN, 0.0]), 0.0),
        Err(ProjectionError::NonFinite)
    );
    let mut wrong_epoch = state([1.0, 0.0, 0.0], [0.0; 3]);
    wrong_epoch.tdb_seconds_past_j2000 = f64::NEG_INFINITY;
    assert_eq!(
        project_spk_fixed_axes(&wrong_epoch, 0.0),
        Err(ProjectionError::NonFinite)
    );
    assert_eq!(
        project_spk_fixed_axes(&state([1e-300, 0.0, 0.0], [0.0, 1e300, 0.0]), 0.0),
        Err(ProjectionError::NonFinite)
    );
}
