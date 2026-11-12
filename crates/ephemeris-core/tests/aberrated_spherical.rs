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

//! Deterministic, synthetic checks for observer-aberrated **fixed-axis**
//! spherical positions and all three TDB-day velocity components.
//! No real JPL vectors, Swiss program code or third-party data included.

use std::f64::consts::{FRAC_PI_2, PI};

use virgostars_ephemeris_core::aberrated_spherical::{
    spherical_from_aberrated_state, AberratedSphericalError,
};
use virgostars_ephemeris_core::aberration::{annual_aberration_icrf, SPEED_OF_LIGHT_KM_PER_DAY};
use virgostars_ephemeris_core::aberration_state::{
    aberrate_light_time_state, AberratedGeocentricState, ObserverAccelerationSource,
};
use virgostars_ephemeris_core::coordinates::ProjectionError;
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb};
use virgostars_ephemeris_core::light_time::GeocentricLightTimeState;

fn near(actual: f64, expected: f64, tol: f64) {
    assert!(
        (actual - expected).abs() <= tol,
        "actual={actual:.16e}, expected={expected:.16e}, tol={tol:.2e}"
    );
}

fn source(r: [f64; 3], v: [f64; 3]) -> GeocentricLightTimeState {
    GeocentricLightTimeState {
        target: Body::Mercury,
        observation_epoch: JulianDateTdb(2_451_545.0),
        emission_epoch: JulianDateTdb(2_451_544.995),
        light_time_seconds: 432.0,
        light_time_rate: -0.001,
        position_km: r,
        velocity_km_per_day: v,
    }
}

fn aberrated(r: [f64; 3], v: [f64; 3]) -> AberratedGeocentricState {
    aberrate_light_time_state(&source(r, v), [0.0; 3], [0.0; 3]).unwrap()
}

#[test]
fn zero_earth_boost_preserves_all_six_spherical_fields_and_metadata() {
    let initial = source([3.0e7, 4.0e7, 0.0], [-40_000.0, 30_000.0, 5_000.0]);
    let cart = aberrate_light_time_state(&initial, [0.0; 3], [0.0; 3]).unwrap();
    let result = spherical_from_aberrated_state(&cart, 0.0).unwrap();
    let six = result.spherical;
    near(six.longitude_rad, (4.0_f64).atan2(3.0), 1e-15);
    near(six.latitude_rad, 0.0, 1e-15);
    near(six.distance_km, 5.0e7, 1e-8);
    // v chosen purely tangential on XY (+ a positive Z component).
    near(six.longitude_rate_rad_per_day, 0.001, 1e-15);
    near(six.latitude_rate_rad_per_day, 0.0001, 1e-15);
    near(six.distance_rate_km_per_day, 0.0, 1e-9);
    assert_eq!(six.target, Body::Mercury);
    assert_eq!(six.center, Body::Earth);
    assert_eq!(six.epoch, initial.observation_epoch);
    assert_eq!(result.emission_epoch, initial.emission_epoch);
    assert_eq!(result.light_time_seconds, initial.light_time_seconds);
    assert_eq!(result.light_time_rate, initial.light_time_rate);
    assert_eq!(
        result.observer_acceleration_source,
        ObserverAccelerationSource::CallerProvided
    );
}

#[test]
fn transverse_lorentz_boost_and_acceleration_appear_in_spherical_speed() {
    // Source along +Y and observer motion along +X:
    // n_ab = (beta, sqrt(1-beta²), 0). Therefore
    // longitude = acos(beta), dlongitude/dt = -beta_dot/sqrt(1-beta²).
    // This is independently hand-computable, unlike a pure module roundtrip.
    let beta = 0.1_f64;
    let beta_dot = 0.000_2_f64;
    let initial = source([0.0, 7.0e7, 0.0], [0.0; 3]);
    let cart = aberrate_light_time_state(
        &initial,
        [beta * SPEED_OF_LIGHT_KM_PER_DAY, 0.0, 0.0],
        [beta_dot * SPEED_OF_LIGHT_KM_PER_DAY, 0.0, 0.0],
    )
    .unwrap();
    let result = spherical_from_aberrated_state(&cart, 0.0).unwrap();
    let six = result.spherical;
    near(six.longitude_rad, beta.acos(), 1e-14);
    near(
        six.longitude_rate_rad_per_day,
        -beta_dot / (1.0 - beta * beta).sqrt(),
        1e-13,
    );
    near(six.latitude_rad, 0.0, 1e-15);
    near(six.latitude_rate_rad_per_day, 0.0, 1e-15);
    near(six.distance_km, 7.0e7, 1e-8);
    near(six.distance_rate_km_per_day, 0.0, 1e-8);
}

