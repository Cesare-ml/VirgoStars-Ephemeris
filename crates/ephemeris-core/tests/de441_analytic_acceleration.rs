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

//! Synthetic quadratic Chebyshev fixtures for exact second TDB
//! derivatives in the classic DE441 reader and yearly aberration rates.
//! No JPL binary or third-party reference code is distributed.

use std::io::Cursor;

use virgostars_ephemeris_core::aberration_state::{
    geocentric_light_time_annual_aberration,
    geocentric_light_time_annual_aberration_analytic, ObserverAccelerationSource,
};
use virgostars_ephemeris_core::de441::{
    Body, De441, De441Error, JulianDateTdb,
};

fn near(value: f64, expected: f64, tolerance: f64) {
    assert!(
        (value - expected).abs() <= tolerance,
        "actual={value:.16e}, expected={expected:.16e}, tol={tolerance:.3e}"
    );
}

fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn set_series(bytes: &mut [u8], record: usize, first_word: usize, values: [f64; 3]) {
    let record_offset = 3_000 * (2 + record);
    for (index, value) in values.iter().enumerate() {
        write_f64(
            bytes,
            record_offset + (first_word - 1 + index) * 8,
            *value,
        );
    }
}

/// A synthetic, **not official NASA/JPL**, little-endian classic DE441
/// with two 32-day records and quadratic T2 terms for Mercury,
/// EMB and the geocentric Moon; Earth is reconstructed from EMB/Moon.
fn quadratic_de441() -> Vec<u8> {
    let record_size = 3_000_usize;
    let mut bytes = vec![0_u8; record_size * 4];
    let name = b"JPL Planetary Ephemeris DE441";
    bytes[..name.len()].copy_from_slice(name);
    write_f64(&mut bytes, 2652, 1_000.0);
    write_f64(&mut bytes, 2660, 1_064.0);
    write_f64(&mut bytes, 2668, 32.0);
    write_f64(&mut bytes, 2688, 81.0);
    write_i32(&mut bytes, 2840, 441);
    // 3-dimensional degree-2 coefficient data start at words:
    // Mercury: 3..11, EMB: 12..20, Moon: 21..29.
    for (index, first, degree) in [(0, 3, 3), (2, 12, 3), (9, 21, 3), (12, 373, 1)] {
        let offset = if index == 12 { 2844 } else { 2696 + index * 12 };
        write_i32(&mut bytes, offset, first);
        write_i32(&mut bytes, offset + 4, degree);
        write_i32(&mut bytes, offset + 8, 1);
    }
    for record in 0..2 {
        let start = record_size * (2 + record);
        write_f64(&mut bytes, start, 1_000.0 + record as f64 * 32.0);
        write_f64(&mut bytes, start + 8, 1_032.0 + record as f64 * 32.0);

        // X coordinate c0*T0 + c1*T1 + c2*T2; Y and Z zero.
        // T2=2*x²-1; d²T2/dt²=4/(16 TDB days)².
        set_series(&mut bytes, record, 3, [2_000_000.0, 80.0, 16.0]);
        set_series(&mut bytes, record, 12, [1_000_000.0, 3_200.0, 512.0]);
        set_series(&mut bytes, record, 21, [8_200.0, 320.0, 256.0]);
        // Non-collinear Y motion makes the Earth's true acceleration
        // affect the aberration direction and its differentiated rate.
        set_series(&mut bytes, record, 6, [400_000.0, -200.0, 60.0]);
        set_series(&mut bytes, record, 15, [300_000.0, 1_000.0, 128.0]);
        set_series(&mut bytes, record, 24, [500.0, 64.0, 32.0]);
    }
    bytes
}

fn eph() -> De441<Cursor<Vec<u8>>> {
    De441::new(Cursor::new(quadratic_de441())).unwrap()
}

#[test]
fn quadratic_earth_and_mercury_have_known_exact_accelerations() {
    let mut reader = eph();
    let t = JulianDateTdb(1_016.0); // exactly x=0 in first record
    let earth = reader
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, t)
        .unwrap();
    let moon = reader
        .state_with_acceleration(Body::Moon, Body::Earth, t)
        .unwrap();
    let mercury = reader
        .state_with_acceleration(Body::Mercury, Body::SolarSystemBarycenter, t)
        .unwrap();
    near(mercury.position_km[0], 2_000_000.0 - 16.0, 1e-9);
    near(mercury.velocity_km_per_day[0], 80.0 / 16.0, 1e-12);
    near(mercury.acceleration_km_per_day2[0], 4.0 * 16.0 / 256.0, 1e-12);
    near(earth.position_km[0], 1_000_000.0 - 512.0 - (8_200.0 - 256.0) / 82.0, 1e-9);
    near(earth.velocity_km_per_day[0], 200.0 - 20.0 / 82.0, 1e-12);
    near(earth.acceleration_km_per_day2[0], 8.0 - 4.0 / 82.0, 1e-12);
    near(moon.acceleration_km_per_day2[0], 4.0, 1e-12);
    near(earth.acceleration_km_per_day2[1], 2.0 - 0.5 / 82.0, 1e-12);
    near(mercury.acceleration_km_per_day2[1], 4.0 * 60.0 / 256.0, 1e-12);
    near(earth.acceleration_km_per_day2[2], 0.0, 1e-12);
    near(mercury.acceleration_km_per_day2[2], 0.0, 1e-12);
    assert_eq!(earth.epoch, t);
    assert_eq!(earth.center, Body::SolarSystemBarycenter);
    assert_eq!(earth.target, Body::Earth);
}

