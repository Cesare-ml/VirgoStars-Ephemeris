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

//! Synthetic full-pipeline civil UTC-offset to geometric six-field tests.
//! No real NASA/JPL data or Swiss software is used.

use std::f64::consts::{PI, TAU};
use std::io::Cursor;

use virgostars_ephemeris_core::civil_geocentric::{
    geocentric_fixed_spherical_from_civil, geocentric_fixed_spherical_many_from_civil,
    CivilGeocentricError, CivilGeocentricFixedSpherical,
};
use virgostars_ephemeris_core::coordinates::ProjectionError;
use virgostars_ephemeris_core::de441::{Body, De441, De441Error};
use virgostars_ephemeris_core::first_sky::{CivilTimestamp, SkyError};
use virgostars_ephemeris_core::light_time::{LightTimeError, SPEED_OF_LIGHT_KM_PER_SECOND};

const RECORD_SIZE: usize = 3_000;
const JD_START: f64 = 2_461_344.0;
const JD_MID: f64 = 2_461_360.0;
const JD_END: f64 = 2_461_376.0;
const BASE_RANGE_KM: f64 = 150_000_000.0;

fn write_f64(bytes: &mut [u8], at: usize, value: f64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(bytes: &mut [u8], at: usize, value: i32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// One record containing an invented Mercury trajectory, a stationary
/// Earth-Moon barycenter, and zero geocentric Moon vector.
fn synthetic_classic_de441(velocity_km_per_tdb_day: [f64; 3]) -> Vec<u8> {
    let mut bytes = vec![0_u8; RECORD_SIZE * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    write_f64(&mut bytes, 2652, JD_START);
    write_f64(&mut bytes, 2660, JD_END);
    write_f64(&mut bytes, 2668, 32.0);
    write_f64(&mut bytes, 2688, 81.0);
    write_i32(&mut bytes, 2840, 441);

    // Mercury: three 2-coefficient axis series (words 3..8).
    // EMB: three 1-coefficient axis series (words 9..11).
    // Geocentric Moon: three 1-coefficient axis series (12..14).
    // Unused optional pointer forces the classic 3000-byte record.
    for (index, first, degree) in [(0usize, 3, 2), (2, 9, 1), (9, 12, 1), (12, 373, 1)] {
        let at = if index == 12 { 2844 } else { 2696 + index * 12 };
        write_i32(&mut bytes, at, first);
        write_i32(&mut bytes, at + 4, degree);
        write_i32(&mut bytes, at + 8, 1);
    }

    let at = RECORD_SIZE * 2;
    write_f64(&mut bytes, at, JD_START);
    write_f64(&mut bytes, at + 8, JD_END);
    for (axis, velocity) in velocity_km_per_tdb_day.into_iter().enumerate() {
        // T1(x)=x, x=(JD-JD_MID)/16: c1 = 16 * velocity.
        let word = 2 + 2 * axis;
        write_f64(
            &mut bytes,
            at + word * 8,
            if axis == 0 { BASE_RANGE_KM } else { 0.0 },
        );
        write_f64(&mut bytes, at + (word + 1) * 8, velocity * 16.0);
    }
    bytes
}

fn stamp(s: &str) -> CivilTimestamp {
    CivilTimestamp::parse(s).unwrap()
}

fn one_call(
    eph: &mut De441<Cursor<Vec<u8>>>,
    timestamp: &str,
    target: Body,
    tilt: f64,
) -> Result<CivilGeocentricFixedSpherical, CivilGeocentricError> {
    geocentric_fixed_spherical_from_civil(eph, target, stamp(timestamp), tilt)
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.14e}, expected={expected:.14e}, tolerance={tolerance:.3e}"
    );
}

#[test]
fn same_instant_different_timezones_and_analytic_radial_speed() {
    let radial_km_per_day = 120_000.0;
    let mut eph = De441::new(Cursor::new(synthetic_classic_de441([
        radial_km_per_day,
        0.0,
        0.0,
    ])))
    .unwrap();
    let a = one_call(&mut eph, "2026-11-10T13:30:00+01:00", Body::Mercury, 0.0).unwrap();
    let b = one_call(&mut eph, "2026-11-10T12:30:00Z", Body::Mercury, 0.0).unwrap();

    near(a.spherical.epoch.0, b.spherical.epoch.0, 1.0e-10);
    near(a.spherical.distance_km, b.spherical.distance_km, 1.0e-8);
    near(
        a.spherical.distance_rate_km_per_day,
        b.spherical.distance_rate_km_per_day,
        1.0e-8,
    );

    // With a stationary Earth and a radially departing linear target,
    // r(t_obs)=(r0+v*(t_obs-JD_MID))/(1+v/c_day).
    let c_day = SPEED_OF_LIGHT_KM_PER_SECOND * 86_400.0;
    let unreduced_range = BASE_RANGE_KM + radial_km_per_day * (a.spherical.epoch.0 - JD_MID);
    let expected_range = unreduced_range / (1.0 + radial_km_per_day / c_day);
    let expected_range_rate = radial_km_per_day / (1.0 + radial_km_per_day / c_day);

    near(a.spherical.distance_km, expected_range, 0.02);
    near(
        a.spherical.distance_rate_km_per_day,
        expected_range_rate,
        1.0e-6,
    );
    near(
        a.light_time_seconds,
        expected_range / SPEED_OF_LIGHT_KM_PER_SECOND,
        0.000_1,
    );
    near(a.spherical.longitude_rad, 0.0, 1.0e-12);
    near(a.spherical.latitude_rad, 0.0, 1.0e-12);
    near(a.spherical.longitude_rate_rad_per_day, 0.0, 1.0e-12);
    near(a.spherical.latitude_rate_rad_per_day, 0.0, 1.0e-12);
    assert!(a.emission_epoch.0 < a.spherical.epoch.0);
    assert_eq!(a.spherical.target, Body::Mercury);
    assert_eq!(a.spherical.center, Body::Earth);
    assert_eq!(a.spherical.tilt_rad, 0.0);
}

#[test]
fn full_civil_to_six_field_angular_rates_match_independent_time_steps() {
    let mut eph = De441::new(Cursor::new(synthetic_classic_de441([
        50_000.0, 500_000.0, 175_000.0,
    ])))
    .unwrap();
    let left = one_call(&mut eph, "2026-11-10T12:28:00Z", Body::Mercury, 0.0).unwrap();
    let center = one_call(&mut eph, "2026-11-10T12:30:00Z", Body::Mercury, 0.0).unwrap();
    let right = one_call(&mut eph, "2026-11-10T12:32:00Z", Body::Mercury, 0.0).unwrap();
    let h = right.spherical.epoch.0 - left.spherical.epoch.0;
    assert!(h > 0.0);

    let signed_lon_delta =
        (right.spherical.longitude_rad - left.spherical.longitude_rad + PI).rem_euclid(TAU) - PI;
    let longitude_rate = signed_lon_delta / h;
    let latitude_rate = (right.spherical.latitude_rad - left.spherical.latitude_rad) / h;
    let distance_rate = (right.spherical.distance_km - left.spherical.distance_km) / h;

    near(
        center.spherical.longitude_rate_rad_per_day,
        longitude_rate,
        2.0e-7,
    );
    near(
        center.spherical.latitude_rate_rad_per_day,
        latitude_rate,
        2.0e-7,
    );
    near(
        center.spherical.distance_rate_km_per_day,
        distance_rate,
        2.0,
    );
    assert!(center.spherical.longitude_rate_rad_per_day > 0.001);
    assert!(center.spherical.latitude_rate_rad_per_day > 0.0001);
    assert!(center.light_time_seconds > 0.0);
}

#[test]
fn retrograde_speed_is_signed_and_fixed_rotation_is_explicit() {
    let mut eph = De441::new(Cursor::new(synthetic_classic_de441([0.0, -400_000.0, 0.0]))).unwrap();
    let result = one_call(&mut eph, "2026-11-10T12:30:00Z", Body::Mercury, 0.0).unwrap();
    assert!(result.spherical.longitude_rate_rad_per_day < 0.0);

    let tilted = one_call(&mut eph, "2026-11-10T12:30:00Z", Body::Mercury, PI / 4.0).unwrap();
    near(
        tilted.spherical.distance_km,
        result.spherical.distance_km,
        1.0e-6,
    );
    near(
        tilted.spherical.distance_rate_km_per_day,
        result.spherical.distance_rate_km_per_day,
        1.0e-6,
    );
    near(tilted.spherical.tilt_rad, PI / 4.0, 0.0);
}

#[test]
fn conversion_kernel_and_projection_errors_remain_distinct() {
    let mut eph = De441::new(Cursor::new(synthetic_classic_de441([0.0; 3]))).unwrap();
    let bad_time = geocentric_fixed_spherical_from_civil(
        &mut eph,
        Body::Mercury,
        stamp("1969-01-01T12:30:00Z"),
        0.0,
    );
    assert!(matches!(
        bad_time,
        Err(CivilGeocentricError::CivilTime(SkyError::UnsupportedUtc))
    ));

    assert!(matches!(
        one_call(&mut eph, "2026-11-10T12:30:00Z", Body::Earth, 0.0),
        Err(CivilGeocentricError::LightTime(
            LightTimeError::UndefinedDirection
        ))
    ));

    assert!(matches!(
        one_call(&mut eph, "2026-11-10T12:30:00Z", Body::Sun, 0.0),
        Err(CivilGeocentricError::LightTime(LightTimeError::Ephemeris(
            De441Error::MissingSeries(_)
        )))
    ));

    assert!(matches!(
        one_call(&mut eph, "2026-11-10T12:30:00Z", Body::Mercury, f64::NAN),
        Err(CivilGeocentricError::Projection(ProjectionError::NonFinite))
    ));

    // Reception near the file's first JD does not hide the fact that
    // the retarded emission epoch falls outside the ephemeris.
    assert!(matches!(
        one_call(&mut eph, "2026-10-30T12:00:00Z", Body::Mercury, 0.0),
        Err(CivilGeocentricError::LightTime(LightTimeError::Ephemeris(
            De441Error::OutsideCoverage { .. }
        )))
    ));
}

/// Add an independent fixed Venus series after Mercury, EMB and Moon,
/// without changing the original one-body synthetic fixtures.
fn synthetic_two_body_kernel(mercury_velocity: [f64; 3]) -> Vec<u8> {
    let mut bytes = synthetic_classic_de441(mercury_velocity);
    let venus_pointer = 2696 + 12;
    write_i32(&mut bytes, venus_pointer, 15);
    write_i32(&mut bytes, venus_pointer + 4, 1);
    write_i32(&mut bytes, venus_pointer + 8, 1);
    let venus_x = RECORD_SIZE * 2 + (15 - 1) * 8;
    write_f64(&mut bytes, venus_x, 210_000_000.0);
    bytes
}

#[test]
fn batch_preserves_order_duplicates_metadata_and_exact_scalar_results() {
    let mut eph = De441::new(Cursor::new(synthetic_two_body_kernel([
        0.0, 300_000.0, 100_000.0,
    ])))
    .unwrap();
    let civil = stamp("2026-11-10T13:30:00+01:00");
    let targets = [Body::Venus, Body::Mercury, Body::Venus];
    let batch = geocentric_fixed_spherical_many_from_civil(&mut eph, &targets, civil, 0.0).unwrap();
    assert_eq!(batch.len(), targets.len());
    assert_eq!(batch[0], batch[2]);
    assert_ne!(
        batch[0].spherical.distance_km,
        batch[1].spherical.distance_km
    );
    for (actual, target) in batch.iter().zip(targets) {
        let expected = geocentric_fixed_spherical_from_civil(&mut eph, target, civil, 0.0).unwrap();
        assert_eq!(*actual, expected);
        assert_eq!(actual.spherical.target, target);
        assert_eq!(actual.spherical.center, Body::Earth);
        assert!(actual.emission_epoch.0 < actual.spherical.epoch.0);
    }
}

#[test]
fn empty_batch_validates_civil_time_and_axes_without_reading_body_series() {
    // This synthetic reader has no Sun series, but an empty request is valid.
    let mut eph = De441::new(Cursor::new(synthetic_classic_de441([0.0; 3]))).unwrap();
    let empty = geocentric_fixed_spherical_many_from_civil(
        &mut eph,
        &[],
        stamp("2026-11-10T12:30:00Z"),
        0.0,
    )
    .unwrap();
    assert!(empty.is_empty());

    let bad_time = geocentric_fixed_spherical_many_from_civil(
        &mut eph,
        &[],
        stamp("1969-01-01T12:30:00Z"),
        0.0,
    );
    assert!(matches!(
        bad_time,
        Err(CivilGeocentricError::CivilTime(SkyError::UnsupportedUtc))
    ));

    let bad_tilt = geocentric_fixed_spherical_many_from_civil(
        &mut eph,
        &[],
        stamp("2026-11-10T12:30:00Z"),
        f64::NAN,
    );
    assert!(matches!(
        bad_tilt,
        Err(CivilGeocentricError::Projection(ProjectionError::NonFinite))
    ));
}

#[test]
fn batch_fails_whole_call_on_missing_series_and_can_be_reused() {
    let mut eph = De441::new(Cursor::new(synthetic_two_body_kernel([0.0; 3]))).unwrap();
    let civil = stamp("2026-11-10T12:30:00Z");
    // The first requested body succeeds internally. A missing second
    // body still rejects the entire result, never returning a partial Vec.
    let failed = geocentric_fixed_spherical_many_from_civil(
        &mut eph,
        &[Body::Mercury, Body::Sun, Body::Venus],
        civil,
        0.0,
    );
    assert!(matches!(
        failed,
        Err(CivilGeocentricError::LightTime(LightTimeError::Ephemeris(
            De441Error::MissingSeries(_)
        )))
    ));

    let resumed = geocentric_fixed_spherical_many_from_civil(
        &mut eph,
        &[Body::Mercury, Body::Venus],
        civil,
        0.0,
    )
    .unwrap();
    assert_eq!(resumed.len(), 2);
    assert_eq!(resumed[0].spherical.target, Body::Mercury);
    assert_eq!(resumed[1].spherical.target, Body::Venus);

    let earth = geocentric_fixed_spherical_many_from_civil(&mut eph, &[Body::Earth], civil, 0.0);
    assert!(matches!(
        earth,
        Err(CivilGeocentricError::LightTime(
            LightTimeError::UndefinedDirection
        ))
    ));
}

#[test]
fn batch_timezone_equivalence_preserves_signed_retrograde_rates() {
    let mut eph = De441::new(Cursor::new(synthetic_two_body_kernel([
        0.0, -350_000.0, 0.0,
    ])))
    .unwrap();
    let targets = [Body::Mercury, Body::Venus];
    let local = geocentric_fixed_spherical_many_from_civil(
        &mut eph,
        &targets,
        stamp("2026-11-10T13:30:00+01:00"),
        0.0,
    )
    .unwrap();
    let utc = geocentric_fixed_spherical_many_from_civil(
        &mut eph,
        &targets,
        stamp("2026-11-10T12:30:00Z"),
        0.0,
    )
    .unwrap();
    assert_eq!(local, utc);
    assert!(local[0].spherical.longitude_rate_rad_per_day < 0.0);
    near(local[1].spherical.longitude_rate_rad_per_day, 0.0, 1.0e-15);
}
