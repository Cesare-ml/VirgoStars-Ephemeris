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

//! Independent synthetic regression for one-call DE441 -> light-time ->
//! six geometric fixed-axis fields and all three reception-time speeds.
//! Not scientific JPL/Horizons or Swiss Ephemeris accuracy evidence.

use std::f64::consts::PI;
use std::io::Cursor;

use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};
use virgostars_ephemeris_core::geocentric_six_fields::{
    geocentric_fixed_six_fields, GeocentricFieldsError, GeocentricFixedAxes, ASTRONOMICAL_UNIT_KM,
};
use virgostars_ephemeris_core::light_time::{LightTimeError, SPEED_OF_LIGHT_KM_PER_SECOND};

const JD_MID: f64 = 2_451_545.0;
const JD_START: f64 = JD_MID - 16.0;
const JD_END: f64 = JD_MID + 16.0;
const RECORD_SIZE: usize = 3_000;
const DISTANCE_KM: f64 = 150_000_000.0;

fn put_f64(bytes: &mut [u8], at: usize, value: f64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], at: usize, value: i32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

// One fictional 32-TDB-day classic DE441 record. Mercury has a constant
// X position and linear XYZ rates; Earth/Moon are stationary at the origin.
// A dummy unused pointer determines the 3000-byte record length.
// For Chebyshev T1, x=(JD-JD_MID)/16, so c1 = 16 * km/day.
fn synthetic_kernel(mercury_velocity_km_per_day: [f64; 3]) -> Vec<u8> {
    let mut bytes = vec![0_u8; RECORD_SIZE * 3];
    let name = b"JPL Planetary Ephemeris DE441";
    bytes[..name.len()].copy_from_slice(name);
    put_f64(&mut bytes, 2652, JD_START);
    put_f64(&mut bytes, 2660, JD_END);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);
    for (index, first, coefficients) in [
        (0usize, 3usize, 2),
        (2, 9, 1),
        (3, 15, 1),
        (9, 12, 1),
        (12, 373, 1),
    ] {
        let at = if index == 12 { 2844 } else { 2696 + index * 12 };
        put_i32(&mut bytes, at, first as i32);
        put_i32(&mut bytes, at + 4, coefficients);
        put_i32(&mut bytes, at + 8, 1);
    }
    let record = RECORD_SIZE * 2;
    put_f64(&mut bytes, record, JD_START);
    put_f64(&mut bytes, record + 8, JD_END);
    for (axis, speed) in mercury_velocity_km_per_day.into_iter().enumerate() {
        let at = record + (2 + axis * 2) * 8;
        put_f64(&mut bytes, at, if axis == 0 { DISTANCE_KM } else { 0.0 });
        put_f64(&mut bytes, at + 8, speed * 16.0);
    }
    // A separate Mars-system-barycenter constant ensures type provenance.
    put_f64(&mut bytes, record + 14 * 8, 180_000_000.0);
    bytes
}

fn six_at(eph: &mut De441<Cursor<Vec<u8>>>, jd: f64, target: Body) ->
    Result<virgostars_ephemeris_core::geocentric_six_fields::GeocentricFixedSixFields, GeocentricFieldsError>
{
    geocentric_fixed_six_fields(
        eph,
        target,
        JulianDateTdb(jd),
        GeocentricFixedAxes::IcrfEquatorial,
    )
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.2e}"
    );
}

fn signed_angle_difference_degrees(a: f64, b: f64) -> f64 {
    (a - b + 540.0).rem_euclid(360.0) - 180.0
}

#[test]
fn moving_radial_target_matches_independent_closed_form_reception_solution() {
    let speed = 120_000.0;
    let mut eph = De441::new(Cursor::new(synthetic_kernel([speed, 0.0, 0.0]))).unwrap();
    let result = six_at(&mut eph, JD_MID, Body::Mercury).unwrap();

    // One-dimensional retarded target: r = r0 - v*r/c_day.
    // Therefore r=r0/(1+v/c_day) and dr/dt=v/(1+v/c_day).
    let c_day = SPEED_OF_LIGHT_KM_PER_SECOND * 86_400.0;
    let expected_range = DISTANCE_KM / (1.0 + speed / c_day);
    let expected_range_rate = speed / (1.0 + speed / c_day);
    near(
        result.distance_au,
        expected_range / ASTRONOMICAL_UNIT_KM,
        1.0e-11,
    );
    near(
        result.distance_speed_au_per_tdb_day,
        expected_range_rate / ASTRONOMICAL_UNIT_KM,
        1.0e-10,
    );
    near(
        result.light_time_seconds,
        expected_range / SPEED_OF_LIGHT_KM_PER_SECOND,
        5.0e-5,
    );
    near(
        result.light_time_rate,
        speed / (c_day + speed),
        1.0e-12,
    );
    near(result.longitude_deg, 0.0, 1.0e-12);
    near(result.latitude_deg, 0.0, 1.0e-12);
    near(result.longitude_speed_deg_per_tdb_day, 0.0, 1.0e-12);
    near(result.latitude_speed_deg_per_tdb_day, 0.0, 1.0e-12);
    assert_eq!(result.target, Body::Mercury);
    assert_eq!(result.center, Body::Earth);
    assert!(result.emission_epoch.0 < result.observation_epoch.0);
}