#[test]
fn new_acceleration_api_preserves_original_position_and_speed_exactly() {
    let mut reader = eph();
    for day in [1_000.0, 1_011.25, 1_016.0, 1_031.5, 1_032.0, 1_048.0, 1_064.0] {
        for (target, center) in [
            (Body::Earth, Body::SolarSystemBarycenter),
            (Body::Moon, Body::Earth),
            (Body::Mercury, Body::Earth),
            (Body::EarthMoonBarycenter, Body::Moon),
        ] {
            let jd = JulianDateTdb(day);
            let old = reader.state(target, center, jd).unwrap();
            let new = reader.state_with_acceleration(target, center, jd).unwrap();
            assert_eq!(old.position_km, new.position_km);
            assert_eq!(old.velocity_km_per_day, new.velocity_km_per_day);
            assert_eq!(old.target, new.target);
            assert_eq!(old.center, new.center);
        }
    }
}

#[test]
fn acceleration_matches_independent_finite_differences_of_original_velocity() {
    let mut reader = eph();
    let h = 0.01;
    for time in [1_003.0, 1_016.0, 1_029.0, 1_038.0, 1_048.0, 1_060.0] {
        let target = Body::Earth;
        let center = Body::SolarSystemBarycenter;
        let left = reader.state(target, center, JulianDateTdb(time - h)).unwrap();
        let right = reader.state(target, center, JulianDateTdb(time + h)).unwrap();
        let analytic = reader
            .state_with_acceleration(target, center, JulianDateTdb(time))
            .unwrap();
        for i in 0..3 {
            near(
                analytic.acceleration_km_per_day2[i],
                (right.velocity_km_per_day[i] - left.velocity_km_per_day[i]) / (2.0 * h),
                1e-7,
            );
        }
    }
}

#[test]
fn center_reversal_and_earth_moon_mass_weights_apply_to_second_derivative() {
    let mut reader = eph();
    let t = JulianDateTdb(1_016.0);
    let earth_to_moon = reader.state_with_acceleration(Body::Moon, Body::Earth, t).unwrap();
    let moon_to_earth = reader.state_with_acceleration(Body::Earth, Body::Moon, t).unwrap();
    let earth_to_emb = reader
        .state_with_acceleration(Body::EarthMoonBarycenter, Body::Earth, t)
        .unwrap();
    let emb_to_moon = reader
        .state_with_acceleration(Body::Moon, Body::EarthMoonBarycenter, t)
        .unwrap();
    for i in 0..3 {
        near(
            earth_to_moon.acceleration_km_per_day2[i],
            -moon_to_earth.acceleration_km_per_day2[i],
            1e-12,
        );
        near(
            earth_to_emb.acceleration_km_per_day2[i],
            earth_to_moon.acceleration_km_per_day2[i] / 82.0,
            1e-12,
        );
        near(
            emb_to_moon.acceleration_km_per_day2[i],
            earth_to_moon.acceleration_km_per_day2[i] * 81.0 / 82.0,
            1e-12,
        );
    }
}

#[test]
fn identical_target_center_has_zero_second_derivative() {
    let mut reader = eph();
    let result = reader
        .state_with_acceleration(Body::Mercury, Body::Mercury, JulianDateTdb(1_016.0))
        .unwrap();
    assert_eq!(result.position_km, [0.0; 3]);
    assert_eq!(result.velocity_km_per_day, [0.0; 3]);
    assert_eq!(result.acceleration_km_per_day2, [0.0; 3]);
}

#[test]
fn analytic_aberration_composes_with_existing_lighttime_without_timestep() {
    let mut reader = eph();
    let time = JulianDateTdb(1_016.0);
    let analytic = geocentric_light_time_annual_aberration_analytic(
        &mut reader,
        Body::Mercury,
        time,
    )
    .unwrap();
    let approximated = geocentric_light_time_annual_aberration(
        &mut reader,
        Body::Mercury,
        time,
        0.02,
    )
    .unwrap();
    assert_eq!(
        analytic.observer_acceleration_source,
        ObserverAccelerationSource::AnalyticDe441Chebyshev
    );
    assert_eq!(
        approximated.observer_acceleration_source,
        ObserverAccelerationSource::CenteredEphemerisVelocity {
            step_tdb_days: 0.02
        }
    );
    for i in 0..3 {
        near(analytic.position_km[i], approximated.position_km[i], 1e-7);
        near(
            analytic.velocity_km_per_tdb_day[i],
            approximated.velocity_km_per_tdb_day[i],
            1e-5,
        );
    }
}

#[test]
fn analytic_acceleration_works_on_last_coverage_day_without_a_future_fd_sample() {
    let mut reader = eph();
    // At t=1064, the reader uses the last available record; no
    // t+h sample exists, but the polynomial endpoint derivative does.
    let t = JulianDateTdb(1_064.0);
    let direct = reader
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, t)
        .unwrap();
    near(direct.acceleration_km_per_day2[0], 8.0 - 4.0 / 82.0, 1e-12);
    assert!(geocentric_light_time_annual_aberration_analytic(
        &mut reader,
        Body::Mercury,
        t,
    )
    .is_ok());
    assert!(geocentric_light_time_annual_aberration(
        &mut reader,
        Body::Mercury,
        t,
        0.01,
    )
    .is_err());
}

#[test]
fn invalid_epoch_and_unavailable_series_remain_explicit() {
    let mut reader = eph();
    for t in [999.0, 1_065.0, f64::NAN] {
        assert!(matches!(
            reader.state_with_acceleration(
                Body::Earth,
                Body::SolarSystemBarycenter,
                JulianDateTdb(t),
            ),
            Err(De441Error::OutsideCoverage { .. })
        ));
    }
    assert!(matches!(
        reader.state_with_acceleration(
            Body::Sun,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1_016.0),
        ),
        Err(De441Error::MissingSeries(_))
    ));
}
