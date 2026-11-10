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
use virgostars_ephemeris_core::coordinates::{project_fixed_axes, ProjectionError};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb, StateVector};

fn state(position_km: [f64; 3], velocity_km_per_day: [f64; 3]) -> StateVector {
    StateVector {
        target: Body::Moon,
        center: Body::Earth,
        epoch: JulianDateTdb(2451545.0),
        position_km,
        velocity_km_per_day,
    }
}
fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected}"
    );
}

#[test]
fn matches_erfa_pv2s_reference_in_fixed_icrf_axes() {
    // Numerical reference: ERFA t_erfa_c.c, t_pv2s; see third_party/ERFA_LICENSE.txt.
    // Units are interpreted here as km and km/day, respectively.
    let vector = state(
        [
            -0.4514964673880165,
            0.03093394277342585,
            0.05594668105108779,
        ],
        [
            1.292_270_850_663_26e-5,
            2.652814182060692e-6,
            2.568431853930293e-6,
        ],
    );
    let s = project_fixed_axes(&vector, 0.0).unwrap();
    near(s.longitude_rad, 3.073_185_307_179_586_7, 1e-12);
    near(s.latitude_rad, 0.123, 1e-12);
    near(s.distance_km, 0.455_999_999_999_999_96, 1e-12);
    near(s.longitude_rate_rad_per_day, -7.8e-6, 1e-16);
    near(s.latitude_rate_rad_per_day, 9.010_000_000_000_002e-6, 1e-16);
    near(
        s.distance_rate_km_per_day,
        -1.229_999_999_999_999_9e-5,
        1e-16,
    );
    assert_eq!(
        (s.target, s.center, s.epoch),
        (vector.target, vector.center, vector.epoch)
    );
}

#[test]
fn fixed_axis_rotation_and_retrograde_rate_have_declared_direction() {
    let s = project_fixed_axes(&state([1.0, 0.0, 1.0], [0.0, -2.0, 0.0]), FRAC_PI_2).unwrap();
    near(s.longitude_rad, PI / 4.0, 1e-15);
    near(s.latitude_rad, 0.0, 1e-15);
    near(s.latitude_rate_rad_per_day, 2.0 / 2.0_f64.sqrt(), 1e-15);
    let retrograde = project_fixed_axes(&state([1.0, -1.0, 0.0], [-1.0, -1.0, 0.0]), 0.0).unwrap();
    near(retrograde.longitude_rad, 7.0 * PI / 4.0, 1e-15);
    near(retrograde.longitude_rate_rad_per_day, -1.0, 1e-15);
}

#[test]
fn undefined_and_nonfinite_inputs_are_explicit_errors() {
    for position in [[0.0, 0.0, 0.0], [0.0, 0.0, 4.0], [0.0, 0.0, -4.0]] {
        assert_eq!(
            project_fixed_axes(&state(position, [0.0; 3]), 0.0),
            Err(ProjectionError::UndefinedDirection)
        );
    }
    assert_eq!(
        project_fixed_axes(&state([1.0, 0.0, 0.0], [f64::NAN, 0.0, 0.0]), 0.0),
        Err(ProjectionError::NonFinite)
    );
    assert_eq!(
        project_fixed_axes(&state([1.0, 0.0, 0.0], [0.0; 3]), f64::INFINITY),
        Err(ProjectionError::NonFinite)
    );
    let mut invalid_epoch = state([1.0, 0.0, 0.0], [0.0; 3]);
    invalid_epoch.epoch = JulianDateTdb(f64::NAN);
    assert_eq!(
        project_fixed_axes(&invalid_epoch, 0.0),
        Err(ProjectionError::NonFinite)
    );
    assert_eq!(
        project_fixed_axes(&state([1e-300, 0.0, 0.0], [0.0, 1e300, 0.0]), 0.0),
        Err(ProjectionError::NonFinite)
    );
}

#[test]
#[ignore = "requires DE441_PATH pointing to the external JPL binary"]
fn projects_external_de441_moon_as_geometric_fixed_axis_state() {
    let path = std::env::var_os("DE441_PATH").expect("set DE441_PATH to the external DE441 file");
    let mut eph = De441::open(path).unwrap();
    let state = eph
        .state(Body::Moon, Body::Earth, JulianDateTdb(2451545.0))
        .unwrap();
    // Explicit fixed illustrative tilt, not a date-dependent ecliptic model.
    let result = project_fixed_axes(&state, 23.4392911_f64.to_radians()).unwrap();
    assert!((0.0..TAU).contains(&result.longitude_rad));
    assert!(result.latitude_rad.abs() < 0.2);
    assert!((350_000.0..410_000.0).contains(&result.distance_km));
    assert!((0.1..0.4).contains(&result.longitude_rate_rad_per_day));
}
