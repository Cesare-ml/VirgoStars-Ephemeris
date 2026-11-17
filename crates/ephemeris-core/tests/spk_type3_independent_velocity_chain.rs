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

//! Regression: SPK Type 3 velocities are stored data, not position derivatives.
//!
//! Modified 2026-11-14: verify that an independently supplied, deliberately
//! inconsistent Type-3 velocity polynomial survives direct and multi-kernel
//! relative-state composition. Type 2 must still derive its velocities.
//! Fully synthetic DAF/SPK data; NOT JPL/NAIF numerical validation.

use std::io::Cursor;

use virgostars_ephemeris_core::spk_index::SpkByteOrder;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

const RECORD_BYTES: usize = 1024;
const FIRST_WORD: usize = 513;
const MID_ET: f64 = 5.0;
const RADIUS_ET: f64 = 5.0;

const EMB_POSITION: [f64; 3] = [100.0, -40.0, 8.0];
const EMB_SPEED: [f64; 3] = [1.5, -0.25, 0.125];

const MOON_POSITION: [f64; 3] = [5.0, 10.0, -3.0];
const MOON_VELOCITY_AT_MID: [f64; 3] = [2.25, -3.5, 0.75];
const MOON_VELOCITY_SLOPE_IN_X: [f64; 3] = [0.5, 1.0, -0.25];

fn put_i32(bytes: &mut [u8], offset: usize, value: i32, endian: SpkByteOrder) {
    let word = match endian {
        SpkByteOrder::LittleEndian => value.to_le_bytes(),
        SpkByteOrder::BigEndian => value.to_be_bytes(),
    };
    bytes[offset..offset + 4].copy_from_slice(&word);
}

fn put_f64(bytes: &mut [u8], offset: usize, value: f64, endian: SpkByteOrder) {
    let word = match endian {
        SpkByteOrder::LittleEndian => value.to_le_bytes(),
        SpkByteOrder::BigEndian => value.to_be_bytes(),
    };
    bytes[offset..offset + 8].copy_from_slice(&word);
}

/// One synthetic J2000 Type 2 or Type 3 segment, ET in [0,10] seconds.
/// For Type 3, the POSITION series is constant and the VELOCITY series is
/// linear: the latter is intentionally NOT the derivative of the former.
fn synthetic_kernel(endian: SpkByteOrder, kind: i32) -> Cursor<Vec<u8>> {
    assert!(kind == 2 || kind == 3);
    let components = if kind == 2 { 3 } else { 6 };
    let record_words = 2 + 2 * components;
    let final_word = FIRST_WORD + record_words + 3;
    let mut bytes = vec![0_u8; 5 * RECORD_BYTES];
    bytes[..8].copy_from_slice(b"DAF/SPK ");
    bytes[88..96].copy_from_slice(match endian {
        SpkByteOrder::LittleEndian => b"LTL-IEEE",
        SpkByteOrder::BigEndian => b"BIG-IEEE",
    });
    put_i32(&mut bytes, 8, 2, endian);
    put_i32(&mut bytes, 12, 6, endian);
    put_i32(&mut bytes, 76, 3, endian);
    put_i32(&mut bytes, 80, 3, endian);
    put_i32(&mut bytes, 84, (final_word + 1) as i32, endian);

    let summary = 2 * RECORD_BYTES;
    put_f64(&mut bytes, summary + 16, 1.0, endian);
    put_f64(&mut bytes, summary + 24, 0.0, endian);
    put_f64(&mut bytes, summary + 32, 10.0, endian);
    let (target, center) = if kind == 2 { (399, 0) } else { (301, 399) };
    for (index, value) in [
        target,
        center,
        1, // NAIF J2000/ICRF fixed frame
        kind,
        FIRST_WORD as i32,
        final_word as i32,
    ]
    .into_iter()
    .enumerate()
    {
        put_i32(&mut bytes, summary + 40 + index * 4, value, endian);
    }

    let record = (FIRST_WORD - 1) * 8;
    put_f64(&mut bytes, record, MID_ET, endian);
    put_f64(&mut bytes, record + 8, RADIUS_ET, endian);
    for axis in 0..3 {
        let (position, position_slope) = if kind == 2 {
            (EMB_POSITION[axis], EMB_SPEED[axis] * RADIUS_ET)
        } else {
            (MOON_POSITION[axis], 0.0)
        };
        put_f64(&mut bytes, record + (2 + axis * 2) * 8, position, endian);
        put_f64(
            &mut bytes,
            record + (3 + axis * 2) * 8,
            position_slope,
            endian,
        );
        if kind == 3 {
            // Unlike Type 2, Type 3 must never derive this from position.
            put_f64(
                &mut bytes,
                record + (2 + (axis + 3) * 2) * 8,
                MOON_VELOCITY_AT_MID[axis],
                endian,
            );
            put_f64(
                &mut bytes,
                record + (3 + (axis + 3) * 2) * 8,
                MOON_VELOCITY_SLOPE_IN_X[axis],
                endian,
            );
        }
    }
    let directory = (final_word - 4) * 8;
    for (index, value) in [0.0, 10.0, record_words as f64, 1.0]
        .into_iter()
        .enumerate()
    {
        put_f64(&mut bytes, directory + index * 8, value, endian);
    }
    Cursor::new(bytes)
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-12,
        "expected {expected}, observed {actual}"
    );
}

