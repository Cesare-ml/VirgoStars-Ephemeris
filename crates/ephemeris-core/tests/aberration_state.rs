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

//! Independent mathematical and offline mock-file checks of annual
//! aberration **Cartesian velocities** at observation TDB.
//! No JPL binary or Swiss/SOFA/ERFA program source is bundled.

use std::io::Cursor;

use virgostars_ephemeris_core::aberration::{
    annual_aberration_icrf, AberrationError, SPEED_OF_LIGHT_KM_PER_DAY,
};
use virgostars_ephemeris_core::aberration_state::{
    aberrate_light_time_state, geocentric_light_time_annual_aberration,
    AberratedStateError, ObserverAccelerationSource,
};
use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};
use virgostars_ephemeris_core::light_time::{
    GeocentricLightTimeState, LightTimeError, SPEED_OF_LIGHT_KM_PER_SECOND,
};

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e} expected={expected:.16e}, tol={tolerance:.2e}"
    );
}

fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn state(r: [f64; 3], v: [f64; 3]) -> GeocentricLightTimeState {
    GeocentricLightTimeState {
        target: Body::Mercury,
        observation_epoch: JulianDateTdb(2_451_545.0),
        emission_epoch: JulianDateTdb(2_451_544.99),
        light_time_seconds: 864.0,
        light_time_rate: -0.000_01,
        position_km: r,
        velocity_km_per_day: v,
    }
}

#[test]
fn zero_observer_velocity_acceleration_preserves_light_time_state_and_rate() {
    let input = state([3.0e6, 4.0e6, 0.0], [-30.0, 4_000.0, 200.0]);
    let output = aberrate_light_time_state(&input, [0.0; 3], [0.0; 3]).unwrap();
    for i in 0..3 {
        near(output.position_km[i], input.position_km[i], 1e-9);
        near(output.velocity_km_per_tdb_day[i], input.velocity_km_per_day[i], 1e-10);
    }
    assert_eq!(output.target, Body::Mercury);
    assert_eq!(output.observation_epoch, input.observation_epoch);
    assert_eq!(output.emission_epoch, input.emission_epoch);
    assert_eq!(output.light_time_seconds, input.light_time_seconds);
    assert_eq!(output.light_time_rate, input.light_time_rate);
    assert_eq!(
        output.observer_acceleration_source,
        ObserverAccelerationSource::CallerProvided
    );
}

#[test]
fn constant_transverse_source_has_exact_relativistic_acceleration_rate() {
    // Independent 1D Lorentz identity when n=(0,1,0), beta along X:
    // n_ab=(beta,sqrt(1-beta²),0).
    // At fixed n, dn_ab/dt=(beta_dot,-beta*beta_dot/sqrt(1-beta²),0).
    let range = 5.0e7;
    let beta = 0.1_f64;
    let beta_dot = 0.000_7_f64; // signed per TDB day
    let input = state([0.0, range, 0.0], [0.0; 3]);
    let result = aberrate_light_time_state(
        &input,
        [beta * SPEED_OF_LIGHT_KM_PER_DAY, 0.0, 0.0],
        [beta_dot * SPEED_OF_LIGHT_KM_PER_DAY, 0.0, 0.0],
    )
    .unwrap();
    let transverse = (1.0 - beta * beta).sqrt();
    near(result.position_km[0], range * beta, 1e-8);
    near(result.position_km[1], range * transverse, 1e-8);
    near(result.velocity_km_per_tdb_day[0], range * beta_dot, 1e-6);
    near(
        result.velocity_km_per_tdb_day[1],
        -range * beta * beta_dot / transverse,
        1e-6,
    );
    near(result.velocity_km_per_tdb_day[2], 0.0, 1e-12);
}

