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

//! Integration-level kinematic invariants for mixed-type, mixed-endian SPK chains.
//!
//! Modification (2026-11-14): independently check the velocity of a geometric
//! target-minus-center chain using positions at neighboring TDB/ET epochs.
//! The polynomial fixture is invented here; it is not JPL/NAIF accuracy evidence.

use std::io::Cursor;

use virgostars_ephemeris_core::spk_index::SpkByteOrder;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

const RECORD_BYTES: usize = 1024;
const FIRST_DATA_WORD: usize = 513;
const MID_ET: f64 = 5.0;
const RADIUS_ET: f64 = 5.0;

#[derive(Clone, Copy)]
struct Link {
    target: i32,
    center: i32,
    kind: i32,
    position_at_mid_km: [f64; 3],
    velocity_km_s: [f64; 3],
}

fn put_i32(bytes: &mut [u8], offset: usize, value: i32, order: SpkByteOrder) {
    let word = match order {
        SpkByteOrder::LittleEndian => value.to_le_bytes(),
        SpkByteOrder::BigEndian => value.to_be_bytes(),
    };
    bytes[offset..offset + 4].copy_from_slice(&word);
}

fn put_f64(bytes: &mut [u8], offset: usize, value: f64, order: SpkByteOrder) {
    let word = match order {
        SpkByteOrder::LittleEndian => value.to_le_bytes(),
        SpkByteOrder::BigEndian => value.to_be_bytes(),
    };
    bytes[offset..offset + 8].copy_from_slice(&word);
}

fn synthetic_kernel(order: SpkByteOrder, links: &[Link]) -> Cursor<Vec<u8>> {
    assert!(!links.is_empty() && links.len() <= 6);
    let mut bytes = vec![0u8; 5 * RECORD_BYTES];
    bytes[..8].copy_from_slice(b"DAF/SPK ");
    bytes[88..96].copy_from_slice(match order {
        SpkByteOrder::LittleEndian => b"LTL-IEEE",
        SpkByteOrder::BigEndian => b"BIG-IEEE",
    });
    put_i32(&mut bytes, 8, 2, order);
    put_i32(&mut bytes, 12, 6, order);
    put_i32(&mut bytes, 76, 3, order);
    put_i32(&mut bytes, 80, 3, order);

    let mut next_word = FIRST_DATA_WORD;
    let summary = 2 * RECORD_BYTES;
    put_f64(&mut bytes, summary + 16, links.len() as f64, order);
    for (index, link) in links.iter().enumerate() {
        assert!(link.kind == 2 || link.kind == 3);
        let components = if link.kind == 2 { 3 } else { 6 };
        let record_words = 2 + 2 * components;
        let first = next_word;
        let last = first + record_words + 3;
        let descriptor = summary + (3 + 5 * index) * 8;
        put_f64(&mut bytes, descriptor, 0.0, order);
        put_f64(&mut bytes, descriptor + 8, 10.0, order);
        for (component, value) in [
            link.target,
            link.center,
            1, // J2000/ICRF frame
            link.kind,
            first as i32,
            last as i32,
        ]
        .into_iter()
        .enumerate()
        {
            put_i32(&mut bytes, descriptor + 16 + 4 * component, value, order);
        }

        let record = (first - 1) * 8;
        put_f64(&mut bytes, record, MID_ET, order);
        put_f64(&mut bytes, record + 8, RADIUS_ET, order);
        for axis in 0..3 {
            // Type 2: derivative of linear position series is velocity.
            // Type 3: velocity is an independently stored constant series.
            let offset = record + (2 + 2 * axis) * 8;
            put_f64(&mut bytes, offset, link.position_at_mid_km[axis], order);
            put_f64(
                &mut bytes,
                offset + 8,
                link.velocity_km_s[axis] * RADIUS_ET,
                order,
            );
            if link.kind == 3 {
                let velocity_offset = record + (2 + 2 * (3 + axis)) * 8;
                put_f64(&mut bytes, velocity_offset, link.velocity_km_s[axis], order);
                put_f64(&mut bytes, velocity_offset + 8, 0.0, order);
            }
        }
        let directory = (last - 4) * 8;
        for (n, value) in [0.0, 10.0, record_words as f64, 1.0]
            .into_iter()
            .enumerate()
        {
            put_f64(&mut bytes, directory + n * 8, value, order);
        }
        next_word = last + 1;
    }
    assert!(next_word <= 640);
    put_i32(&mut bytes, 84, next_word as i32, order);
    Cursor::new(bytes)
}

