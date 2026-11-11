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

//! Mathematical and synthetic-file regressions for the MVP-1 geocentric
//! fixed-frame six-field pipeline. No JPL kernel or Swiss software bundled.

use std::f64::consts::{FRAC_PI_2, PI};
use std::io::Cursor;

use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::geocentric_six_fields::{
    fields_from_light_time, geocentric_fixed_six_fields, GeocentricFieldsError,
    GeocentricFixedAxes, ASTRONOMICAL_UNIT_KM,
};
use virgostars_ephemeris_core::light_time::{
    GeocentricLightTimeState, LightTimeError, SPEED_OF_LIGHT_KM_PER_SECOND,
};

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.2e}"
    );
}

fn state(position: [f64; 3], velocity: [f64; 3]) -> GeocentricLightTimeState {
    GeocentricLightTimeState {
        target: Body::Mercury,
        observation_epoch: JulianDateTdb(2_451_545.0),
        emission_epoch: JulianDateTdb(2_451_545.0 - 1e-4),
        light_time_seconds: 8.64,
        light_time_rate: -0.000_1,
        position_km: position,
        velocity_km_per_day: velocity,
    }
}

#[test]
fn six_fields_have_explicit_iau_au_degrees_and_tdb_day_rates() {
    let au = ASTRONOMICAL_UNIT_KM;
    let input = state(
        [au, 0.0, 0.0],
        [0.02 * au, (2.0 * PI / 180.0) * au, 0.01 * au],
    );
    let result = fields_from_light_time(&input, GeocentricFixedAxes::IcrfEquatorial).unwrap();
    near(result.longitude_deg, 0.0, 1e-12);
    near(result.latitude_deg, 0.0, 1e-12);
    near(result.distance_au, 1.0, 1e-15);
    near(result.longitude_speed_deg_per_tdb_day, 2.0, 1e-12);
    near(result.latitude_speed_deg_per_tdb_day, 0.01 * 180.0 / PI, 1e-12);
    near(result.distance_speed_au_per_tdb_day, 0.02, 1e-15);
    assert_eq!(result.center, Body::Earth);
    assert_eq!(result.target, Body::Mercury);
    assert_eq!(result.observation_epoch, input.observation_epoch);
    assert_eq!(result.emission_epoch, input.emission_epoch);
    assert_eq!(result.light_time_seconds, input.light_time_seconds);
    assert_eq!(result.light_time_rate, input.light_time_rate);
}

#[test]
fn explicit_fixed_tilt_rotates_position_and_velocity_without_a_date_frame() {
    let au = ASTRONOMICAL_UNIT_KM;
    // For a +90° passive rotation around X, (x,y,z)=(AU,0,AU/2)
    // becomes (AU,AU/2,0), hence atan(1/2) longitude and zero latitude.
    // A Y-directed velocity becomes -Z-directed, making latitude decrease.
    let input = state([au, 0.0, au / 2.0], [0.0, au, 0.0]);
    let result = fields_from_light_time(
        &input,
        GeocentricFixedAxes::TiltedIcrf { tilt_rad: FRAC_PI_2 },
    )
    .unwrap();
    near(result.longitude_deg, (0.5f64).atan() * 180.0 / PI, 1e-12);
    near(result.latitude_deg, 0.0, 1e-12);
    near(result.distance_au, (1.25_f64).sqrt(), 1e-14);
    near(
        result.latitude_speed_deg_per_tdb_day,
        -(1.0 / (1.25_f64).sqrt()) * 180.0 / PI,
        1e-12,
    );
    assert_eq!(
        result.axes,
        GeocentricFixedAxes::TiltedIcrf { tilt_rad: FRAC_PI_2 }
    );
}

#[test]
fn negative_longitude_is_normalized_and_retrograde_speed_is_preserved() {
    let au = ASTRONOMICAL_UNIT_KM;
    let input = state([0.0, -au, 0.0], [-0.2 * au, 0.0, 0.0]);
    let result = fields_from_light_time(&input, GeocentricFixedAxes::IcrfEquatorial).unwrap();
    near(result.longitude_deg, 270.0, 1e-12);
    // dλ=(x*vy-y*vx)/(x²+y²) = -0.2 rad/day at (0,-AU).
    near(result.longitude_speed_deg_per_tdb_day, -0.2 * 180.0 / PI, 1e-12);
}