#[test]
fn finite_difference_of_independently_aberrated_positions_matches_all_three_rates() {
    // Source moves along a curved path; observer has different components
    // of velocity and constant acceleration. Check against centered
    // *positions* obtained by the already-tested independent direction API.
    let r0 = 120_000_000.0;
    let omega = -0.014;
    let n0 = 0.7_f64;
    let h = 0.000_1;
    let earth_v = [2.3e6, -1.5e6, 4.0e5];
    let earth_acc = [-7.0e4, -8.0e4, 1.5e4];
    let src = |t: f64| {
        let phase = n0 + omega * t;
        [
            r0 * phase.cos(),
            r0 * phase.sin(),
            (0.13 * r0) + (0.008 * r0) * t,
        ]
    };
    let src_rate = [
        -r0 * omega * n0.sin(),
        r0 * omega * n0.cos(),
        0.008 * r0,
    ];
    let input = state(src(0.0), src_rate);
    let result = aberrate_light_time_state(&input, earth_v, earth_acc).unwrap();
    let position = |dt: f64| {
        let r = src(dt);
        let v = std::array::from_fn(|i| earth_v[i] + earth_acc[i] * dt);
        let direction = annual_aberration_icrf(r, v).unwrap();
        let radius = norm(r);
        direction.map(|coordinate| coordinate * radius)
    };
    let left = position(-h);
    let right = position(h);
    for i in 0..3 {
        let independent = (right[i] - left[i]) / (2.0 * h);
        near(result.velocity_km_per_tdb_day[i], independent, 0.05);
    }
}

#[test]
fn full_relative_motion_preserves_range_and_radial_velocity() {
    let r = [8.0e7, -6.0e7, 2.0e7];
    let v = [-80_000.0, 30_000.0, 20_000.0];
    let input = state(r, v);
    let output = aberrate_light_time_state(
        &input,
        [8.0e5, 2.5e6, -2.0e5],
        [1_000.0, -9_500.0, 4_200.0],
    )
    .unwrap();
    near(norm(output.position_km), norm(r), 1e-7);
    let radial_original = dot(r, v) / norm(r);
    let radial_aberrated = dot(output.position_km, output.velocity_km_per_tdb_day)
        / norm(output.position_km);
    near(radial_aberrated, radial_original, 1e-8);
    let expected_direction = annual_aberration_icrf(r, [8.0e5, 2.5e6, -2.0e5]).unwrap();
    for i in 0..3 {
        near(output.position_km[i] / norm(r), expected_direction[i], 1e-15);
    }
}

#[test]
fn no_physical_motion_or_acceleration_causes_no_spurious_rate() {
    let input = state([50_000_000.0, 12_000_000.0, -8_000_000.0], [0.0; 3]);
    let output = aberrate_light_time_state(&input, [1.2e6, -2.0e6, 0.0], [0.0; 3])
        .unwrap();
    for value in output.velocity_km_per_tdb_day {
        near(value, 0.0, 1e-10);
    }
}

#[test]
fn refuses_nonfinite_and_unphysical_earth_motion_without_nan_fallback() {
    let input = state([1.0e7, 0.0, 0.0], [0.0; 3]);
    assert!(matches!(
        aberrate_light_time_state(&input, [f64::NAN, 0.0, 0.0], [0.0; 3]),
        Err(AberratedStateError::Aberration(AberrationError::NonFiniteInput))
    ));
    assert!(matches!(
        aberrate_light_time_state(&input, [0.0; 3], [f64::INFINITY, 0.0, 0.0]),
        Err(AberratedStateError::NonFiniteAcceleration)
    ));
    assert!(matches!(
        aberrate_light_time_state(
            &input,
            [SPEED_OF_LIGHT_KM_PER_DAY, 0.0, 0.0],
            [0.0; 3],
        ),
        Err(AberratedStateError::Aberration(
            AberrationError::SuperluminalObserver
        ))
    ));
}

