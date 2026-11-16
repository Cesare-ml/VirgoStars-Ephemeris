/*
EXHIBIT A. Common Public Attribution License Version 1.0.

The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

// Modification record: 2026-11-11 — deterministic fixed-axis singularity and wrap regressions.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

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

#[test]
fn exact_origin_and_poles_fail_closed_instead_of_inventing_longitude() {
    for position in [[0.0, 0.0, 0.0], [0.0, 0.0, 5.0], [0.0, 0.0, -5.0]] {
        let result = project_fixed_axes(&state(position, [1.0, 2.0, 3.0]), 0.0);
        assert_eq!(result, Err(ProjectionError::UndefinedDirection));
    }
}

#[test]
fn longitude_wrap_and_quadrants_follow_atan2_convention() {
    let cases = [
        ([1.0, 0.0, 0.0], 0.0),
        ([0.0, 1.0, 0.0], FRAC_PI_2),
        ([-1.0, 0.0, 0.0], PI),
        ([0.0, -1.0, 0.0], 3.0 * FRAC_PI_2),
        ([1.0, -1.0, 0.0], 7.0 * PI / 4.0),
    ];
    for (position, expected) in cases {
        let projected = project_fixed_axes(&state(position, [0.0; 3]), 0.0).unwrap();
        assert!(projected.longitude_rad >= 0.0 && projected.longitude_rad < TAU);
        assert!(
            (projected.longitude_rad - expected).abs() < 1e-14,
            "position={position:?}, longitude={}",
            projected.longitude_rad
        );
        assert_eq!(projected.latitude_rad, 0.0);
    }
}

#[test]
fn rejects_nonfinite_epoch_tilt_and_velocity() {
    let valid = state([1.0, 2.0, 3.0], [0.0; 3]);
    assert_eq!(project_fixed_axes(&valid, f64::NAN), Err(ProjectionError::NonFinite));
    let mut bad_epoch = valid;
    bad_epoch.epoch = JulianDateTdb(f64::INFINITY);
    assert_eq!(project_fixed_axes(&bad_epoch, 0.0), Err(ProjectionError::NonFinite));
    let mut bad_velocity = valid;
    bad_velocity.velocity_km_per_day[1] = f64::NEG_INFINITY;
    assert_eq!(project_fixed_axes(&bad_velocity, 0.0), Err(ProjectionError::NonFinite));
}
