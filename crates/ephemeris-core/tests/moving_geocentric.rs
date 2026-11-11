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

//! Offline independent mathematics and synthetic classic-DE441 tests for
//! the **moving-frame** geocentric spherical six-field composition.
//! No official JPL file, IAU model or Swiss source is included.

use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::io::Cursor;

use virgostars_ephemeris_core::coordinates::{project_fixed_axes, ProjectionError};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::light_time::{
    GeocentricLightTimeState, LightTimeError, SPEED_OF_LIGHT_KM_PER_SECOND,
};
use virgostars_ephemeris_core::moving_geocentric::{
    geocentric_light_time_moving_spherical, moving_spherical_from_icrf_state,
    moving_spherical_from_light_time, MovingGeocentricError,
};
use virgostars_ephemeris_core::rotating_frame::{RotatingFrame, TtPerTdbDay};

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual {actual:.16e}, expected {expected:.16e}, tolerance {tolerance:.2e}"
    );
}

fn input(position: [f64; 3], velocity: [f64; 3]) -> GeocentricLightTimeState {
    GeocentricLightTimeState {
        target: Body::Mercury,
        observation_epoch: JulianDateTdb(2_451_545.0),
        emission_epoch: JulianDateTdb(2_451_544.995),
        light_time_seconds: 432.0,
        light_time_rate: -0.001,
        position_km: position,
        velocity_km_per_day: velocity,
    }
}

fn passive_z(angle: f64, rate: f64) -> RotatingFrame {
    let (sin, cos) = angle.sin_cos();
    RotatingFrame::new(
        [[cos, sin, 0.0], [-sin, cos, 0.0], [0.0, 0.0, 1.0]],
        [
            [-rate * sin, rate * cos, 0.0],
            [-rate * cos, -rate * sin, 0.0],
            [0.0, 0.0, 0.0],
        ],
    )
    .unwrap()
}

fn passive_x(angle: f64, rate: f64) -> RotatingFrame {
    let (sin, cos) = angle.sin_cos();
    RotatingFrame::new(
        [[1.0, 0.0, 0.0], [0.0, cos, sin], [0.0, -sin, cos]],
        [
            [0.0; 3],
            [0.0, -rate * sin, rate * cos],
            [0.0, -rate * cos, -rate * sin],
        ],
    )
    .unwrap()
}

#[test]
fn identity_frame_matches_existing_projector_for_all_six_fields() {
    let source = input([8.0e7, -4.0e7, 2.0e7], [11_000.0, 20_000.0, -3_000.0]);
    let expected_state = StateVector {
        target: source.target,
        center: Body::Earth,
        epoch: source.observation_epoch,
        position_km: source.position_km,
        velocity_km_per_day: source.velocity_km_per_day,
    };
    let expected = project_fixed_axes(&expected_state, 0.0).unwrap();
    let result = moving_spherical_from_light_time(&source, &passive_z(0.0, 0.0)).unwrap();
    assert_eq!(result.spherical, expected);
    assert_eq!(result.emission_epoch, source.emission_epoch);
    assert_eq!(result.light_time_seconds, source.light_time_seconds);
    assert_eq!(result.light_time_rate, source.light_time_rate);
    assert_eq!(result.spherical.center, Body::Earth);
    assert_eq!(result.spherical.target, Body::Mercury);
    assert_eq!(result.spherical.epoch, source.observation_epoch);
    assert_eq!(result.spherical.tilt_rad, 0.0);
}

#[test]
fn moving_axes_create_negative_longitude_rate_for_stationary_target() {
    let radius = 384_400.0;
    let source = input([radius, 0.0, 0.0], [0.0; 3]);
    let result = moving_spherical_from_light_time(&source, &passive_z(0.0, 0.1)).unwrap();
    near(result.spherical.longitude_rad, 0.0, 1e-15);
    near(result.spherical.latitude_rad, 0.0, 1e-15);
    near(result.spherical.distance_km, radius, 1e-9);
    near(result.spherical.longitude_rate_rad_per_day, -0.1, 1e-15);
    near(result.spherical.latitude_rate_rad_per_day, 0.0, 1e-15);
    near(result.spherical.distance_rate_km_per_day, 0.0, 1e-9);
}

#[test]
fn moving_x_axes_create_nonzero_latitude_rate_without_physical_motion() {
    let source = input([0.0, 2.0e5, 0.0], [0.0; 3]);
    let result = moving_spherical_from_light_time(&source, &passive_x(0.0, 0.07)).unwrap();
    near(result.spherical.longitude_rad, FRAC_PI_2, 1e-15);
    near(result.spherical.latitude_rad, 0.0, 1e-15);
    near(result.spherical.latitude_rate_rad_per_day, -0.07, 1e-15);
    near(result.spherical.longitude_rate_rad_per_day, 0.0, 1e-15);
    near(result.spherical.distance_rate_km_per_day, 0.0, 1e-9);
}

