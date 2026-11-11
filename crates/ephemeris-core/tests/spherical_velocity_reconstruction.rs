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
// Modification record: 2026-11-11 — add independent spherical velocity reconstruction tests.

use virgostars_ephemeris_core::coordinates::project_fixed_axes;
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};

fn assert_near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual}, expected={expected}, tolerance={tolerance}"
    );
}

#[test]
fn spherical_rates_reconstruct_cartesian_velocity_in_original_axes() {
    // Geometric ICRF axes; no precession, nutation or apparent corrections.
    let position = [3.0, 4.0, 12.0];
    let velocity = [-2.0, 5.0, 7.0];
    let state = StateVector {
        target: Body::Moon,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        position_km: position,
        velocity_km_per_day: velocity,
    };
    let projected = project_fixed_axes(&state, 0.0).unwrap();
    let (sin_lon, cos_lon) = projected.longitude_rad.sin_cos();
    let (sin_lat, cos_lat) = projected.latitude_rad.sin_cos();
    let r = projected.distance_km;
    let dr = projected.distance_rate_km_per_day;
    let dlon = projected.longitude_rate_rad_per_day;
    let dlat = projected.latitude_rate_rad_per_day;

    // Differentiate x=r cos(lat) cos(lon), y=r cos(lat) sin(lon),
    // z=r sin(lat) independently of the projection implementation.
    let reconstructed = [
        dr * cos_lat * cos_lon - r * dlat * sin_lat * cos_lon
            - r * dlon * cos_lat * sin_lon,
        dr * cos_lat * sin_lon - r * dlat * sin_lat * sin_lon
            + r * dlon * cos_lat * cos_lon,
        dr * sin_lat + r * dlat * cos_lat,
    ];
    for (actual, expected) in reconstructed.into_iter().zip(velocity) {
        assert_near(actual, expected, 1e-12);
    }
}

#[test]
fn spherical_longitude_rate_matches_angular_momentum_about_z() {
    let state = StateVector {
        target: Body::MarsBarycenter,
        center: Body::SolarSystemBarycenter,
        epoch: JulianDateTdb(2_451_545.0),
        position_km: [-8.0, 6.0, 5.0],
        velocity_km_per_day: [-3.0, -4.0, 2.0],
    };
    let projected = project_fixed_axes(&state, 0.0).unwrap();
    let [x, y, _] = state.position_km;
    let [vx, vy, _] = state.velocity_km_per_day;
    let angular_momentum_z = x * vy - y * vx;
    let expected = angular_momentum_z / (x * x + y * y);
    assert_near(projected.longitude_rate_rad_per_day, expected, 1e-14);
    assert_near(projected.longitude_rate_rad_per_day, 0.5, 1e-14);
}
