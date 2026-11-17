/* EXHIBIT A. Common Public Attribution License Version 1.0.
The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

//! Regression for classic-DE441 record selection at representable TDB JD
//! boundaries; synthetic Cartesian coefficients only, not JPL science data.

use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

const RECORD_BYTES: usize = 3000;

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// 2026-11-14. Two synthetic 32-day records, with intentionally distinct
// Mercury positions on either side of the exact TDB JD 0.0 boundary.
fn fixture(start: f64) -> Vec<u8> {
    let mut bytes = vec![0; RECORD_BYTES * 4];
    let name = b"JPL Planetary Ephemeris DE441";
    bytes[..name.len()].copy_from_slice(name);
    put_f64(&mut bytes, 2652, start);
    put_f64(&mut bytes, 2660, start + 64.0);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);

    // Mercury: one constant Chebyshev coefficient per ICRF Cartesian axis.
    put_i32(&mut bytes, 2696, 3);
    put_i32(&mut bytes, 2700, 1);
    put_i32(&mut bytes, 2704, 1);
    // Unused libration pointer pads the record to 3000 bytes.
    put_i32(&mut bytes, 2844, 373);
    put_i32(&mut bytes, 2848, 1);
    put_i32(&mut bytes, 2852, 1);

    for index in 0..2 {
        let offset = (index + 2) * RECORD_BYTES;
        let record_start = start + 32.0 * index as f64;
        put_f64(&mut bytes, offset, record_start);
        put_f64(&mut bytes, offset + 8, record_start + 32.0);
        for axis in 0..3 {
            put_f64(
                &mut bytes,
                offset + (2 + axis) * 8,
                (index * 100 + axis + 11) as f64,
            );
        }
    }
    bytes
}

fn sample(reader: &mut De441<Cursor<Vec<u8>>>, jd: f64) -> [f64; 3] {
    let state = reader
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(jd),
        )
        .unwrap();
    assert_eq!(state.velocity_km_per_day, [0.0; 3]);
    assert_eq!(state.epoch, JulianDateTdb(jd));
    state.position_km
}

#[test]
fn subnormal_negative_jd_remains_before_exact_zero_record_boundary() {
    let mut reader = De441::new(Cursor::new(fixture(-32.0))).unwrap();
    // With the old floor((jd - start) / step) expression, -MIN_SUBNORMAL
    // rounds up to 0 after adding 32 and selects the *wrong* record.
    let before = -f64::from_bits(1);
    let after = f64::from_bits(1);
    assert!(before < 0.0 && after > 0.0);

    assert_eq!(sample(&mut reader, before), [11.0, 12.0, 13.0]);
    assert_eq!(sample(&mut reader, 0.0), [111.0, 112.0, 113.0]);
    assert_eq!(sample(&mut reader, after), [111.0, 112.0, 113.0]);
    // Cache transition must work again when rewinding to the prior record.
    assert_eq!(sample(&mut reader, before), [11.0, 12.0, 13.0]);
}

#[test]
fn ordinary_binary64_neighbors_and_inclusive_coverage_end_remain_correct() {
    let start = 2_451_513.0;
    let boundary = start + 32.0;
    let mut reader = De441::new(Cursor::new(fixture(start))).unwrap();
    let prior = f64::from_bits(boundary.to_bits() - 1);
    let next = f64::from_bits(boundary.to_bits() + 1);

    assert_eq!(sample(&mut reader, start), [11.0, 12.0, 13.0]);
    assert_eq!(sample(&mut reader, prior), [11.0, 12.0, 13.0]);
    assert_eq!(sample(&mut reader, boundary), [111.0, 112.0, 113.0]);
    assert_eq!(sample(&mut reader, next), [111.0, 112.0, 113.0]);
    assert_eq!(sample(&mut reader, start + 64.0), [111.0, 112.0, 113.0]);
}

#[test]
fn negative_era_record_boundary_and_backward_lookup_keep_correct_record() {
    let start = -3_027_231.5;
    let boundary = start + 32.0;
    let mut reader = De441::new(Cursor::new(fixture(start))).unwrap();
    // Negative f64 encoding reverses the bit-order relative to the value:
    // adding one bit moves the value *down* by one ULP.
    let before = f64::from_bits(boundary.to_bits() + 1);
    let after = f64::from_bits(boundary.to_bits() - 1);

    assert!(before < boundary && boundary < after);
    assert_eq!(sample(&mut reader, before), [11.0, 12.0, 13.0]);
    assert_eq!(sample(&mut reader, boundary), [111.0, 112.0, 113.0]);
    assert_eq!(sample(&mut reader, after), [111.0, 112.0, 113.0]);
    assert_eq!(sample(&mut reader, before), [11.0, 12.0, 13.0]);
}