#[test]
fn independent_centered_differences_match_all_three_moving_spherical_rates() {
    let r0 = [2.5e7, -5.0e7, 8.0e6];
    let v = [14_000.0, 30_000.0, -9_000.0];
    let theta0 = 0.3;
    let rate = 0.024;
    let h = 0.001;
    let at = |t: f64| {
        let pos = std::array::from_fn::<_, 3, _>(|i| r0[i] + v[i] * t);
        let mut state = input(pos, v);
        state.observation_epoch = JulianDateTdb(2_451_545.0 + t);
        state.emission_epoch = JulianDateTdb(2_451_544.995 + t);
        moving_spherical_from_light_time(&state, &passive_z(theta0 + rate * t, rate))
            .unwrap()
            .spherical
    };
    let now = at(0.0);
    let before = at(-h);
    let after = at(h);
    let signed = (after.longitude_rad - before.longitude_rad + PI).rem_euclid(TAU) - PI;
    near(now.longitude_rate_rad_per_day, signed / (2.0 * h), 5e-10);
    near(
        now.latitude_rate_rad_per_day,
        (after.latitude_rad - before.latitude_rad) / (2.0 * h),
        5e-10,
    );
    near(
        now.distance_rate_km_per_day,
        (after.distance_km - before.distance_km) / (2.0 * h),
        2e-5,
    );
}

#[test]
fn rotation_preserves_distance_and_signed_radial_rate() {
    let source = input([4.0e7, -1.0e7, 2.0e7], [-11_000.0, 9_000.0, 8_000.0]);
    let reference = moving_spherical_from_light_time(&source, &passive_z(0.0, 0.0)).unwrap();
    for angle in [-PI, -0.2, 0.0, 0.8, PI] {
        let actual = moving_spherical_from_light_time(&source, &passive_z(angle, 0.15)).unwrap();
        near(
            actual.spherical.distance_km,
            reference.spherical.distance_km,
            1e-8,
        );
        near(
            actual.spherical.distance_rate_km_per_day,
            reference.spherical.distance_rate_km_per_day,
            1e-9,
        );
    }
}

#[test]
fn invalid_epochs_zero_delay_nonfinite_rates_or_earth_target_fail_closed() {
    let base = input([1.0e7, 2.0e7, 0.0], [0.0; 3]);
    let frame = passive_z(0.0, 0.02);
    let mut bad = base;
    bad.target = Body::Earth;
    assert!(matches!(
        moving_spherical_from_light_time(&bad, &frame),
        Err(MovingGeocentricError::InvalidIntermediate)
    ));
    bad = base;
    bad.emission_epoch = JulianDateTdb(base.observation_epoch.0 + 1.0);
    assert!(matches!(
        moving_spherical_from_light_time(&bad, &frame),
        Err(MovingGeocentricError::InvalidIntermediate)
    ));
    bad = base;
    bad.light_time_seconds = 0.0;
    assert!(matches!(
        moving_spherical_from_light_time(&bad, &frame),
        Err(MovingGeocentricError::InvalidIntermediate)
    ));
    bad = base;
    bad.light_time_rate = f64::INFINITY;
    assert!(matches!(
        moving_spherical_from_light_time(&bad, &frame),
        Err(MovingGeocentricError::InvalidIntermediate)
    ));
    bad = base;
    bad.velocity_km_per_day[0] = f64::NAN;
    assert!(matches!(
        moving_spherical_from_light_time(&bad, &frame),
        Err(MovingGeocentricError::InvalidIntermediate)
    ));
}

#[test]
fn moving_frame_propagates_pole_singularity_and_preserves_barycenter_id() {
    let mut pole = input([0.0, 0.0, 1.0], [0.0; 3]);
    pole.target = Body::JupiterBarycenter;
    assert!(matches!(
        moving_spherical_from_light_time(&pole, &passive_z(0.0, 0.0)),
        Err(MovingGeocentricError::Projection(
            ProjectionError::UndefinedDirection
        ))
    ));
    pole.position_km = [1.0, 0.0, 0.0];
    let result = moving_spherical_from_light_time(&pole, &passive_z(PI, 0.1)).unwrap();
    assert_eq!(result.spherical.target, Body::JupiterBarycenter);
    assert_eq!(result.spherical.center, Body::Earth);
    near(result.spherical.longitude_rad, PI, 1e-15);
    near(result.spherical.longitude_rate_rad_per_day, -0.1, 1e-15);
}

