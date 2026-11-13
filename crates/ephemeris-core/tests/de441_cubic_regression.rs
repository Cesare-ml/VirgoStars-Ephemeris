/* EXHIBIT A. Common Public Attribution License Version 1.0.
The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0.
The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party materials licensed separately.
The Original Developer and Initial Developer is Cesare-ml (VirgoStars project). Copyright (c) 2026 Cesare-ml. All Rights Reserved.
Contributors retain applicable rights in their own contributions under the License.
SPDX-License-Identifier: CPAL-1.0
*/

use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

const RECORD_BYTES: usize = 3000;

fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn cubic_fixture() -> Vec<u8> {
    let mut bytes = vec![0u8; RECORD_BYTES * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    write_f64(&mut bytes, 2652, 1000.0);
    write_f64(&mut bytes, 2660, 1032.0);
    write_f64(&mut bytes, 2668, 32.0);
    write_f64(&mut bytes, 2688, 81.0);
    write_i32(&mut bytes, 2840, 441);

    // Mercury: four Chebyshev coefficients per axis, two 16-day granules.
    write_i32(&mut bytes, 2696, 3);
    write_i32(&mut bytes, 2700, 4);
    write_i32(&mut bytes, 2704, 2);
    // Unused libration pointer fixes the synthetic record length at 3000 bytes.
    write_i32(&mut bytes, 2844, 373);
    write_i32(&mut bytes, 2848, 1);
    write_i32(&mut bytes, 2852, 1);

    write_f64(&mut bytes, RECORD_BYTES * 2, 1000.0);
    write_f64(&mut bytes, RECORD_BYTES * 2 + 8, 1032.0);

    for (granule, coefficients) in [[10.0, 2.0, 3.0, 4.0], [20.0, -1.0, 0.0, 2.0]]
        .into_iter()
        .enumerate()
    {
        let offset = RECORD_BYTES * 2 + (2 + granule * 12) * 8;
        for (index, coefficient) in coefficients.into_iter().enumerate() {
            write_f64(&mut bytes, offset + index * 8, coefficient);
        }
    }
    bytes
}

#[test]
fn cubic_chebyshev_position_and_analytic_velocity_match_independent_polynomials() {
    // T0=1, T1=x, T2=2x²-1, T3=4x³-3x.
    // dT1/dx=1, dT2/dx=4x, dT3/dx=12x²-3.
    // Each granule spans 16 TDB days, so dx/dJD=2/16.
    let mut ephemeris = De441::new(Cursor::new(cubic_fixture())).unwrap();
    for (jd, expected_position, expected_velocity) in [
        // Negative normalized time, plus the exact first-granule start.
        (1000.0, 7.0, 3.25),
        (1004.0, 11.5, -0.5),
        (1008.0, 7.0, -1.25),
        (1012.0, 5.5, 1.0),
        // Granule boundary must select the second polynomial, not the first.
        (1016.0, 19.0, 2.125),
        (1020.0, 22.5, -0.125),
        (1024.0, 20.0, -0.875),
        (1028.0, 17.5, -0.125),
        (1032.0, 21.0, 2.125),
    ] {
        let state = ephemeris
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd),
            )
            .unwrap();
        assert_eq!(state.position_km, [expected_position, 0.0, 0.0]);
        assert_eq!(
            state.velocity_km_per_day,
            [expected_velocity, 0.0, 0.0],
            "unexpected TDB-day velocity at JD {jd}"
        );
    }
}

// Modification (2026-11-13): exercise independent Y/Z coefficient offsets and
// reversed target/center signs with cubic velocity in TDB days.
#[test]
fn cubic_interpolation_keeps_axes_separate_and_reverses_relative_states() {
    let mut bytes = cubic_fixture();
    // First granule starts at word 3. Each axis has four coefficients.
    // Y = 5*T0 - 2*T1 + T2, Z = -3*T0 + T3.
    let first = RECORD_BYTES * 2 + 2 * 8;
    for (index, value) in [5.0, -2.0, 1.0, 0.0].into_iter().enumerate() {
        write_f64(&mut bytes, first + (4 + index) * 8, value);
    }
    for (index, value) in [-3.0, 0.0, 0.0, 1.0].into_iter().enumerate() {
        write_f64(&mut bytes, first + (8 + index) * 8, value);
    }

    let mut ephemeris = De441::new(Cursor::new(bytes)).unwrap();
    // At JD 1008, x=0, so T2=-1, T3=0, and dx/dJD=1/8.
    let forward = ephemeris
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1008.0),
        )
        .unwrap();
    assert_eq!(forward.position_km, [7.0, 4.0, -3.0]);
    assert_eq!(forward.velocity_km_per_day, [-1.25, -0.25, -0.375]);

    let reverse = ephemeris
        .state(
            Body::SolarSystemBarycenter,
            Body::Mercury,
            JulianDateTdb(1008.0),
        )
        .unwrap();
    assert_eq!(reverse.position_km, [-7.0, -4.0, 3.0]);
    assert_eq!(reverse.velocity_km_per_day, [1.25, 0.25, 0.375]);
}