#[test]
fn rejects_invalid_reception_intermediate_or_undefined_direction() {
    let original = state([1.0e7, 0.0, 0.0], [0.0; 3]);
    let mut changed = original;
    changed.target = Body::Earth;
    assert!(matches!(
        aberrate_light_time_state(&changed, [0.0; 3], [0.0; 3]),
        Err(AberratedStateError::InvalidIntermediate)
    ));
    changed = original;
    changed.emission_epoch = JulianDateTdb(original.observation_epoch.0 + 1.0);
    assert!(matches!(
        aberrate_light_time_state(&changed, [0.0; 3], [0.0; 3]),
        Err(AberratedStateError::InvalidIntermediate)
    ));
    changed = original;
    changed.velocity_km_per_day[0] = f64::NAN;
    assert!(matches!(
        aberrate_light_time_state(&changed, [0.0; 3], [0.0; 3]),
        Err(AberratedStateError::InvalidIntermediate)
    ));
    changed = original;
    changed.position_km = [0.0; 3];
    assert!(matches!(
        aberrate_light_time_state(&changed, [0.0; 3], [0.0; 3]),
        Err(AberratedStateError::Aberration(
            AberrationError::UndefinedDirection
        ))
    ));
}

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn synthetic_classic_de441() -> Vec<u8> {
    // Two 32-day synthetic records, Mercury and Earth stationary,
    // with no external NASA kernel or compressed Swiss files.
    let size = 3_000_usize;
    let mut bytes = vec![0_u8; size * 4];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    put_f64(&mut bytes, 2652, 1_000.0);
    put_f64(&mut bytes, 2660, 1_064.0);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);
    for (index, first) in [(0, 3), (2, 6), (9, 9), (12, 373)] {
        let at = if index == 12 { 2844 } else { 2696 + index * 12 };
        put_i32(&mut bytes, at, first);
        put_i32(&mut bytes, at + 4, 1);
        put_i32(&mut bytes, at + 8, 1);
    }
    for record in 0..2 {
        let at = size * (record + 2);
        put_f64(&mut bytes, at, 1_000.0 + record as f64 * 32.0);
        put_f64(&mut bytes, at + 8, 1_032.0 + record as f64 * 32.0);
        put_f64(&mut bytes, at + 2 * 8, 2_000.0);
        put_f64(&mut bytes, at + 5 * 8, 1_000.0);
    }
    bytes
}

#[test]
fn one_call_through_de441_lighttime_and_aberration_preserves_synthetic_result() {
    let mut ephemeris = De441::new(Cursor::new(synthetic_classic_de441())).unwrap();
    let output = geocentric_light_time_annual_aberration(
        &mut ephemeris,
        Body::Mercury,
        JulianDateTdb(1_016.0),
        0.02,
    )
    .unwrap();
    near(output.position_km[0], 1_000.0, 1e-8);
    near(output.position_km[1], 0.0, 1e-12);
    near(output.position_km[2], 0.0, 1e-12);
    for v in output.velocity_km_per_tdb_day {
        near(v, 0.0, 1e-10);
    }
    near(
        output.light_time_seconds,
        1_000.0 / SPEED_OF_LIGHT_KM_PER_SECOND,
        1e-8,
    );
    assert_eq!(
        output.observer_acceleration_source,
        ObserverAccelerationSource::CenteredEphemerisVelocity {
            step_tdb_days: 0.02
        }
    );
}

#[test]
fn explicit_fd_step_and_external_data_bounds_fail_closed() {
    let mut ephemeris = De441::new(Cursor::new(synthetic_classic_de441())).unwrap();
    for invalid in [0.0, -0.01, 0.6, f64::NAN, 1e-12] {
        assert!(matches!(
            geocentric_light_time_annual_aberration(
                &mut ephemeris,
                Body::Mercury,
                JulianDateTdb(1_016.0),
                invalid
            ),
            Err(AberratedStateError::InvalidAccelerationStep)
        ));
    }
    assert!(matches!(
        geocentric_light_time_annual_aberration(
            &mut ephemeris,
            Body::Mercury,
            JulianDateTdb(1_000.0),
            0.01
        ),
        Err(AberratedStateError::Ephemeris(De441Error::OutsideCoverage {
            ..
        }))
    ));
    assert!(matches!(
        geocentric_light_time_annual_aberration(
            &mut ephemeris,
            Body::Earth,
            JulianDateTdb(1_016.0),
            0.01
        ),
        Err(AberratedStateError::LightTime(
            LightTimeError::UndefinedDirection
        ))
    ));
}