#[test]
fn retarded_angular_and_radial_speeds_match_neighboring_full_kernel_evaluations() {
    let mut eph =
        De441::new(Cursor::new(synthetic_kernel([0.0, 500_000.0, 200_000.0]))).unwrap();
    let h = 0.02;
    let before = six_at(&mut eph, JD_MID - h, Body::Mercury).unwrap();
    let center = six_at(&mut eph, JD_MID, Body::Mercury).unwrap();
    let after = six_at(&mut eph, JD_MID + h, Body::Mercury).unwrap();
    let denominator = 2.0 * h;
    let finite_longitude =
        signed_angle_difference_degrees(after.longitude_deg, before.longitude_deg) / denominator;
    let finite_latitude = (after.latitude_deg - before.latitude_deg) / denominator;
    let finite_distance = (after.distance_au - before.distance_au) / denominator;

    assert!(center.longitude_speed_deg_per_tdb_day > 0.0);
    assert!(center.latitude_speed_deg_per_tdb_day > 0.0);
    near(
        center.longitude_speed_deg_per_tdb_day,
        finite_longitude,
        2.0e-5,
    );
    near(
        center.latitude_speed_deg_per_tdb_day,
        finite_latitude,
        2.0e-5,
    );
    near(
        center.distance_speed_au_per_tdb_day,
        finite_distance,
        2.0e-9,
    );
    // A nonzero vector slope must not be silently replaced with
    // a zero-speed 'position-only' API response.
    assert!(center.longitude_speed_deg_per_tdb_day > 0.1);
    assert!(center.latitude_speed_deg_per_tdb_day > 0.01);
}

#[test]
fn retrograde_direction_and_system_barycenter_names_are_not_lost() {
    let mut eph =
        De441::new(Cursor::new(synthetic_kernel([0.0, -500_000.0, 0.0]))).unwrap();
    let mercury = six_at(&mut eph, JD_MID, Body::Mercury).unwrap();
    assert!(mercury.longitude_speed_deg_per_tdb_day < 0.0);
    near(mercury.latitude_speed_deg_per_tdb_day, 0.0, 1.0e-12);

    let mars = six_at(&mut eph, JD_MID, Body::MarsBarycenter).unwrap();
    assert_eq!(mars.target, Body::MarsBarycenter);
    assert_eq!(mars.center, Body::Earth);
    assert_eq!(mars.axes, GeocentricFixedAxes::IcrfEquatorial);
    near(mars.longitude_deg, 0.0, 1.0e-12);
    near(mars.latitude_deg, 0.0, 1.0e-12);
    near(mars.longitude_speed_deg_per_tdb_day, 0.0, 1.0e-12);
    near(mars.latitude_speed_deg_per_tdb_day, 0.0, 1.0e-12);
    near(mars.distance_speed_au_per_tdb_day, 0.0, 1.0e-12);
    assert!(mars.distance_au > 1.0);
    assert!(mars.light_time_seconds > 0.0);
    near(mars.distance_au * ASTRONOMICAL_UNIT_KM, 180_000_000.0, 1.0e-3);
}

#[test]
fn reception_time_inside_kernel_does_not_hide_missing_emission_coverage() {
    let mut eph = De441::new(Cursor::new(synthetic_kernel([0.0; 3]))).unwrap();
    assert!(matches!(
        six_at(&mut eph, JD_START, Body::Mercury),
        Err(GeocentricFieldsError::LightTime(
            LightTimeError::Ephemeris(De441Error::OutsideCoverage { .. })
        ))
    ));
    let valid = six_at(&mut eph, JD_END, Body::Mercury).unwrap();
    assert_eq!(valid.target, Body::Mercury);
    near(valid.longitude_deg, 0.0, 1.0e-12);
}