#[test]
fn caller_supplied_fixed_tilt_rotates_both_position_and_speed() {
    let cart = aberrated([10.0, 0.0, 5.0], [0.0, 1.0, 0.0]);
    let tilted = spherical_from_aberrated_state(&cart, FRAC_PI_2).unwrap();
    let untitled = spherical_from_aberrated_state(&cart, 0.0).unwrap();
    near(tilted.spherical.longitude_rad, (0.5_f64).atan(), 1e-15);
    near(tilted.spherical.latitude_rad, 0.0, 1e-15);
    near(
        tilted.spherical.latitude_rate_rad_per_day,
        -1.0 / (125.0_f64).sqrt(),
        1e-15,
    );
    near(
        tilted.spherical.distance_km,
        untitled.spherical.distance_km,
        1e-14,
    );
    near(
        tilted.spherical.distance_rate_km_per_day,
        untitled.spherical.distance_rate_km_per_day,
        1e-14,
    );
    near(tilted.spherical.tilt_rad, FRAC_PI_2, 0.0);
}

#[test]
fn negative_retrograde_rate_and_longitude_wrap_survive_composition() {
    let cart = aberrated([0.0, -100.0, 0.0], [-20.0, 0.0, 0.0]);
    let result = spherical_from_aberrated_state(&cart, 0.0).unwrap();
    near(result.spherical.longitude_rad, 1.5 * PI, 1e-15);
    near(result.spherical.longitude_rate_rad_per_day, -0.2, 1e-15);
    near(result.spherical.latitude_rate_rad_per_day, 0.0, 1e-15);
}

#[test]
fn invalid_epoch_lighttime_and_target_metadata_are_rejected() {
    let baseline = aberrated([1.0e7, 2.0e7, 0.0], [0.0; 3]);
    let mut bad = baseline;
    bad.target = Body::Earth;
    assert!(matches!(
        spherical_from_aberrated_state(&bad, 0.0),
        Err(AberratedSphericalError::InvalidIntermediate)
    ));
    bad = baseline;
    bad.emission_epoch = JulianDateTdb(bad.observation_epoch.0 + 1.0);
    assert!(matches!(
        spherical_from_aberrated_state(&bad, 0.0),
        Err(AberratedSphericalError::InvalidIntermediate)
    ));
    bad = baseline;
    bad.light_time_seconds = 0.0;
    assert!(matches!(
        spherical_from_aberrated_state(&bad, 0.0),
        Err(AberratedSphericalError::InvalidIntermediate)
    ));
    bad = baseline;
    bad.light_time_rate = f64::INFINITY;
    assert!(matches!(
        spherical_from_aberrated_state(&bad, 0.0),
        Err(AberratedSphericalError::InvalidIntermediate)
    ));
}

#[test]
fn nonfinite_tilt_or_undefined_poles_fail_instead_of_inventing_longitude() {
    let cart = aberrated([100.0, 0.0, 0.0], [0.0; 3]);
    assert!(matches!(
        spherical_from_aberrated_state(&cart, f64::NAN),
        Err(AberratedSphericalError::Projection(
            ProjectionError::NonFinite
        ))
    ));
    let pole = aberrated([0.0, 0.0, 100.0], [1.0, 0.0, 0.0]);
    assert!(matches!(
        spherical_from_aberrated_state(&pole, 0.0),
        Err(AberratedSphericalError::Projection(
            ProjectionError::UndefinedDirection
        ))
    ));
}

#[test]
fn independent_finite_difference_checks_aberrated_longitude_rate() {
    let r = 1.2e8_f64;
    let angular_speed = -0.015_f64;
    let phase = 1.0_f64;
    let beta = [8.0e5, -1.5e6, 3.0e5];
    let acceleration = [1.0e4, -4.0e4, 2.0e4];
    let step = 0.000_1_f64;
    let at = |dt: f64| {
        let longitude = phase + angular_speed * dt;
        let pos = [r * longitude.cos(), r * longitude.sin(), 0.1 * r];
        let observer = std::array::from_fn(|i| beta[i] + dt * acceleration[i]);
        annual_aberration_icrf(pos, observer).unwrap()
    };
    let a = at(-step);
    let b = at(step);
    let longitude = |v: [f64; 3]| v[1].atan2(v[0]);
    let numerical = (longitude(b) - longitude(a)) / (2.0 * step);
    let source_pos = [r * phase.cos(), r * phase.sin(), 0.1 * r];
    let source_rate = [
        -r * angular_speed * phase.sin(),
        r * angular_speed * phase.cos(),
        0.0,
    ];
    let cart =
        aberrate_light_time_state(&source(source_pos, source_rate), beta, acceleration).unwrap();
    let result = spherical_from_aberrated_state(&cart, 0.0).unwrap();
    near(
        result.spherical.longitude_rate_rad_per_day,
        numerical,
        5e-10,
    );
}
