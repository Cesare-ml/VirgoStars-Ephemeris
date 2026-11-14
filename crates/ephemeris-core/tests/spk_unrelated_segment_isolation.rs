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

//! Synthetic regression: unrelated corrupt SPK records must not contaminate
//! geometric queries for valid targets in an ordered, mixed-endian kernel set.
//!
//! Added 2026-11-14. These invented DAF/SPK bytes are NOT NASA/JPL fixtures
//! and do not establish accuracy against independent NAIF/SPICE vectors.

use std::io::Cursor;

use virgostars_ephemeris_core::spk_index::SpkByteOrder;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;
use virgostars_ephemeris_core::spk_relative_state::SpkRelativeError;

const RECORD_BYTES: usize = 1024;
const FIRST_WORD: usize = 513;
const LAST_WORD: usize = 524;

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

/// Single Type-2 segment covering inclusive ET [0, 10], with linear XYZ
/// position and analytically constant velocity in km/s. A poisoned x
/// coefficient is intentionally nonfinite, while all DAF metadata is valid.
fn kernel(
    order: SpkByteOrder,
    target: i32,
    center: i32,
    position_at_midpoint: [f64; 2],
    velocity: [f64; 2],
    poison_x: bool,
) -> Cursor<Vec<u8>> {
    let mut bytes = vec![0; 5 * RECORD_BYTES];
    bytes[..8].copy_from_slice(b"DAF/SPK ");
    bytes[88..96].copy_from_slice(match order {
        SpkByteOrder::LittleEndian => b"LTL-IEEE",
        SpkByteOrder::BigEndian => b"BIG-IEEE",
    });
    for (offset, value) in [(8, 2), (12, 6), (76, 3), (80, 3), (84, 525)] {
        put_i32(&mut bytes, offset, value, order);
    }

    let summary = 2 * RECORD_BYTES;
    put_f64(&mut bytes, summary + 16, 1.0, order);
    put_f64(&mut bytes, summary + 24, 0.0, order);
    put_f64(&mut bytes, summary + 32, 10.0, order);
    for (index, value) in [target, center, 1, 2, FIRST_WORD as i32, LAST_WORD as i32]
        .into_iter()
        .enumerate()
    {
        put_i32(&mut bytes, summary + 40 + index * 4, value, order);
    }

    let record = (FIRST_WORD - 1) * 8;
    let coeffs = [
        if poison_x {
            f64::NAN
        } else {
            position_at_midpoint[0]
        },
        velocity[0] * 5.0,
        position_at_midpoint[1],
        velocity[1] * 5.0,
        0.0,
        0.0,
    ];
    for (index, value) in [5.0, 5.0].into_iter().chain(coeffs).enumerate() {
        put_f64(&mut bytes, record + index * 8, value, order);
    }

    // Directory: INIT, INTLEN, RSIZE and record count.
    for (index, value) in [0.0, 10.0, 8.0, 1.0].into_iter().enumerate() {
        put_f64(&mut bytes, (LAST_WORD - 4 + index) * 8, value, order);
    }
    Cursor::new(bytes)
}

fn valid_set() -> SpkKernelSet<Cursor<Vec<u8>>> {
    let mut set = SpkKernelSet::new();
    set.load(kernel(
        SpkByteOrder::LittleEndian,
        399,
        0,
        [100.0, 5.0],
        [1.0, -0.5],
        false,
    ))
    .unwrap();
    set.load(kernel(
        SpkByteOrder::BigEndian,
        301,
        399,
        [4.0, -3.0],
        [-0.5, 0.25],
        false,
    ))
    .unwrap();
    set
}

fn assert_vector(actual: [f64; 3], expected: [f64; 3]) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() <= 1e-12,
            "axis {axis}: actual {} vs expected {}",
            actual[axis],
            expected[axis],
        );
    }
}

#[test]
fn unrelated_invalid_coefficients_do_not_poison_valid_chains_or_seek_state() {
    let mut set = valid_set();
    set.load(kernel(
        SpkByteOrder::LittleEndian,
        499,
        0,
        [7.0, 8.0],
        [0.0, 0.0],
        true,
    ))
    .unwrap();
    assert_eq!(set.kernel_count(), 3);

    for et in [0.0, 3.0, 5.0, 7.0, 10.0] {
        // Exercise the bad record first so its read/seek/error path cannot
        // accidentally alter subsequent unrelated kernel evaluations.
        assert!(matches!(
            set.relative_state(499, 0, et),
            Err(SpkRelativeError::Segment(_))
        ));

        // Independent analytic oracle for two linear target-minus-center
        // records: (301-0) = (301-399) + (399-0).
        let dt = et - 5.0;
        let expected_pos = [104.0 + 0.5 * dt, 2.0 - 0.25 * dt, 0.0];
        let expected_vel = [0.5, -0.25, 0.0];
        let moon_ssb = set.relative_state(301, 0, et).unwrap();
        assert_eq!(moon_ssb.common_ancestor_id, 0);
        assert_vector(moon_ssb.position_km, expected_pos);
        assert_vector(moon_ssb.velocity_km_per_second, expected_vel);

        // Reverse direction and direct Earth-relative Moon must remain
        // coherent without evaluating the unrelated broken target.
        let reverse = set.relative_state(0, 301, et).unwrap();
        assert_vector(reverse.position_km, expected_pos.map(|value| -value));
        assert_vector(
            reverse.velocity_km_per_second,
            expected_vel.map(|value| -value),
        );
        let moon_earth = set.relative_state(301, 399, et).unwrap();
        assert_eq!(moon_earth.common_ancestor_id, 399);
        assert_vector(
            moon_earth.position_km,
            [4.0 - 0.5 * dt, -3.0 + 0.25 * dt, 0.0],
        );
        assert_vector(moon_earth.velocity_km_per_second, [-0.5, 0.25, 0.0]);
    }
}

#[test]
fn damaged_high_priority_target_is_not_masked_by_valid_older_source() {
    let mut set = valid_set();
    set.load(kernel(
        SpkByteOrder::LittleEndian,
        301,
        399,
        [900.0, 900.0],
        [5.0, 5.0],
        true,
    ))
    .unwrap();

    // The *selected* later source is invalid. Silent fallback would return a
    // plausible but incorrect Moon position and violate SPK load precedence.
    assert!(matches!(
        set.relative_state(301, 0, 5.0),
        Err(SpkRelativeError::Segment(_))
    ));
    let earth = set.relative_state(399, 0, 5.0).unwrap();
    assert_vector(earth.position_km, [100.0, 5.0, 0.0]);

    set.unload_last().unwrap();
    let recovered = set.relative_state(301, 0, 5.0).unwrap();
    assert_vector(recovered.position_km, [104.0, 2.0, 0.0]);
    assert_vector(recovered.velocity_km_per_second, [0.5, -0.25, 0.0]);
}
