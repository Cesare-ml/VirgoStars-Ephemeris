/*
EXHIBIT A. Common Public Attribution License Version 1.0.

The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

// New synthetic arithmetic regressions, 2026-11-14.
// These are not independent JPL/NAIF scientific reference vectors.
use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};

const RECORD_BYTES: usize = 3000;
const SLOPES: [f64; 3] = [1.0e308, -1.0e308, 5.0e307];

fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn linear_fixture(step: f64) -> Vec<u8> {
    // Two header records and one coefficient record. A harmless libration
    // pointer pads the declared record to 3000 bytes, like other test fixtures.
    let mut bytes = vec![0u8; RECORD_BYTES * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    write_f64(&mut bytes, 2652, 1000.0);
    write_f64(&mut bytes, 2660, 1000.0 + step);
    write_f64(&mut bytes, 2668, step);
    write_f64(&mut bytes, 2688, 81.0);
    write_i32(&mut bytes, 2840, 441);

    // Mercury's three Cartesian axes: independent T0 + slope*T1.
    write_i32(&mut bytes, 2696, 3);
    write_i32(&mut bytes, 2700, 2);
    write_i32(&mut bytes, 2704, 1);
    write_i32(&mut bytes, 2844, 373);
    write_i32(&mut bytes, 2848, 1);
    write_i32(&mut bytes, 2852, 1);

    let record = RECORD_BYTES * 2;
    write_f64(&mut bytes, record, 1000.0);
    write_f64(&mut bytes, record + 8, 1000.0 + step);
    for (axis, slope) in SLOPES.iter().enumerate() {
        let offset = record + (2 + axis * 2) * 8;
        write_f64(&mut bytes, offset, 0.0);
        write_f64(&mut bytes, offset + 8, *slope);
    }
    bytes
}

#[test]
fn finite_linear_derivative_has_finite_velocity_without_intermediate_overflow() {
    let mut eph = De441::new(Cursor::new(linear_fixture(32.0))).unwrap();
    for (jd, x) in [(1000.0, -1.0), (1016.0, 0.0), (1032.0, 1.0)] {
        let state = eph
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd),
            )
            .expect("the interpolated position and velocity are both finite");
        for (axis, slope) in SLOPES.iter().enumerate() {
            assert_eq!(state.position_km[axis], slope * x);
            let expected = (slope / 32.0) * 2.0;
            assert!(
                (state.velocity_km_per_day[axis] - expected).abs()
                    <= expected.abs() * 1.0e-14,
                "axis {axis} at JD {jd}: expected {expected}, got {}",
                state.velocity_km_per_day[axis]
            );
        }
    }
}

#[test]
fn truly_unrepresentable_velocity_still_fails_closed() {
    // With a one-day record, 2 * 1e308 km/day cannot be represented.
    let mut eph = De441::new(Cursor::new(linear_fixture(1.0))).unwrap();
    assert!(matches!(
        eph.state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1000.5)
        ),
        Err(De441Error::InvalidFile("non-finite interpolated state"))
    ));
}

#[test]
fn preserves_representable_subnormal_velocity_at_short_record_width() {
    // With width = 2 days, d(T1)/dJD = 1. A divide-then-multiply
    // implementation loses the smallest positive subnormal to zero.
    let mut bytes = linear_fixture(2.0);
    for axis in 0..3 {
        let offset = RECORD_BYTES * 2 + (2 + axis * 2) * 8 + 8;
        write_f64(&mut bytes, offset, f64::from_bits(1));
    }
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    let state = eph
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1001.0),
        )
        .unwrap();
    for velocity in state.velocity_km_per_day {
        assert_eq!(velocity, f64::from_bits(1));
    }
}
