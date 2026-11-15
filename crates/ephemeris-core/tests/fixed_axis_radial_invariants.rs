/*
EXHIBIT A. Common Public Attribution License Version 1.0.
The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
The Original Developer is the Initial Developer.
The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
Contributors retain applicable rights in their own contributions under the terms of this License.
SPDX-License-Identifier: CPAL-1.0
*/

// Modification record: 2026-11-11 — synthetic fixed-axis radial-motion regressions.
use virgostars_ephemeris_core::coordinates::project_fixed_axes;
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};

fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-11,
        "expected {expected}, got {actual}"
    );
}

fn radial_state() -> StateVector {
    // Norm = 13 km; velocity = (2/13) * position km/day.
    StateVector {
        target: Body::Moon,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        position_km: [3.0, 4.0, 12.0],
        velocity_km_per_day: [6.0 / 13.0, 8.0 / 13.0, 24.0 / 13.0],
    }
}

#[test]
fn radial_motion_has_zero_angular_rates_and_expected_distance_rate() {
    let state = radial_state();
    let projected = project_fixed_axes(&state, 0.0).unwrap();
    assert_eq!(projected.target, state.target);
    assert_eq!(projected.center, state.center);
    assert_eq!(projected.epoch, state.epoch);
    near(projected.distance_km, 13.0);
    near(projected.longitude_rad, 4.0_f64.atan2(3.0));
    near(projected.latitude_rad, 12.0_f64.atan2(5.0));
    near(projected.distance_rate_km_per_day, 2.0);
    near(projected.longitude_rate_rad_per_day, 0.0);
    near(projected.latitude_rate_rad_per_day, 0.0);
}

#[test]
fn fixed_axis_rotation_preserves_radial_speed_and_zero_angular_rates() {
    let state = radial_state();
    for tilt in [-1.0, -0.4, 0.0, 0.4, 1.0] {
        let projected = project_fixed_axes(&state, tilt).unwrap();
        near(projected.distance_km, 13.0);
        near(projected.distance_rate_km_per_day, 2.0);
        near(projected.longitude_rate_rad_per_day, 0.0);
        near(projected.latitude_rate_rad_per_day, 0.0);
    }
}
