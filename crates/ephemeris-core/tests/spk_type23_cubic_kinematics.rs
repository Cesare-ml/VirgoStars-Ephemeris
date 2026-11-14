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

//! Independent cubic kinematics regression for experimental J2000 SPK types 2/3.
//!
//! Modification (2026-11-14): test physical km/s against a direct polynomial
//! derivative, not against another Chebyshev evaluator or a bundled JPL kernel.
//! This synthetic record is not independent NASA/JPL numerical validation.

use std::io::Cursor;

use virgostars_ephemeris_core::spk_direct_state::direct_spk_state;
use virgostars_ephemeris_core::spk_index::{inspect_spk, SpkByteOrder};

const RECORD_BYTES: usize = 1024;
const FIRST_WORD: usize = 513;
const POSITION: [[f64; 4]; 3] = [
    [100.0, 2.0, -1.0, 0.5],
    [-20.0, -3.0, 0.25, -0.125],
    [1.0, 0.5, -0.75, 0.0],
];

// Type-3 velocity is an *independent stored polynomial*, in km/TDB-second.
// For each position c0*T0 + c1*T1 + c2*T2 + c3*T3, its derivative divided
// by radius=5 has coefficients [(c1+3*c3)/5, 4*c2/5, 6*c3/5, 0].
const VELOCITY: [[f64; 4]; 3] = [
    [0.7, -0.8, 0.6, 0.0],
    [-0.675, 0.2, -0.15, 0.0],
    [0.1, -0.6, 0.0, 0.0],
];

fn put_i32(bytes: &mut [u8], offset: usize, value: i32, order: SpkByteOrder) {
    let data = match order {
        SpkByteOrder::LittleEndian => value.to_le_bytes(),
        SpkByteOrder::BigEndian => value.to_be_bytes(),
    };
    bytes[offset..offset + 4].copy_from_slice(&data);
}

fn put_f64(bytes: &mut [u8], offset: usize, value: f64, order: SpkByteOrder) {
    let data = match order {
        SpkByteOrder::LittleEndian => value.to_le_bytes(),
        SpkByteOrder::BigEndian => value.to_be_bytes(),
    };
    bytes[offset..offset + 8].copy_from_slice(&data);
}

fn synthetic_spk(order: SpkByteOrder, kind: i32) -> Vec<u8> {
    let components = if kind == 2 { 3 } else { 6 };
    let record_words = 2 + 4 * components;
    let final_word = FIRST_WORD + record_words + 3;
    let mut bytes = vec![0_u8; 5 * RECORD_BYTES];

    bytes[..8].copy_from_slice(b"DAF/SPK ");
    bytes[88..96].copy_from_slice(match order {
        SpkByteOrder::LittleEndian => b"LTL-IEEE",
        SpkByteOrder::BigEndian => b"BIG-IEEE",
    });
    put_i32(&mut bytes, 8, 2, order);
    put_i32(&mut bytes, 12, 6, order);
    put_i32(&mut bytes, 76, 3, order);
    put_i32(&mut bytes, 80, 3, order);
    put_i32(&mut bytes, 84, (final_word + 1) as i32, order);

    // One segment, 499 relative to 10, frame 1 (J2000), [-10, 0] TDB/ET s.
    let summary = 2 * RECORD_BYTES;
    put_f64(&mut bytes, summary + 16, 1.0, order);
    put_f64(&mut bytes, summary + 24, -10.0, order);
    put_f64(&mut bytes, summary + 32, 0.0, order);
    for (index, value) in [499, 10, 1, kind, FIRST_WORD as i32, final_word as i32]
        .into_iter()
        .enumerate()
    {
        put_i32(&mut bytes, summary + 40 + index * 4, value, order);
    }

    let record_base = (FIRST_WORD - 1) * 8;
    put_f64(&mut bytes, record_base, -5.0, order);
    put_f64(&mut bytes, record_base + 8, 5.0, order);
    for (axis, coefficients) in POSITION.iter().enumerate() {
        for (index, &coefficient) in coefficients.iter().enumerate() {
            put_f64(
                &mut bytes,
                record_base + (2 + axis * 4 + index) * 8,
                coefficient,
                order,
            );
        }
    }
    if kind == 3 {
        for (axis, coefficients) in VELOCITY.iter().enumerate() {
            for (index, &coefficient) in coefficients.iter().enumerate() {
                put_f64(
                    &mut bytes,
                    record_base + (2 + (3 + axis) * 4 + index) * 8,
                    coefficient,
                    order,
                );
            }
        }
    }
    let footer = (final_word - 4) * 8;
    for (index, value) in [-10.0, 10.0, record_words as f64, 1.0]
        .into_iter()
        .enumerate()
    {
        put_f64(&mut bytes, footer + 8 * index, value, order);
    }
    bytes
}

fn polynomial_oracle(coefficients: [f64; 4], x: f64) -> (f64, f64) {
    let x2 = x * x;
    let position = coefficients[0]
        + coefficients[1] * x
        + coefficients[2] * (2.0 * x2 - 1.0)
        + coefficients[3] * (4.0 * x2 * x - 3.0 * x);
    let velocity =
        (coefficients[1] + 4.0 * coefficients[2] * x + coefficients[3] * (12.0 * x2 - 3.0)) / 5.0;
    (position, velocity)
}

fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-12,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn type2_and_type3_cubic_states_match_independent_polynomial_oracle() {
    for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
        let mut type2_file = Cursor::new(synthetic_spk(order, 2));
        let mut type3_file = Cursor::new(synthetic_spk(order, 3));
        let type2_index = inspect_spk(&mut type2_file).unwrap();
        let type3_index = inspect_spk(&mut type3_file).unwrap();

        // Negative ET, both inclusive boundaries and interior points.
        for et in [-10.0, -9.0, -7.5, -5.0, -2.5, -1.0, 0.0] {
            let type2 = direct_spk_state(&mut type2_file, &type2_index, 499, et).unwrap();
            let type3 = direct_spk_state(&mut type3_file, &type3_index, 499, et).unwrap();
            assert_eq!(
                (type2.target_id, type2.center_id, type2.frame_id),
                (499, 10, 1)
            );
            assert_eq!(
                (type3.target_id, type3.center_id, type3.frame_id),
                (499, 10, 1)
            );
            assert_eq!(type2.tdb_seconds_past_j2000, et);
            assert_eq!(type3.tdb_seconds_past_j2000, et);
            for (axis, &coefficients) in POSITION.iter().enumerate() {
                let (expected_position, expected_velocity) =
                    polynomial_oracle(coefficients, (et + 5.0) / 5.0);
                near(type2.position_km[axis], expected_position);
                near(type3.position_km[axis], expected_position);
                near(type2.velocity_km_per_second[axis], expected_velocity);
                near(type3.velocity_km_per_second[axis], expected_velocity);
            }
        }
    }
}