#[test]
fn cubic_velocity_agrees_with_centered_position_difference_inside_each_granule() {
    // Independent numerical cross-check of the analytic derivative at non-boundary
    // epochs in both granules. The step is exactly representable in binary days.
    let mut ephemeris = De441::new(Cursor::new(cubic_fixture())).unwrap();
    let step_days = 1.0 / 1024.0;
    for jd in [1005.25, 1011.75, 1019.25, 1026.75] {
        let before = ephemeris
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd - step_days),
            )
            .unwrap();
        let at = ephemeris
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd),
            )
            .unwrap();
        let after = ephemeris
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd + step_days),
            )
            .unwrap();
        for axis in 0..3 {
            let numerical_velocity =
                (after.position_km[axis] - before.position_km[axis]) / (2.0 * step_days);
            // Centered differences of a cubic have O(h^2) truncation error.
            assert!(
                (at.velocity_km_per_day[axis] - numerical_velocity).abs() < 1.0e-6,
                "velocity/position derivative mismatch at JD {jd}, axis {axis}"
            );
        }
    }
}

// Modification (2026-11-13): check XYZ coefficient offsets and analytic
// TDB-day derivatives independently in the second Chebyshev granule.
#[test]
fn second_granule_xyz_coefficients_and_velocities_are_independent() {
    let mut bytes = cubic_fixture();
    // Granule 2 begins at word 15; Y and Z follow the four X coefficients.
    let second = RECORD_BYTES * 2 + (2 + 12) * 8;
    for (index, value) in [1.0, 2.0, 3.0, 4.0].into_iter().enumerate() {
        write_f64(&mut bytes, second + (4 + index) * 8, value);
    }
    for (index, value) in [4.0, -2.0, 0.0, 1.0].into_iter().enumerate() {
        write_f64(&mut bytes, second + (8 + index) * 8, value);
    }

    // At JD 1024, normalized x=0: T2=-1, T3=0,
    // dT1/dJD=1/8 and dT3/dJD=-3/8.
    let mut ephemeris = De441::new(Cursor::new(bytes)).unwrap();
    let state = ephemeris
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1024.0),
        )
        .unwrap();
    assert_eq!(state.position_km, [20.0, -2.0, 4.0]);
    assert_eq!(state.velocity_km_per_day, [-0.875, -1.25, -0.625]);
}

// Modification (2026-11-13): verify the inclusive final epoch and the
// exact granule transition for all three axes, including reverse geometry.
#[test]
fn second_granule_xyz_boundaries_preserve_velocity_and_reverse_signs() {
    let mut bytes = cubic_fixture();
    let second = RECORD_BYTES * 2 + (2 + 12) * 8;
    for (index, value) in [1.0, 2.0, 3.0, 4.0].into_iter().enumerate() {
        write_f64(&mut bytes, second + (4 + index) * 8, value);
    }
    for (index, value) in [4.0, -2.0, 0.0, 1.0].into_iter().enumerate() {
        write_f64(&mut bytes, second + (8 + index) * 8, value);
    }
    let mut ephemeris = De441::new(Cursor::new(bytes)).unwrap();
    // At x=-1 (JD 1016), T=[1,-1,1,-1], dT/dx=[0,1,-4,9].
    // At x=+1 (JD 1032), T=[1,1,1,1], dT/dx=[0,1,4,9].
    // Both derivative vectors are scaled by dx/dJD=1/8.
    for (jd, position, velocity) in [
        (1016.0, [19.0, -2.0, 5.0], [2.125, 3.25, 0.875]),
        (1032.0, [21.0, 10.0, 3.0], [2.125, 6.25, 0.875]),
    ] {
        let forward = ephemeris
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd),
            )
            .unwrap();
        assert_eq!(forward.position_km, position, "position at JD {jd}");
        assert_eq!(forward.velocity_km_per_day, velocity, "velocity at JD {jd}");
        let reverse = ephemeris
            .state(
                Body::SolarSystemBarycenter,
                Body::Mercury,
                JulianDateTdb(jd),
            )
            .unwrap();
        for axis in 0..3 {
            assert_eq!(reverse.position_km[axis], -position[axis]);
            assert_eq!(reverse.velocity_km_per_day[axis], -velocity[axis]);
        }
    }
}