fn loaded_set() -> SpkKernelSet<Cursor<Vec<u8>>> {
    let mut set = SpkKernelSet::new();
    // Oldest file: Type 2 and 3 segments are independent of the second file.
    set.load(synthetic_kernel(
        SpkByteOrder::BigEndian,
        &[
            Link {
                target: 3,
                center: 0,
                kind: 3,
                position_at_mid_km: [100.0, 200.0, -12.0],
                velocity_km_s: [0.7, -0.2, 0.5],
            },
            Link {
                target: 399,
                center: 3,
                kind: 2,
                position_at_mid_km: [20.0, -30.0, 5.0],
                velocity_km_s: [0.6, 1.2, -0.25],
            },
            Link {
                target: 4,
                center: 0,
                kind: 2,
                position_at_mid_km: [-80.0, 22.0, 13.0],
                velocity_km_s: [-0.4, -0.7, 0.1],
            },
        ],
    ))
    .unwrap();
    set.load(synthetic_kernel(
        SpkByteOrder::LittleEndian,
        &[Link {
            target: 301,
            center: 399,
            kind: 3,
            position_at_mid_km: [5.0, 9.0, -3.0],
            velocity_km_s: [0.1, -0.3, 0.8],
        }],
    ))
    .unwrap();
    set
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected:.15}, got {actual:.15}, tolerance {tolerance}"
    );
}

#[test]
fn cross_kernel_chain_velocity_matches_independent_position_difference() {
    let mut set = loaded_set();
    assert_eq!(set.kernel_count(), 2);
    // Analytic oracle for 301 -> 399 -> 3 -> 0 minus 4 -> 0.
    let position_at_mid = [205.0, 157.0, -23.0];
    let velocity = [1.8, 1.4, 0.95];
    for et in [0.0, 2.25, 5.0, 7.75, 10.0] {
        let state = set.relative_state(301, 4, et).unwrap();
        assert_eq!(state.common_ancestor_id, 0);
        assert_eq!((state.target_id, state.center_id), (301, 4));
        assert_eq!(state.tdb_seconds_past_j2000, et);
        for axis in 0..3 {
            near(
                state.position_km[axis],
                position_at_mid[axis] + velocity[axis] * (et - MID_ET),
                1.0e-11,
            );
            near(state.velocity_km_per_second[axis], velocity[axis], 1.0e-12);
        }
    }

    // The velocity is tested against two *separate position evaluations*.
    // This catches time-unit or sign errors in a chain even if its velocity
    // happens to agree with another internal evaluator.
    let step = 0.125_f64; // TDB seconds, not days
    let before = set.relative_state(301, 4, MID_ET - step).unwrap();
    let after = set.relative_state(301, 4, MID_ET + step).unwrap();
    let center = set.relative_state(301, 4, MID_ET).unwrap();
    for axis in 0..3 {
        near(
            (after.position_km[axis] - before.position_km[axis]) / (2.0 * step),
            center.velocity_km_per_second[axis],
            1.0e-10,
        );
    }
}

#[test]
fn geometric_chain_is_antisymmetric_and_transitive_in_three_axes() {
    let mut set = loaded_set();
    for et in [0.0, 5.0, 10.0] {
        let whole = set.relative_state(301, 4, et).unwrap();
        let reverse = set.relative_state(4, 301, et).unwrap();
        let part1 = set.relative_state(301, 399, et).unwrap();
        let part2 = set.relative_state(399, 3, et).unwrap();
        let part3 = set.relative_state(3, 4, et).unwrap();
        let identity = set.relative_state(301, 301, et).unwrap();

        for axis in 0..3 {
            near(whole.position_km[axis], -reverse.position_km[axis], 1.0e-11);
            near(
                whole.velocity_km_per_second[axis],
                -reverse.velocity_km_per_second[axis],
                1.0e-12,
            );
            near(
                whole.position_km[axis],
                part1.position_km[axis] + part2.position_km[axis] + part3.position_km[axis],
                1.0e-11,
            );
            near(
                whole.velocity_km_per_second[axis],
                part1.velocity_km_per_second[axis]
                    + part2.velocity_km_per_second[axis]
                    + part3.velocity_km_per_second[axis],
                1.0e-12,
            );
        }
        assert_eq!(identity.position_km, [0.0; 3]);
        assert_eq!(identity.velocity_km_per_second, [0.0; 3]);
    }
}