fn set_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn set_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// Minimal synthetic classic-DE441 file, not a NASA numerical reference.
/// Two 32-day data records, Mercury=(2000,0,0) km, Earth=(1000,0,0)
/// km and Moon relative Earth=(0,0,0), all with zero speed.
fn synthetic_de441() -> Vec<u8> {
    let record_size = 3_000_usize;
    let mut bytes = vec![0_u8; record_size * 4];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    set_f64(&mut bytes, 2652, 1_000.0);
    set_f64(&mut bytes, 2660, 1_064.0);
    set_f64(&mut bytes, 2668, 32.0);
    set_f64(&mut bytes, 2688, 81.0);
    set_i32(&mut bytes, 2840, 441);
    for (index, start) in [(0, 3), (2, 6), (9, 9), (12, 373)] {
        let location = if index == 12 { 2844 } else { 2696 + index * 12 };
        set_i32(&mut bytes, location, start);
        set_i32(&mut bytes, location + 4, 1);
        set_i32(&mut bytes, location + 8, 1);
    }
    for i in 0..2 {
        let offset = record_size * (i + 2);
        set_f64(&mut bytes, offset, 1_000.0 + (i as f64) * 32.0);
        set_f64(&mut bytes, offset + 8, 1_032.0 + (i as f64) * 32.0);
        set_f64(&mut bytes, offset + 2 * 8, 2_000.0);
        set_f64(&mut bytes, offset + 5 * 8, 1_000.0);
    }
    bytes
}

#[test]
fn real_one_call_de441_light_time_and_moving_axes_gives_all_six_fields() {
    let mut reader = De441::new(Cursor::new(synthetic_de441())).unwrap();
    let reception = JulianDateTdb(1_016.0);
    let result = geocentric_light_time_moving_spherical(
        &mut reader,
        Body::Mercury,
        reception,
        &passive_z(0.0, 0.125),
    )
    .unwrap();
    assert_eq!(result.spherical.target, Body::Mercury);
    assert_eq!(result.spherical.center, Body::Earth);
    assert_eq!(result.spherical.epoch, reception);
    near(result.spherical.longitude_rad, 0.0, 1e-15);
    near(result.spherical.latitude_rad, 0.0, 1e-15);
    near(result.spherical.distance_km, 1_000.0, 1e-8);
    near(result.spherical.longitude_rate_rad_per_day, -0.125, 1e-15);
    near(result.spherical.latitude_rate_rad_per_day, 0.0, 1e-15);
    near(result.spherical.distance_rate_km_per_day, 0.0, 1e-10);
    near(
        result.light_time_seconds,
        1_000.0 / SPEED_OF_LIGHT_KM_PER_SECOND,
        1e-9,
    );
    assert!(result.emission_epoch.0 < result.spherical.epoch.0);
}

#[test]
fn one_call_returns_missing_ephemeris_body_and_undefined_earth_errors() {
    let mut reader = De441::new(Cursor::new(synthetic_de441())).unwrap();
    let reception = JulianDateTdb(1_016.0);
    let frame = passive_z(0.0, 0.1);
    assert!(matches!(
        geocentric_light_time_moving_spherical(&mut reader, Body::Earth, reception, &frame,),
        Err(MovingGeocentricError::LightTime(
            LightTimeError::UndefinedDirection
        ))
    ));
    assert!(matches!(
        geocentric_light_time_moving_spherical(&mut reader, Body::Sun, reception, &frame,),
        Err(MovingGeocentricError::LightTime(LightTimeError::Ephemeris(
            _
        )))
    ));
}

#[test]
fn previously_corrected_cartesian_velocity_is_rotated_once_with_rdot_term() {
    // Models a light-time/aberration-corrected ICRF Cartesian state,
    // supplied by a separate astronomical stage. The moving-axis
    // adapter must not add new light-time or aberration.
    let already_corrected = StateVector {
        target: Body::Venus,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        position_km: [20_000.0, 0.0, 0.0],
        velocity_km_per_day: [100.0, 200.0, 0.0],
    };
    let moved =
        moving_spherical_from_icrf_state(&already_corrected, &passive_z(0.0, 0.05)).unwrap();
    let s = moved.spherical;
    assert_eq!(s.target, Body::Venus);
    assert_eq!(s.center, Body::Earth);
    assert_eq!(s.epoch, already_corrected.epoch);
    assert_eq!(s.tilt_rad, 0.0);
    near(s.longitude_rad, 0.0, 1e-15);
    near(s.latitude_rad, 0.0, 1e-15);
    near(s.distance_km, 20_000.0, 1e-9);
    // The physical velocity creates +200/20_000 = 0.01 rad/day,
    // but moving axes subtract 0.05 rad/day, leaving -0.04 rad/day.
    near(s.longitude_rate_rad_per_day, -0.04, 1e-15);
    near(s.distance_rate_km_per_day, 100.0, 1e-12);
    near(s.latitude_rate_rad_per_day, 0.0, 1e-15);
    // Raw fixed-axis projection must NOT be mistaken for moving speed.
    let inertial = project_fixed_axes(&already_corrected, 0.0).unwrap();
    near(inertial.longitude_rate_rad_per_day, 0.01, 1e-15);
}

