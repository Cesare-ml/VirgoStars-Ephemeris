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

//! Independent arithmetic tests of the shared rad/km → deg/AU six-field
//! display-unit adapter. These are NOT Swiss `swe_calc_ut` or true
//! ecliptic-of-date parity fixtures; no external data are bundled.

use std::f64::consts::{PI, TAU};

use virgostars_ephemeris_core::aberrated_spherical::{
    aberrated_spherical_to_degrees_au, spherical_from_aberrated_state,
};
use virgostars_ephemeris_core::aberration_state::{
    aberrate_light_time_state, ObserverAccelerationSource,
};
use virgostars_ephemeris_core::coordinates::SphericalState;
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb};
use virgostars_ephemeris_core::light_time::GeocentricLightTimeState;
use virgostars_ephemeris_core::six_field_units::{
    fixed_spherical_to_degrees_au, UnitConversionError, IAU_AU_KM,
};

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e} expected={expected:.16e} tolerance={tolerance:.2e}"
    );
}

fn sample() -> SphericalState {
    SphericalState {
        target: Body::Mercury,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        tilt_rad: 0.15,
        longitude_rad: PI / 2.0,
        latitude_rad: -PI / 6.0,
        distance_km: IAU_AU_KM * 2.0,
        longitude_rate_rad_per_day: -PI / 180.0,
        latitude_rate_rad_per_day: PI / 360.0,
        distance_rate_km_per_day: -IAU_AU_KM * 0.02,
    }
}

#[test]
fn independent_exact_units_for_all_six_fields_and_provenance() {
    let source = sample();
    let fields = fixed_spherical_to_degrees_au(&source).unwrap();
    near(fields.longitude_deg, 90.0, 1e-13);
    near(fields.latitude_deg, -30.0, 1e-13);
    near(fields.distance_au, 2.0, 1e-15);
    near(fields.longitude_speed_deg_per_tdb_day, -1.0, 1e-15);
    near(fields.latitude_speed_deg_per_tdb_day, 0.5, 1e-15);
    near(fields.distance_speed_au_per_tdb_day, -0.02, 1e-16);
    assert_eq!(fields.target, source.target);
    assert_eq!(fields.center, Body::Earth);
    assert_eq!(fields.epoch, source.epoch);
    assert_eq!(fields.fixed_tilt_rad, source.tilt_rad);
}

#[test]
fn zero_and_just_below_full_turn_remain_distinct_without_double_wrapping() {
    let mut source = sample();
    source.longitude_rad = 0.0;
    let first = fixed_spherical_to_degrees_au(&source).unwrap();
    assert_eq!(first.longitude_deg, 0.0);
    source.longitude_rad = TAU - 1e-9;
    let last = fixed_spherical_to_degrees_au(&source).unwrap();
    assert!(last.longitude_deg > 359.999_999_9);
    assert!(last.longitude_deg < 360.0);
}

#[test]
fn out_of_range_longitude_fails_instead_of_silently_normalizing() {
    let mut source = sample();
    for bad in [TAU, -1e-6, -TAU] {
        source.longitude_rad = bad;
        assert_eq!(
            fixed_spherical_to_degrees_au(&source),
            Err(UnitConversionError::InvalidLongitude)
        );
    }
}

#[test]
fn invalid_latitude_or_undefined_distance_fails_closed() {
    let mut source = sample();
    for bad in [-PI / 2.0 - 1e-6, PI / 2.0 + 1e-6] {
        source.latitude_rad = bad;
        assert_eq!(
            fixed_spherical_to_degrees_au(&source),
            Err(UnitConversionError::InvalidLatitude)
        );
    }
    source = sample();
    for bad in [0.0, -10.0] {
        source.distance_km = bad;
        assert_eq!(
            fixed_spherical_to_degrees_au(&source),
            Err(UnitConversionError::NonPositiveDistance)
        );
    }
}

#[test]
fn nonfinite_and_unrepresentable_inputs_are_rejected() {
    let mut source = sample();
    source.longitude_rate_rad_per_day = f64::NAN;
    assert_eq!(
        fixed_spherical_to_degrees_au(&source),
        Err(UnitConversionError::NonFinite)
    );
    source = sample();
    source.epoch = JulianDateTdb(f64::INFINITY);
    assert_eq!(
        fixed_spherical_to_degrees_au(&source),
        Err(UnitConversionError::NonFinite)
    );
    source = sample();
    source.latitude_rate_rad_per_day = f64::MAX;
    assert_eq!(
        fixed_spherical_to_degrees_au(&source),
        Err(UnitConversionError::Overflow)
    );
    source = sample();
    source.distance_km = f64::from_bits(1);
    assert_eq!(
        fixed_spherical_to_degrees_au(&source),
        Err(UnitConversionError::Overflow)
    );
}

#[test]
fn full_signed_retrograde_and_radial_motion_preserve_units() {
    let mut source = sample();
    source.longitude_rad = 3.0 * PI / 2.0;
    source.latitude_rad = PI / 2.0;
    source.longitude_rate_rad_per_day = -0.2;
    source.latitude_rate_rad_per_day = -0.001;
    source.distance_rate_km_per_day = IAU_AU_KM * 3.0;
    let fields = fixed_spherical_to_degrees_au(&source).unwrap();
    near(fields.longitude_deg, 270.0, 1e-13);
    near(fields.latitude_deg, 90.0, 1e-13);
    near(fields.longitude_speed_deg_per_tdb_day, -0.2 * 180.0 / PI, 1e-12);
    near(fields.latitude_speed_deg_per_tdb_day, -0.001 * 180.0 / PI, 1e-12);
    near(fields.distance_speed_au_per_tdb_day, 3.0, 1e-15);
}

#[test]
fn one_conversion_after_aberration_preserves_all_metadata_and_body_id() {
    let source = GeocentricLightTimeState {
        target: Body::JupiterBarycenter,
        observation_epoch: JulianDateTdb(2_451_545.0),
        emission_epoch: JulianDateTdb(2_451_544.99),
        light_time_seconds: 864.0,
        light_time_rate: 0.01,
        position_km: [IAU_AU_KM, 0.0, 0.0],
        velocity_km_per_day: [
            0.0,
            (PI / 180.0) * IAU_AU_KM,
            0.01 * IAU_AU_KM,
        ],
    };
    let cartesian =
        aberrate_light_time_state(&source, [0.0; 3], [0.0; 3]).unwrap();
    let angular = spherical_from_aberrated_state(&cartesian, 0.0).unwrap();
    let result = aberrated_spherical_to_degrees_au(&angular).unwrap();
    near(result.fields.longitude_deg, 0.0, 1e-12);
    near(result.fields.longitude_speed_deg_per_tdb_day, 1.0, 1e-12);
    near(result.fields.latitude_speed_deg_per_tdb_day, 0.01 * 180.0 / PI, 1e-12);
    near(result.fields.distance_speed_au_per_tdb_day, 0.0, 1e-14);
    near(result.fields.distance_au, 1.0, 1e-15);
    assert_eq!(result.fields.target, Body::JupiterBarycenter);
    assert_eq!(result.fields.center, Body::Earth);
    assert_eq!(result.fields.epoch, source.observation_epoch);
    assert_eq!(result.emission_epoch, source.emission_epoch);
    assert_eq!(result.light_time_seconds, source.light_time_seconds);
    assert_eq!(result.light_time_rate, source.light_time_rate);
    assert_eq!(
        result.observer_acceleration_source,
        ObserverAccelerationSource::CallerProvided
    );
}