#[test]
fn mixed_type_center_chain_preserves_independent_type3_velocity_series() {
    for (old_endian, new_endian) in [
        (SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian),
        (SpkByteOrder::BigEndian, SpkByteOrder::LittleEndian),
    ] {
        let mut kernels = SpkKernelSet::new();
        kernels.load(synthetic_kernel(old_endian, 2)).unwrap();
        kernels.load(synthetic_kernel(new_endian, 3)).unwrap();

        for et in [0.0, 2.5, 5.0, 7.5, 10.0] {
            let x = (et - MID_ET) / RADIUS_ET;
            let type2 = kernels.direct_state(399, et).unwrap();
            let type3 = kernels.direct_state(301, et).unwrap();
            let direct_link = kernels.relative_state(301, 399, et).unwrap();
            let combined = kernels.relative_state(301, 0, et).unwrap();
            let reverse = kernels.relative_state(0, 301, et).unwrap();

            assert_eq!(type2.center_id, 0);
            assert_eq!(type3.center_id, 399);
            assert_eq!(type2.tdb_seconds_past_j2000, et);
            assert_eq!(type3.tdb_seconds_past_j2000, et);
            assert_eq!(combined.common_ancestor_id, 0);
            assert_eq!(direct_link.common_ancestor_id, 399);
            assert_eq!(combined.tdb_seconds_past_j2000, et);

            for axis in 0..3 {
                let base_position = EMB_POSITION[axis] + EMB_SPEED[axis] * (et - MID_ET);
                let stored_type3_velocity =
                    MOON_VELOCITY_AT_MID[axis] + MOON_VELOCITY_SLOPE_IN_X[axis] * x;
                let combined_position = base_position + MOON_POSITION[axis];
                let combined_velocity = EMB_SPEED[axis] + stored_type3_velocity;

                close(type2.position_km[axis], base_position);
                close(type2.velocity_km_per_second[axis], EMB_SPEED[axis]);
                close(type3.position_km[axis], MOON_POSITION[axis]);
                close(type3.velocity_km_per_second[axis], stored_type3_velocity);
                close(
                    direct_link.velocity_km_per_second[axis],
                    stored_type3_velocity,
                );
                close(combined.position_km[axis], combined_position);
                close(combined.velocity_km_per_second[axis], combined_velocity);
                close(reverse.position_km[axis], -combined_position);
                close(reverse.velocity_km_per_second[axis], -combined_velocity);
            }
        }
    }
}

#[test]
fn stored_type3_velocity_is_not_forced_to_match_position_differences() {
    let mut kernels = SpkKernelSet::new();
    kernels
        .load(synthetic_kernel(SpkByteOrder::LittleEndian, 2))
        .unwrap();
    kernels
        .load(synthetic_kernel(SpkByteOrder::BigEndian, 3))
        .unwrap();

    let before = kernels.relative_state(301, 0, 4.0).unwrap();
    let after = kernels.relative_state(301, 0, 6.0).unwrap();
    let at_mid = kernels.relative_state(301, 0, MID_ET).unwrap();
    for axis in 0..3 {
        let numerical_position_slope = (after.position_km[axis] - before.position_km[axis]) / 2.0;
        close(numerical_position_slope, EMB_SPEED[axis]);
        close(
            at_mid.velocity_km_per_second[axis],
            EMB_SPEED[axis] + MOON_VELOCITY_AT_MID[axis],
        );
        assert!(
            (numerical_position_slope - at_mid.velocity_km_per_second[axis]).abs() > 0.5,
            "synthetic Type-3 velocity should be independent of position derivative"
        );
    }
}