#[test]
fn light_time_derivative_is_used_instead_of_undoing_it() {
    // This synthetic velocity is already d(position at retarded emission
    // minus Earth at reception)/d(observation TDB). No extra retiming or
    // velocity subtraction is permitted in the projection bridge.
    let au = ASTRONOMICAL_UNIT_KM;
    let input = state([au, 0.0, 0.0], [0.0, 0.2 * au, 0.0]);
    let result = fields_from_light_time(&input, GeocentricFixedAxes::IcrfEquatorial).unwrap();
    near(result.longitude_speed_deg_per_tdb_day, 0.2 * 180.0 / PI, 1e-12);
    near(result.distance_speed_au_per_tdb_day, 0.0, 1e-15);
}

#[test]
fn undefined_angles_invalid_tilt_and_nonfinite_geometry_fail_closed() {
    let zeros = state([0.0; 3], [0.0; 3]);
    assert!(matches!(
        fields_from_light_time(&zeros, GeocentricFixedAxes::IcrfEquatorial),
        Err(GeocentricFieldsError::Projection(_))
    ));
    let pole = state([0.0, 0.0, 1.0], [0.0; 3]);
    assert!(matches!(
        fields_from_light_time(&pole, GeocentricFixedAxes::IcrfEquatorial),
        Err(GeocentricFieldsError::Projection(_))
    ));
    let vector = state([ASTRONOMICAL_UNIT_KM, 0.0, 0.0], [0.0; 3]);
    assert!(matches!(
        fields_from_light_time(
            &vector,
            GeocentricFixedAxes::TiltedIcrf { tilt_rad: f64::NAN }
        ),
        Err(GeocentricFieldsError::Projection(_))
    ));
    let invalid_velocity = state([1.0, 1.0, 1.0], [f64::INFINITY, 0.0, 0.0]);
    assert!(matches!(
        fields_from_light_time(&invalid_velocity, GeocentricFixedAxes::IcrfEquatorial),
        Err(GeocentricFieldsError::Projection(_))
    ));
}

#[test]
fn corrupted_reception_emission_or_lighttime_metadata_is_rejected() {
    let valid = state([1.0, 2.0, 3.0], [0.0; 3]);
    let mut bad = valid;
    bad.target = Body::Earth;
    assert!(matches!(
        fields_from_light_time(&bad, GeocentricFixedAxes::IcrfEquatorial),
        Err(GeocentricFieldsError::InvalidIntermediate)
    ));
    bad = valid;
    bad.emission_epoch = JulianDateTdb(valid.observation_epoch.0 + 1.0);
    assert!(matches!(
        fields_from_light_time(&bad, GeocentricFixedAxes::IcrfEquatorial),
        Err(GeocentricFieldsError::InvalidIntermediate)
    ));
    bad = valid;
    bad.light_time_seconds = f64::NAN;
    assert!(matches!(
        fields_from_light_time(&bad, GeocentricFixedAxes::IcrfEquatorial),
        Err(GeocentricFieldsError::InvalidIntermediate)
    ));
    bad = valid;
    bad.light_time_rate = f64::INFINITY;
    assert!(matches!(
        fields_from_light_time(&bad, GeocentricFixedAxes::IcrfEquatorial),
        Err(GeocentricFieldsError::InvalidIntermediate)
    ));
}

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// A minimal **synthetic** classic DE441 file with Mercury = (2000,0,0)
/// km and Earth = (1000,0,0) km, both barycentric and stationary.
/// Reader end-to-end coverage does not establish real NASA numerical accuracy.
fn synthetic_classic_de441() -> Vec<u8> {
    let record_size = 3_000_usize;
    let mut bytes = vec![0_u8; record_size * 4];
    let name = b"JPL Planetary Ephemeris DE441";
    bytes[..name.len()].copy_from_slice(name);
    put_f64(&mut bytes, 2652, 1_000.0);
    put_f64(&mut bytes, 2660, 1_064.0);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);
    // Mercury, EMB, geocentric Moon, then a synthetic unused last series
    // pointer to reserve a complete 3000-byte classic coefficient record.
    for (index, first) in [(0, 3), (2, 6), (9, 9), (12, 373)] {
        let offset = if index == 12 { 2844 } else { 2696 + index * 12 };
        put_i32(&mut bytes, offset, first);
        put_i32(&mut bytes, offset + 4, 1);
        put_i32(&mut bytes, offset + 8, 1);
    }
    for record in 0..2 {
        let at = record_size * (record + 2);
        put_f64(&mut bytes, at, 1_000.0 + record as f64 * 32.0);
        put_f64(&mut bytes, at + 8, 1_032.0 + record as f64 * 32.0);
        put_f64(&mut bytes, at + 2 * 8, 2_000.0); // Mercury X
        put_f64(&mut bytes, at + 5 * 8, 1_000.0); // EMB X = Earth X (Moon=0)
    }
    bytes
}