#[test]
fn full_light_time_pipeline_uses_explicit_tt_nutation_precession_rate() {
    // Synthetic P is a rotating Z axis, N is the identity matrix:
    // not an IAU precession or nutation accuracy reference.
    // This checks the actual DE441/light-time + rate-bridge
    // integration and does not pretend that TT = TDB.
    let mut reader = De441::new(Cursor::new(synthetic_de441())).unwrap();
    let omega_tt = 0.125;
    let ratio = 1.000_2;
    let precession = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let p_rate_tt = [[0.0, omega_tt, 0.0], [-omega_tt, 0.0, 0.0], [0.0; 3]];
    let frame = RotatingFrame::from_tt_precession_nutation(
        precession,
        p_rate_tt,
        precession,
        [[0.0; 3]; 3],
        TtPerTdbDay::new(ratio).unwrap(),
    )
    .unwrap();
    let epoch = JulianDateTdb(1_016.0);
    let result =
        geocentric_light_time_moving_spherical(&mut reader, Body::Mercury, epoch, &frame).unwrap();
    near(result.spherical.longitude_rad, 0.0, 1e-15);
    near(result.spherical.distance_km, 1_000.0, 1e-8);
    near(
        result.spherical.longitude_rate_rad_per_day,
        -omega_tt * ratio,
        1e-15,
    );
    near(result.spherical.latitude_rate_rad_per_day, 0.0, 1e-15);
    near(result.spherical.distance_rate_km_per_day, 0.0, 1e-10);
    assert_eq!(result.spherical.epoch, epoch);
    assert!(result.emission_epoch.0 < epoch.0);
}

#[test]
fn full_de441_to_moving_ecliptic_latitude_and_rate_uses_true_obliquity() {
    // Synthetic classic-DE441 reader; the source is stationary along
    // inertial +X relative to Earth. A +90° passive Z precession maps
    // it to -Y; the *positive* true-obliquity ecliptic tilt maps it
    // to +ecliptic latitude with rate +epsilon_dot*dTT/dTDB.
    // This is math, not an independently validated IAU epoch.
    let mut reader = De441::new(Cursor::new(synthetic_de441())).unwrap();
    let angle = FRAC_PI_2;
    let eps = 0.4_f64;
    let epsdot_per_tt_day = 0.012_f64;
    let ratio = 1.000_2_f64;
    let precession = passive_z(angle, 0.0);
    // RotatingFrame has no matrix getters: construct the same
    // independently known +90° passive Z rotation from sin/cos.
    let (s, c) = angle.sin_cos();
    let p = [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]];
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let frame = RotatingFrame::from_tt_true_ecliptic(
        p,
        [[0.0; 3]; 3],
        identity,
        [[0.0; 3]; 3],
        eps,
        epsdot_per_tt_day,
        TtPerTdbDay::new(ratio).unwrap(),
    )
    .unwrap();
    let epoch = JulianDateTdb(1_016.0);
    let result =
        geocentric_light_time_moving_spherical(&mut reader, Body::Mercury, epoch, &frame).unwrap();
    near(result.spherical.longitude_rad, 1.5 * PI, 1e-15);
    near(result.spherical.latitude_rad, eps, 1e-15);
    near(result.spherical.latitude_rate_rad_per_day, epsdot_per_tt_day * ratio, 1e-15);
    near(result.spherical.longitude_rate_rad_per_day, 0.0, 1e-15);
    near(result.spherical.distance_km, 1_000.0, 1e-8);
    near(result.spherical.distance_rate_km_per_day, 0.0, 1e-10);
    // The constant precession was only used to compute synthetic
    // axes; no source speed or second light-time correction exists.
    let dummy = precession.rotate_state(&StateVector {
        target: Body::Mercury,
        center: Body::Earth,
        epoch,
        position_km: [1_000.0, 0.0, 0.0],
        velocity_km_per_day: [0.0; 3],
    }).unwrap();
    near(dummy.position_km[1], -1_000.0, 1e-10);
}