#[test]
fn one_call_reads_synthetic_de441_solves_delay_and_outputs_all_six_fields() {
    let mut eph = De441::new(Cursor::new(synthetic_classic_de441())).unwrap();
    let jd = JulianDateTdb(1_016.0);
    let answer = geocentric_fixed_six_fields(
        &mut eph,
        Body::Mercury,
        jd,
        GeocentricFixedAxes::IcrfEquatorial,
    )
    .unwrap();
    assert_eq!(answer.target, Body::Mercury);
    assert_eq!(answer.center, Body::Earth);
    near(answer.longitude_deg, 0.0, 1e-12);
    near(answer.latitude_deg, 0.0, 1e-12);
    near(answer.distance_au, 1_000.0 / ASTRONOMICAL_UNIT_KM, 1e-15);
    near(answer.longitude_speed_deg_per_tdb_day, 0.0, 1e-12);
    near(answer.latitude_speed_deg_per_tdb_day, 0.0, 1e-12);
    near(answer.distance_speed_au_per_tdb_day, 0.0, 1e-14);
    near(answer.light_time_seconds, 1_000.0 / SPEED_OF_LIGHT_KM_PER_SECOND, 1e-9);
    assert!(answer.emission_epoch.0 < answer.observation_epoch.0);
}

#[test]
fn one_call_propagates_missing_series_and_rejects_earth_observation_target() {
    let mut eph = De441::new(Cursor::new(synthetic_classic_de441())).unwrap();
    let jd = JulianDateTdb(1_016.0);
    assert!(matches!(
        geocentric_fixed_six_fields(
            &mut eph,
            Body::Earth,
            jd,
            GeocentricFixedAxes::IcrfEquatorial,
        ),
        Err(GeocentricFieldsError::LightTime(LightTimeError::UndefinedDirection))
    ));
    assert!(matches!(
        geocentric_fixed_six_fields(
            &mut eph,
            Body::Sun,
            jd,
            GeocentricFixedAxes::IcrfEquatorial,
        ),
        Err(GeocentricFieldsError::LightTime(LightTimeError::Ephemeris(_)))
    ));
}

#[test]
fn longitude_rate_is_finite_difference_of_fixed_axis_angles() {
    let au = ASTRONOMICAL_UNIT_KM;
    // Synthetic constant circular motion around the origin. The *input
    // Cartesian states* at neighboring epochs are independently generated,
    // rather than using the angular derivative inside the module.
    let phase = 1.0_f64;
    let rate = -0.012_f64;
    let h = 0.001_f64;
    let evaluate = |elapsed: f64| {
        let a = phase + elapsed * rate;
        let input = state(
            [au * a.cos(), au * a.sin(), 0.0],
            [-au * rate * a.sin(), au * rate * a.cos(), 0.0],
        );
        fields_from_light_time(&input, GeocentricFixedAxes::IcrfEquatorial).unwrap()
    };
    let before = evaluate(-h);
    let after = evaluate(h);
    let current = evaluate(0.0);
    let independent = (after.longitude_deg - before.longitude_deg) / (2.0 * h);
    near(current.longitude_speed_deg_per_tdb_day, independent, 1e-9);
    near(current.longitude_speed_deg_per_tdb_day, rate * 180.0 / PI, 1e-12);
}
